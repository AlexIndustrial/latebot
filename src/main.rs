use mongodb::{
    bson::{doc, DateTime},
    options::ClientOptions,
    Client,
};
use serde::{Deserialize, Serialize};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::RequestError;
use std::env;
use database_actions::DatabaseService;

pub mod database_actions;

#[derive(Debug, Serialize, Deserialize)]
struct Vote {
    user_id: i64,
    username: String,
    is_late: bool,
    timestamp: DateTime,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting late tracking bot...");

    let target_name = env::var("LATE_TARGET_NAME").unwrap_or_else(|_| "Поверинов".to_string());
    let notification_chat_id = env::var("NOTIFICATION_CHAT_ID").unwrap_or_else(|_| "0".to_string()).parse::<i64>().unwrap_or(0);
    let ping_user = env::var("PING_USER").unwrap_or_else(|_| "@Test".to_string());

    let database_service = database_actions::DatabaseServiceInner::new("mongodb://10.10.10.10:27017/").await;
    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(handle_callback));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![target_name, ping_user, database_service, notification_chat_id])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    target_name: String,
    ping_user: String,
    database_service: DatabaseService,
    notification_chat_id: i64,
) -> Result<(), RequestError> {
    match msg.text() {
        Some("/start") => {
            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![
                    InlineKeyboardButton::callback("✅ Опоздал", "late"),
                    InlineKeyboardButton::callback("❌ Не опоздал", "unlate")
                ],
                vec![
                    InlineKeyboardButton::callback("📊 Статистика", "stats")
                ]
            ]);

            bot.send_message(
                msg.chat.id,
                format!("👋 Добро пожаловать в бот учета опозданий!\n\n\
                🕒 Здесь вы можете голосовать, опоздал ли сегодня {}.\n\n\
                Команды:\n\
                /late - голосовать за опоздание\n\
                /unlate - голосовать против опоздания\n\
                /stats - посмотреть статистику\n\
                /get_chat_id - получить ID текущего чата\n\n\
                ⚠️ Голосовать можно только один раз в день!", target_name)
            )
            .reply_markup(keyboard)
            .await?;
        }
        Some("/late") | Some("/unlate") => {
            let user_id = msg.from().unwrap().id;
            let username = msg.from().unwrap().username.clone().unwrap_or_else(|| "anonymous".to_string());
            let is_late = msg.text() == Some("/late");

            let _ = database_service.check_today_document().await;

            match database_service.vote(user_id.0 as i64, is_late).await {
                Ok(_) => {
                    let vote_type = if is_late { "за опоздание" } else { "против опоздания" };
                    bot.send_message(
                        msg.chat.id,
                        format!("✅ Ваш голос {} успешно зарегистрирован!", vote_type),
                    ).await?;

                    if is_late {
                        if let Ok(total_late_days) = database_service.check_today_document().await {
                            if total_late_days.votes_yes.len() % 5 == 0 && notification_chat_id != 0 {
                                bot.send_message(
                                    ChatId(notification_chat_id),
                                    format!("🎉 {} Человек сообщили, что {}({}) опоздал! 🎉🎉🎉🎉🎉 Давайте его поздравим! 🎉🎉🎉🎉🎉 ", total_late_days.votes_yes.len(),  target_name,ping_user,)
                                ).await?;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Ошибка при голосовании: {}", e);
                    bot.send_message(
                        msg.chat.id,
                        "❌ Произошла ошибка при регистрации голоса. Пожалуйста, попробуйте позже."
                    ).await?;
                }
            }
        }
        Some("/stats") => {
            if let Ok(today_document) = database_service.check_today_document().await {
                let user_id = msg.from().unwrap().id.0 as i64;
                let user_vote = if today_document.votes_yes.contains(&user_id) {
                    "✅ Вы сегодня голосовали ЗА опоздание"
                } else if today_document.votes_no.contains(&user_id) {
                    "❌ Вы сегодня голосовали ПРОТИВ опоздания"
                } else {
                    "⚠️ Вы сегодня еще не голосовали"
                };

                let stats_message = format!(
                    "📊 Статистика за сегодня:\n\n\
                    За опоздание: {} голосов\n\
                    Против опоздания: {} голосов\n\n\
                    Всего проголосовало: {} человек\n\n\
                    {}",
                    today_document.votes_yes.len(),
                    today_document.votes_no.len(),
                    today_document.votes_yes.len() + today_document.votes_no.len(),
                    user_vote
                );
                
                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![
                        InlineKeyboardButton::callback("✅ Опоздал", "late"),
                        InlineKeyboardButton::callback("❌ Не опоздал", "unlate")
                    ]
                ]);
                
                bot.send_message(msg.chat.id, stats_message)
                    .reply_markup(keyboard)
                    .await?;
            } else {
                bot.send_message(msg.chat.id, "Произошла ошибка при получении статистики. Пожалуйста, попробуйте позже.").await?;
            }
        }
        Some("/get_chat_id") => {
            bot.send_message(
                msg.chat.id,
                format!("ID этого чата: {}", msg.chat.id)
            ).await?;
        }
        _ => {
            bot.send_message(
                msg.chat.id,
                "Используйте /start для информации, /late для голосования за опоздание, /unlate для голосования против, /stats для статистики за сегодня, /stats_day YYYY-MM-DD для статистики за конкретный день или /get_chat_id для получения ID чата"
            ).await?;
        }
    }
    Ok(())
}

