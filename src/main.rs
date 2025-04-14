use database_actions::DatabaseService;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use teloxide::prelude::*;

pub mod database_actions;
pub mod handlers;
pub mod securiy;

use securiy::{config::BotSecurityConfig, manager::SecurityManager};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting late tracking bot...");

    let target_name = env::var("LATE_TARGET_NAME").unwrap_or_else(|_| "Не указан".to_string());
    let notification_chat_id = env::var("NOTIFICATION_CHAT_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<i64>()
        .unwrap_or(0);
    let ping_user = env::var("PING_USER").unwrap_or_else(|_| "@Test".to_string());

    // Initialize security manager with default config or load from environment
    let security_config = BotSecurityConfig {
        requests_per_minute_limit: 1,// Default to 30 requests per minute
        ddos_protection_enabled: true
    };

    log::info!(
        "Initializing security manager with rate limit: {} requests per minute",
        security_config.requests_per_minute_limit
    );
    let security_manager = Arc::new(SecurityManager::new(security_config).await);

    let database_service =
        database_actions::DatabaseServiceInner::new("mongodb://10.10.10.10:27017/").await;
    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(
            |bot: Bot,
             msg: Message,
             target_name: String,
             ping_user: String,
             database_service: DatabaseService,
             notification_chat_id: i64,
             security_manager: Arc<SecurityManager>| async move {
                // Get user ID for rate limiting
                if let Some(user) = msg.from() {
                    let user_id = user.id.0 as i64;

                    // Check if the request is allowed by the rate limiter
                    if !security_manager.handle_request(user_id).await {
                        // If rate limit exceeded, inform the user and don't process the request
                        let _ = bot
                            .send_message(
                                msg.chat.id,
                                "⚠️ Слишком много запросов. Пожалуйста, попробуйте позже.",
                            )
                            .await;
                        return Ok(());
                    }
                }

                // Proceed with normal request handling
                handlers::message_handler(
                    bot,
                    msg,
                    target_name,
                    ping_user,
                    database_service,
                    notification_chat_id,
                )
                .await
            },
        ))
        .branch(Update::filter_callback_query().endpoint(
            |bot: Bot,
             q: CallbackQuery,
             database_service: DatabaseService,
             security_manager: Arc<SecurityManager>| async move {
                // Get user ID for rate limiting
                let user = q.from.id;
                let user_id = user.0 as i64;

                // Check if the request is allowed by the rate limiter
                if !security_manager.handle_request(user_id).await {
                    // If rate limit exceeded, answer the callback query with an error message
                    let id = q.id.as_str();
                        let _ = bot
                            .answer_callback_query(id)
                            .text("⚠️ Слишком много запросов. Пожалуйста, попробуйте позже.")
                            .show_alert(true)
                            .await;
                    
                    return Ok(());
                }

                // Proceed with normal callback handling
                handlers::handle_callback(bot, q, database_service).await
            },
        ));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            target_name,
            ping_user,
            database_service,
            notification_chat_id,
            security_manager
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(Debug, Serialize, Deserialize)]
struct Vote {
    user_id: i64,
    username: String,
    is_late: bool,
    timestamp: DateTime,
}
