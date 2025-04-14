use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::securiy::config::BotSecurityConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub bot: BotConfig,
    pub database: DatabaseConfig,
    pub security: BotSecurityConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BotConfig {
    pub target_name: String,
    pub notification_chat_id: i64,
    pub ping_user: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub connection_uri: String,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        // Open the file in read-only mode
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read the JSON contents into the Config struct
        let config = serde_json::from_reader(reader)?;
        Ok(config)
    }

    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        match Self::load(path) {
            Ok(config) => {
                log::info!("Configuration loaded successfully");
                config
            }
            Err(e) => {
                log::warn!("Failed to load configuration: {}. Using default values", e);
                Self::default()
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bot: BotConfig {
                target_name: "Не указан".to_string(),
                notification_chat_id: 0,
                ping_user: "@Test".to_string(),
            },
            database: DatabaseConfig {
                connection_uri: "mongodb://10.10.10.10:27017/".to_string(),
            },
            security: BotSecurityConfig::default(),
        }
    }
}
