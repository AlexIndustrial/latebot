use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::time::sleep;

use super::config::BotSecurityConfig;

/// Structure to track user request rates
struct UserRequestInfo {
    /// Timestamps of recent requests
    request_timestamps: Vec<Instant>,
    /// Last time the request count was reset
    last_reset_time: Instant,
}

pub struct SecurityManager {
    /// Security configuration
    config: BotSecurityConfig,
    /// Map of user IDs to their request information, protected by a mutex for thread safety
    request_map: Mutex<HashMap<i64, UserRequestInfo>>,
}

pub enum CheckResult {
    /// Request is allowed to proceed
    Pass,
    /// Request is blocked due to rate limiting
    Block(Duration),
}

impl SecurityManager {
    pub async fn new(config: BotSecurityConfig) -> Self {
        Self { 
            config,
            request_map: Mutex::new(HashMap::new()),
        }
    }

    /// Checks if a request from a user should be allowed or blocked based on rate limits
    /// 
    /// # Arguments
    /// * `user_id` - The ID of the user making the request
    /// 
    /// # Returns
    /// * `CheckResult::Pass` if the request is allowed
    /// * `CheckResult::Block(Duration)` if the request is blocked, with the duration to wait
    pub async fn check_request_rate(&self, user_id: i64) -> CheckResult {
        // If DDoS protection is disabled, always allow the request
        if !self.config.ddos_protection_enabled {
            return CheckResult::Pass;
        }

        let now = Instant::now();
        let mut request_map = self.request_map.lock().await;
        
        // Get or create user request info
        let user_info = request_map.entry(user_id).or_insert_with(|| UserRequestInfo {
            request_timestamps: Vec::new(),
            last_reset_time: now,
        });
        
        // Clean up old timestamps (older than 1 minute)
        let one_minute_ago = now - Duration::from_secs(60);
        
        // If it's been more than a minute since the last reset, reset the timestamps
        if user_info.last_reset_time <= one_minute_ago {
            user_info.request_timestamps.clear();
            user_info.last_reset_time = now;
        } else {
            // Otherwise, just remove timestamps older than a minute
            user_info.request_timestamps.retain(|&timestamp| timestamp > one_minute_ago);
        }
        
        // Check if the user has exceeded the rate limit
        if user_info.request_timestamps.len() >= self.config.requests_per_minute_limit as usize {
            // If rate limit exceeded, calculate how long to wait
            if let Some(oldest_timestamp) = user_info.request_timestamps.first() {
                let time_to_wait = Duration::from_secs(60) - (now - *oldest_timestamp);
                return CheckResult::Block(time_to_wait);
            }
            // Fallback in case the vector is empty (shouldn't happen)
            return CheckResult::Block(Duration::from_secs(60));
        }
        
        // Record this request
        user_info.request_timestamps.push(now);
        
        CheckResult::Pass
    }

    /// Handles a request from a user, potentially blocking if rate limit is exceeded
    /// 
    /// # Arguments
    /// * `user_id` - The ID of the user making the request
    /// 
    /// # Returns
    /// * `true` if the request was allowed
    /// * `false` if the request was blocked
    pub async fn handle_request(&self, user_id: i64) -> bool {
        match self.check_request_rate(user_id).await {
            CheckResult::Pass => true,
            CheckResult::Block(_) => false,
        }
    }
    
    /// Handles a request from a user, waiting if necessary before proceeding
    /// 
    /// # Arguments
    /// * `user_id` - The ID of the user making the request
    /// 
    /// # Returns
    /// * `true` if the request was allowed immediately
    /// * `false` if the request had to wait before being allowed
    pub async fn handle_request_with_wait(&self, user_id: i64) -> bool {
        match self.check_request_rate(user_id).await {
            CheckResult::Pass => true,
            CheckResult::Block(wait_time) => {
                // Wait for the specified time before proceeding
                sleep(wait_time).await;
                true
            }
        }
    }
}