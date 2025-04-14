use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BotSecurityConfig {
    /// Maximum number of requests allowed within the time window
    pub request_limit: u32,
    /// Time window in seconds for rate limiting
    pub time_window_seconds: u32,
    /// Whether DDoS protection is enabled
    pub ddos_protection_enabled: bool,
    /// List of user IDs that are whitelisted (exempt from rate limiting)
    pub whitelist: Vec<i64>,
}

impl Default for BotSecurityConfig {
    fn default() -> Self {
        Self {
            request_limit: 30, // Default limit of 30 requests
            time_window_seconds: 60, // Default time window of 60 seconds (1 minute)
            ddos_protection_enabled: true,
            whitelist: Vec::new(), // Empty whitelist by default
        }
    }
}
