#!/usr/bin/env python3
"""
Test script for Success Library feedback loop.
Tests that the bot correctly checks for blacklisted tokens.
"""

import psycopg2
import sys

def test_blacklist_check():
    """Test the blacklist query matches our Rust implementation"""
    try:
        conn = psycopg2.connect(
            dbname="mev_bot_success_library",
            user="lycanbeats",
            host="localhost",
            port=5432
        )
        cur = conn.cursor()
        
        # Test 1: Check blacklisted token (should return TRUE)
        blacklisted_token = "BpUmo4qv5NhDGjGgL2LfUCE4Xp1yXsF9NqShqkFb8qiA"
        cur.execute("""
            SELECT EXISTS(
                SELECT 1 FROM success_stories 
                WHERE token_address = %s 
                AND is_false_positive = TRUE
            )
        """, (blacklisted_token,))
        
        result = cur.fetchone()[0]
        print(f"✅ Test 1 - Blacklisted token check: {result} (expected: True)")
        assert result == True, "Blacklisted token should return True"
        
        # Test 2: Check non-blacklisted token (should return FALSE)
        good_token = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"
        cur.execute("""
            SELECT EXISTS(
                SELECT 1 FROM success_stories 
                WHERE token_address = %s 
                AND is_false_positive = TRUE
            )
        """, (good_token,))
        
        result = cur.fetchone()[0]
        print(f"✅ Test 2 - Non-blacklisted token check: {result} (expected: False)")
        assert result == False, "Non-blacklisted token should return False"
        
        # Test 3: Check non-existent token (should return FALSE)
        unknown_token = "UnknownTokenAddress123456789"
        cur.execute("""
            SELECT EXISTS(
                SELECT 1 FROM success_stories 
                WHERE token_address = %s 
                AND is_false_positive = TRUE
            )
        """, (unknown_token,))
        
        result = cur.fetchone()[0]
        print(f"✅ Test 3 - Unknown token check: {result} (expected: False)")
        assert result == False, "Unknown token should return False"
        
        # Summary
        print("\n🎉 All blacklist tests passed!")
        print(f"Database connection: postgresql://lycanbeats@localhost:5432/mev_bot_success_library")
        
        cur.close()
        conn.close()
        return 0
        
    except Exception as e:
        print(f"❌ Test failed: {e}")
        return 1

if __name__ == "__main__":
    sys.exit(test_blacklist_check())
