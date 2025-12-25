import os
import re

def check_day1_implementation():
    config_path = "engine/src/config.rs"
    main_path = "engine/src/main.rs"
    
    print("🔍 Starting Day 1 Implementation Verification...")
    
    # 1. Check MONITORED_POOLS in config.rs
    if os.path.exists(config_path):
        with open(config_path, "r") as f:
            content = f.read()
            triangles = [
                "Triangle 1: SOL → JUP → USDC → SOL",
                "Triangle 2: SOL → RAY → USDC → SOL",
                "Triangle 3: SOL → BONK → USDC → SOL",
                "Triangle 4: SOL → WIF → USDC → SOL"
            ]
            missing = []
            for t in triangles:
                if t not in content:
                    missing.append(t)
            
            if not missing:
                print("✅ All 4 triangular paths found in MONITORED_POOLS.")
            else:
                print(f"❌ Missing triangular paths in config.rs: {missing}")
    else:
        print(f"❌ {config_path} not found.")

    # 2. Check ATA validation logic in main.rs
    if os.path.exists(main_path):
        with open(main_path, "r") as f:
            content = f.read()
            if "Validating Wallet state for monitored tokens" in content and "unique_mints.insert" in content:
                print("✅ Found comprehensive ATA validation logic in main.rs.")
            else:
                print("❌ Comprehensive ATA validation logic missing from main.rs.")
                
            if "config::MONITORED_POOLS" in content and "pools_to_watch.insert" in content:
                print("✅ Listener initialization correctly uses MONITORED_POOLS.")
            else:
                print("❌ Listener initialization not using MONITORED_POOLS correctly.")
    # 3. Check Balance/Inventory logic in main.rs
    if os.path.exists(main_path):
        with open(main_path, "r") as f:
            content = f.read()
            if "get_sol_balance" in content and "LOW SOL BALANCE" in content:
                print("✅ Found SOL Gas Safety check logic in main.rs.")
            else:
                print("❌ SOL Gas Safety check logic missing from main.rs.")
                
            if "STARTUP TOKEN INVENTORY" in content and "get_token_balance" in content:
                print("✅ Found Token Inventory Report logic in main.rs.")
            else:
                print("❌ Token Inventory Report logic missing from main.rs.")

    print("\n🏁 Verification Complete.")

if __name__ == "__main__":
    check_day1_implementation()
