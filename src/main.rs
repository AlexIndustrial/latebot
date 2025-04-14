use database_actions::DatabaseService;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use std::env;
use teloxide::prelude::*;

pub mod database_actions;
pub mod handlers;

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

    let database_service =
        database_actions::DatabaseServiceInner::new("mongodb://10.10.10.10:27017/").await;
    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(
            |
            bot: Bot,
            msg: Message,
            target_name: String,
            ping_user: String,
            database_service: DatabaseService,
            notification_chat_id: i64| {
                handlers::message_handler(
                    bot,
                    msg,
                    target_name,
                    ping_user,
                    database_service,
                    notification_chat_id,
                )
            },
        ))
        .branch(Update::filter_callback_query().endpoint(
            |bot: Bot, q: CallbackQuery, database_service: DatabaseService| {
                handlers::handle_callback(bot, q, database_service)
            },
        ));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            target_name,
            ping_user,
            database_service,
            notification_chat_id
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
