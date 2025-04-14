use std::sync::Arc;

use super::{config::BotSecurityConfig, manager::SecurityManager};



pub async fn init(config: BotSecurityConfig) -> Arc<SecurityManager> {


    let manager = SecurityManager::new(config).await;

    Arc::new(manager)
}