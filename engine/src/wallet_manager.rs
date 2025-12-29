use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_associated_token_account::get_associated_token_address;
use solana_client::nonblocking::rpc_client::RpcClient;
use anyhow::Result;

pub struct WalletManager {
    rpc: RpcClient,
}

impl WalletManager {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url.to_string()),
        }
    }

    /// Ensure an ATA exists for the given mint. 
    /// Returns Some(Instruction) if creation is needed, None otherwise.
    pub async fn ensure_ata_exists(&self, payer: &Pubkey, token_mint: &Pubkey) -> Option<Instruction> {
        let ata = get_associated_token_address(payer, token_mint);
        
        match self.rpc.get_account(&ata).await {
            Ok(_) => None, // Account exists
            Err(_) => {
                println!("📦 Creating ATA for mint: {}", token_mint);
                Some(create_associated_token_account(
                    payer,
                    payer,
                    token_mint,
                    &spl_token::id(),
                ))
            }
        }
    }

    /// Prepares WSOL by wrapping native SOL if balance is low.
    pub async fn sync_wsol(&self, payer: &Keypair, amount_lamports: u64) -> Result<Vec<Instruction>> {
        let wsol_mint = spl_token::native_mint::id();
        let ata = get_associated_token_address(&payer.pubkey(), &wsol_mint);
        
        let mut instructions = Vec::new();

        // 1. Ensure ATA exists
        if let Some(ix) = self.ensure_ata_exists(&payer.pubkey(), &wsol_mint).await {
            instructions.push(ix);
        }

        // 2. Transfer SOL to WSOL ATA
        instructions.push(system_instruction::transfer(
            &payer.pubkey(),
            &ata,
            amount_lamports,
        ));

        // 3. Sync Native
        instructions.push(spl_token::instruction::sync_native(
            &spl_token::id(),
            &ata,
        )?);

        Ok(instructions)
    }

    #[allow(dead_code)]
    /// Unwraps WSOL back to native SOL
    pub fn unwrap_wsol(&self, payer: &Pubkey) -> Result<Instruction> {
        let wsol_mint = spl_token::native_mint::id();
        let ata = get_associated_token_address(payer, &wsol_mint);

        // Close account instruction sends remaining SOL to destination (payer)
        Ok(spl_token::instruction::close_account(
            &spl_token::id(),
            &ata,
            payer,
            payer,
            &[],
        )?)
    }

    /// Batch get token balances using get_multiple_accounts
    pub async fn get_multiple_token_balances(&self, owner: &Pubkey, mints: &[Pubkey]) -> Result<HashMap<Pubkey, u64>> {
        use spl_associated_token_account::get_associated_token_address;
        use solana_sdk::program_pack::Pack;
        use std::collections::HashMap;

        let atas: Vec<Pubkey> = mints.iter().map(|m| get_associated_token_address(owner, m)).collect();
        let mut results = HashMap::new();

        // RPC get_multiple_accounts limit is typically 100
        for chunk in atas.chunks(100) {
            let accounts = self.rpc.get_multiple_accounts(chunk).await?;
            for (i, account_opt) in accounts.into_iter().enumerate() {
                let mint = mints[results.len()];
                let balance = if let Some(account) = account_opt {
                    if let Ok(token_account) = spl_token::state::Account::unpack(&account.data) {
                        token_account.amount
                    } else {
                        0
                    }
                } else {
                    0
                };
                results.insert(mint, balance);
            }
        }

        Ok(results)
    }

    /// Check which ATAs exist for a list of mints
    pub async fn check_atas_exist(&self, owner: &Pubkey, mints: &[Pubkey]) -> Result<Vec<(Pubkey, bool)>> {
        let atas: Vec<Pubkey> = mints.iter().map(|m| get_associated_token_address(owner, m)).collect();
        let accounts = self.rpc.get_multiple_accounts(&atas).await?;
        
        let mut results = Vec::new();
        for (i, acc) in accounts.into_iter().enumerate() {
            results.push((mints[i], acc.is_some()));
        }
        Ok(results)
    }

    /// Get native SOL balance
    pub async fn get_sol_balance(&self, address: &Pubkey) -> Result<u64> {
        Ok(self.rpc.get_balance(address).await?)
    }

    /// Get token balance for a given mint
    pub async fn get_token_balance(&self, owner: &Pubkey, mint: &Pubkey) -> Result<u64> {
        let ata = get_associated_token_address(owner, mint);
        match self.rpc.get_token_account_balance(&ata).await {
            Ok(balance) => Ok(balance.amount.parse::<u64>().unwrap_or(0)),
            Err(_) => Ok(0), // Account likely doesn't exist
        }
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwrap_wsol_instruction() {
        let payer = Pubkey::new_unique();
        let wallet_mgr = WalletManager::new("http://localhost:8899");
        
        let ix_result = wallet_mgr.unwrap_wsol(&payer);
        assert!(ix_result.is_ok());
        
        let ix = ix_result.unwrap();
        // Check program ID (Token Program)
        assert_eq!(ix.program_id, spl_token::id());
        
        // Check number of accounts (CloseAccount needs: account, destination, owner)
        // spl-token close_account typically has 3 accounts + signer M of N (if multisig)
        // but here it's 3 accounts (ata, destination, owner)
        assert!(ix.accounts.len() >= 3);
        
        // Destination should be the payer
        assert_eq!(ix.accounts[1].pubkey, payer);
        assert_eq!(ix.accounts[2].pubkey, payer);
    }
}
