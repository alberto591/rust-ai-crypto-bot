use solana_sdk::signature::{Keypair, Signer, write_keypair_file};
use std::env;
use std::path::Path;

fn main() {
    let home = env::var("HOME").expect("HOME not set");
    let path_str = format!("{}/.config/solana/id.json", home);
    let path = Path::new(&path_str);
    
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create config dir");
    }

    let keypair = Keypair::new();
    write_keypair_file(&keypair, path_str.clone()).expect("Failed to write keypair");
    
    println!("✅ Generated new keypair at: {}", path_str);
    println!("🔑 Pubkey: {}", keypair.pubkey());
    println!("\n⚠️  Since Solana CLI is missing, you cannot airdrop via CLI.");
    println!("   Please send devnet SOL to this address manually to run the full test.");
}
