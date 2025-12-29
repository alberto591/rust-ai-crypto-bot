use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use borsh::{BorshSerialize, to_vec};
use std::str::FromStr;

pub const PUMP_FUN_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const GLOBAL: &str = "4wTVyMKBvC6P4p58Prv7SzaLxe89nJ641PzN4331vX78";
pub const FEE_RECIPIENT: &str = "CebNoMenuPSToyuS9232FNRXF288Ukyy5oBv3e2xR9sN";
pub const EVENT_AUTHORITY: &str = "Ce6TQqeHC9p8KBAZvYtSAsvS6J2fXmzkm1Yw2XJ1f3R7";

#[derive(BorshSerialize)]
struct BuyArgs {
    amount: u64,
    max_sol_cost: u64,
}

#[derive(BorshSerialize)]
struct SellArgs {
    amount: u64,
    min_sol_output: u64,
}

pub fn buy(
    payer: Pubkey,
    mint: Pubkey,
    bonding_curve: Pubkey,
    associated_bonding_curve: Pubkey,
    associated_user_token: Pubkey,
    amount: u64,
    max_sol_cost: u64,
) -> Instruction {
    let program_id = Pubkey::from_str(PUMP_FUN_PROGRAM).unwrap();
    let global = Pubkey::from_str(GLOBAL).unwrap();
    let fee_recipient = Pubkey::from_str(FEE_RECIPIENT).unwrap();
    let event_authority = Pubkey::from_str(EVENT_AUTHORITY).unwrap();
    
    // Anchor Discriminator for 'buy'
    let mut data = vec![102, 6, 61, 18, 1, 218, 235, 234];
    let args = BuyArgs { amount, max_sol_cost };
    data.extend(to_vec(&args).unwrap());

    let accounts = vec![
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(fee_recipient, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(associated_bonding_curve, false),
        AccountMeta::new(associated_user_token, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn sell(
    payer: Pubkey,
    mint: Pubkey,
    bonding_curve: Pubkey,
    associated_bonding_curve: Pubkey,
    associated_user_token: Pubkey,
    amount: u64,
    min_sol_output: u64,
) -> Instruction {
    let program_id = Pubkey::from_str(PUMP_FUN_PROGRAM).unwrap();
    let global = Pubkey::from_str(GLOBAL).unwrap();
    let fee_recipient = Pubkey::from_str(FEE_RECIPIENT).unwrap();
    let event_authority = Pubkey::from_str(EVENT_AUTHORITY).unwrap();

    // Anchor Discriminator for 'sell'
    let mut data = vec![51, 230, 133, 164, 1, 127, 131, 210];
    let args = SellArgs { amount, min_sol_output };
    data.extend(to_vec(&args).unwrap());

    let accounts = vec![
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(fee_recipient, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(associated_bonding_curve, false),
        AccountMeta::new(associated_user_token, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    Instruction {
        program_id,
        accounts,
        data,
    }
}