async fn handle_callback(bot: Bot, q: CallbackQuery, database_service: DatabaseService) -> Result<(), RequestError> {
    if let Some(data) = q.data {
        match data.as_str() {
            "late" | "unlate" => {
                let user_id = q.from.id.0 as i64;
                let is_late = data == "late";
                
                match database_service.vote(user_id, is_late).await {
                    Ok(_) => {
                        let vote_type = if is_late { "за опоздание" } else { "против опоздания" };
                        bot.answer_callback_query(q.id)
                            .text(format!("✅ Ваш голос {} успешно зарегистрирован!", vote_type))
                            .await?;
                    }
                    Err(e) => {
                        log::error!("Ошибка при голосовании: {}", e);
                        bot.answer_callback_query(q.id)
                            .text("❌ Произошла ошибка при регистрации голоса. Пожалуйста, попробуйте позже.")
                            .await?;
                    }
                }
            }
            "stats" => {
                if let Ok(today_document) = database_service.check_today_document().await {
                    let user_id = q.from.id.0 as i64;
                    let user_vote = if today_document.votes_yes.contains(&user_id) {
                        "✅ Вы сегодня голосовали ЗА опоздание"
                    } else if today_document.votes_no.contains(&user_id) {
                        "❌ Вы сегодня голосовали ПРОТИВ опоздания"
                    } else {
                        "⚠️ Вы сегодня еще не голосовали"
                    };

                    let stats_message = format!(
                        "📊 Статистика за сегодня:\n\n\
                        За опоздание: {} голосов\n\
                        Против опоздания: {} голосов\n\n\
                        Всего проголосовало: {} человек\n\n\
                        {}",
                        today_document.votes_yes.len(),
                        today_document.votes_no.len(),
                        today_document.votes_yes.len() + today_document.votes_no.len(),
                        user_vote
                    );
                    
                    let keyboard = InlineKeyboardMarkup::new(vec![
                        vec![
                            InlineKeyboardButton::callback("✅ Опоздал", "late"),
                            InlineKeyboardButton::callback("❌ Не опоздал", "unlate")
                        ]
                    ]);
                    
                    bot.answer_callback_query(q.id).await?;
                    
                    if let Some(message) = q.message {
                        let chat = message.chat();
                        bot.send_message(chat.id, stats_message)
                            .reply_markup(keyboard)
                            .await?;
                        
                    }
                } else {
                    bot.answer_callback_query(q.id)
                        .text("❌ Произошла ошибка при получении статистики. Пожалуйста, попробуйте позже.")
                        .await?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
