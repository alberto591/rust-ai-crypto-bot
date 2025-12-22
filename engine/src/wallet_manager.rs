use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_associated_token_account::get_associated_token_address;
use solana_client::rpc_client::RpcClient;
use std::error::Error;

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
    pub fn ensure_ata_exists(&self, payer: &Pubkey, token_mint: &Pubkey) -> Option<Instruction> {
        let ata = get_associated_token_address(payer, token_mint);
        
        match self.rpc.get_account(&ata) {
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
    pub fn sync_wsol(&self, payer: &Keypair, amount_lamports: u64) -> Result<Vec<Instruction>, Box<dyn Error>> {
        let wsol_mint = spl_token::native_mint::id();
        let ata = get_associated_token_address(&payer.pubkey(), &wsol_mint);
        
        let mut instructions = Vec::new();

        // 1. Ensure ATA exists
        if let Some(ix) = self.ensure_ata_exists(&payer.pubkey(), &wsol_mint) {
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

    /// Unwraps WSOL back to native SOL
    pub fn unwrap_wsol(&self, payer: &Pubkey) -> Result<Instruction, Box<dyn Error>> {
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
}

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
