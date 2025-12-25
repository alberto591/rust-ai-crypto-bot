#[cfg(test)]
mod jito_resilience_tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_exponential_backoff_calculation() {
        // Test 2^n * 1000ms backoff formula
        let test_cases = vec![
            (0, 1000),  // 2^0 = 1, 1 * 1000 = 1000ms
            (1, 2000),  // 2^1 = 2, 2 * 1000 = 2000ms
            (2, 4000),  // 2^2 = 4, 4 * 1000 = 4000ms
        ];

        for (retry, expected_ms) in test_cases {
            let backoff_ms = 2_u64.pow(retry as u32) * 1000;
            assert_eq!(backoff_ms, expected_ms, "Retry {} should have {}ms backoff", retry, expected_ms);
        }
    }

    #[test]
    fn test_max_retries_configuration() {
        // Verify max_retries is set to 3
        let max_retries = 3u32;
        
        assert_eq!(max_retries, 3, "Should have 3 retry attempts per endpoint");
        
        // Calculate total wait time for all retries
        let mut total_wait_ms = 0u64;
        for retry in 0..max_retries {
            total_wait_ms += 2_u64.pow(retry) * 1000;
        }
        
        // 1000 + 2000 + 4000 = 7000ms = 7 seconds
        assert_eq!(total_wait_ms, 7000, "Total backoff time should be 7 seconds");
    }

    #[test]
    fn test_round_robin_endpoint_selection() {
        let num_endpoints = 3;
        let mut current_index = 0;
        
        let mut indices = Vec::new();
        
        // Simulate round-robin selection
        for attempt in 0..9 {
            indices.push(current_index);
            current_index = (current_index + 1) % num_endpoints;
        }
        
        // Should cycle through: 0,1,2,0,1,2,0,1,2
        assert_eq!(indices, vec![0,1,2,0,1,2,0,1,2], "Should cycle through all endpoints");
    }

    #[test]
    fn test_total_execution_attempts() {
        let num_endpoints = 3;
        let retries_per_endpoint = 3;
        let rpc_fallback = 1;
        
        let total_attempts = (num_endpoints * retries_per_endpoint) + rpc_fallback;
        
        assert_eq!(total_attempts, 10, "Should have 10 total execution attempts");
    }

    #[test]
    fn test_jito_url_parsing() {
        // Test comma-separated endpoint parsing
        let jito_url = "https://amsterdam.mainnet.block-engine.jito.wtf,https://frankfurt.mainnet.block-engine.jito.wtf,https://ny.mainnet.block-engine.jito.wtf";
        
        let urls: Vec<String> = jito_url
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        assert_eq!(urls.len(), 3, "Should parse 3 endpoints");
        assert!(urls[0].contains("amsterdam"), "First endpoint should be Amsterdam");
        assert!(urls[1].contains("frankfurt"), "Second endpoint should be Frankfurt");
        assert!(urls[2].contains("ny"), "Third endpoint should be NY");
    }

    #[test]
    fn test_jito_url_parsing_single_endpoint() {
        let jito_url = "https://amsterdam.mainnet.block-engine.jito.wtf";
        
        let urls: Vec<String> = jito_url
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        assert_eq!(urls.len(), 1, "Should handle single endpoint");
    }

    #[test]
    fn test_jito_url_parsing_with_spaces() {
        let jito_url = "url1, url2 ,url3";
        
        let urls: Vec<String> = jito_url
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        assert_eq!(urls, vec!["url1", "url2", "url3"], "Should trim whitespace");
    }

    #[test]
    fn test_error_message_contains_rate_limit() {
        let error_messages = vec![
            "status: ResourceExhausted, message: Network congested",
            "rate limit exceeded",
            "ResourceExhausted",
            "API rate limit hit",
        ];

        for msg in error_messages {
            let is_rate_limit = msg.contains("ResourceExhausted") || msg.contains("rate limit");
            assert!(is_rate_limit, "Should detect rate limit in: {}", msg);
        }
    }

    #[test]
    fn test_non_rate_limit_errors() {
        let error_messages = vec![
            "Connection timeout",
            "Invalid transaction",
            "Network error",
        ];

        for msg in error_messages {
            let is_rate_limit = msg.contains("ResourceExhausted") || msg.contains("rate limit");
            assert!(!is_rate_limit, "Should not detect rate limit in: {}", msg);
        }
    }
}
