use database_actions::DatabaseService;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use teloxide::prelude::*;

pub mod config;
pub mod console;
pub mod database_actions;
pub mod handlers;
pub mod securiy;

use config::Config;
use securiy::manager::SecurityManager;

#[tokio::main]
async fn main() {
    use flexi_logger::{Logger, Duplicate, FileSpec};
    use chrono::Local;
    let now = Local::now();
    let logfile_name = format!("latebot-{}", now.format("%Y-%m-%d_%H-%M-%S"));
    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory(".").basename(&logfile_name).suppress_timestamp())
        .duplicate_to_stdout(Duplicate::Info)
        .start()
        .unwrap();
    log::info!("Starting late tracking bot...");

    // Load configuration from config.json
    let config = Config::load_or_default("config.json");
    
    let target_name = config.bot.target_name;
    let notification_chat_id = config.bot.notification_chat_id;
    let ping_user = config.bot.ping_user;
    
    // Start console interface
    console::start_console_interface().await;

    // Initialize security manager
    let security_config = config.security;

    log::info!(
        "Initializing security manager with rate limit: {} requests per {} seconds",
        security_config.request_limit,
        security_config.time_window_seconds
    );
    let security_manager = Arc::new(SecurityManager::new(security_config).await);

    let database_service =
        database_actions::DatabaseServiceInner::new(&config.database.connection_uri).await;
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
                if let Some(user) = &msg.from {
                    let user_id = user.id.0 as i64;
                    log::info!("Text request from user: {}", user_id);

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

                log::info!("Callback request from user: {}", user_id);

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
