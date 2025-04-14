use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct BotSecurityConfig {
    /// Maximum number of requests allowed per minute per user
    pub requests_per_minute_limit: u32,
    /// Whether DDoS protection is enabled
    pub ddos_protection_enabled: bool,
}

impl Default for BotSecurityConfig {
    fn default() -> Self {
        Self {
            requests_per_minute_limit: 30, // Default limit of 30 requests per minute
            ddos_protection_enabled: true,
        }
    }
}
