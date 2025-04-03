use mongodb::{
    bson::{doc, DateTime},
    options::ClientOptions,
    Client,
};
use serde::{Deserialize, Serialize};
use teloxide::prelude::*;
use teloxide::RequestError;
use std::env;

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




    // Получаем имя из переменной окружения или используем значение по умолчанию
    let target_name = env::var("LATE_TARGET_NAME").unwrap_or_else(|_| "Поверинов".to_string());


    let database_service = database_actions::DatabaseServiceInner::new("mongodb://10.10.10.10:27017/").await;
    // Подключение к MongoDB
    

    let bot = Bot::from_env();

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let target_name = target_name.clone();
        let database_service = database_service.clone();
        async move {
            match msg.text() {
                Some("/start") => {
                    bot.send_message(
                        msg.chat.id,
                        format!("👋 Добро пожаловать в бот учета опозданий!\n\n\
                        🕒 Здесь вы можете голосовать, опоздал ли сегодня {}.\n\n\
                        Команды:\n\
                        /late - голосовать за опоздание\n\
                        /unlate - голосовать против опоздания\n\
                        /stats - посмотреть статистику\n\n\
                        ⚠️ Голосовать можно только один раз в день!", target_name)
                    ).await?;
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
                                format!("✅ Ваш голос {} успешно зарегистрирован!", vote_type)
                            ).await?;
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
                Some("/unlate") => {
                    let user_id = msg.from().unwrap().id;
                    let is_late = false;
                    
                    let _ = database_service.check_today_document().await;
                    match database_service.vote(user_id.0 as i64, is_late).await {
                        Ok(_) => {
                            bot.send_message(
                                msg.chat.id,
                                "✅ Ваш голос против опоздания успешно зарегистрирован!"
                            ).await?;
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
                        let stats_message = format!(
                            "📊 Статистика за сегодня:\n\n\
                            За опоздание: {} голосов\n\
                            Против опоздания: {} голосов\n\n\
                            Всего проголосовало: {} человек",
                            today_document.votes_yes.len(),
                            today_document.votes_no.len(),
                            today_document.votes_yes.len() + today_document.votes_no.len()
                        );
                    bot.send_message(msg.chat.id, stats_message).await?;
                    } else {
                        bot.send_message(msg.chat.id, "Произошла ошибка при получении статистики. Пожалуйста, попробуйте позже.").await?;
                    }
                }
                Some(text) if text.starts_with("/stats_day") => {
                    let args: Vec<&str> = text.split_whitespace().collect();
                    if args.len() != 2 {
                        bot.send_message(
                            msg.chat.id,
                            "Используйте формат: /stats_day YYYY-MM-DD\nНапример: /stats_day 2024-03-20"
                        ).await?;
                        return Ok(());
                    }

                    match DateTime::parse_rfc3339_str(args[1]) {
                        Ok(date) => {
                            match database_service.get_day_stats(date).await {
                                Ok(day) => {
                                    let stats_message = format!(
                                        "📊 Статистика за {}:\n\n\
                                        За опоздание: {} голосов\n\
                                        Против опоздания: {} голосов\n\n\
                                        Всего проголосовало: {} человек",
                                        args[1],
                                        day.votes_yes.len(),
                                        day.votes_no.len(),
                                        day.votes_yes.len() + day.votes_no.len()
                                    );
                                    bot.send_message(msg.chat.id, stats_message).await?;
                                }
                                Err(_) => {
                                    bot.send_message(
                                        msg.chat.id,
                                        "❌ Документ за указанную дату не найден"
                                    ).await?;
                                }
                            }
                        }
                        Err(_) => {
                            bot.send_message(
                                msg.chat.id,
                                "❌ Неверный формат даты. Используйте YYYY-MM-DD"
                            ).await?;
                        }
                    }
                }
                _ => {
                    bot.send_message(
                        msg.chat.id,
                        "Используйте /start для информации, /late для голосования за опоздание, /unlate для голосования против, /stats для статистики за сегодня или /stats_day YYYY-MM-DD для статистики за конкретный день"
                    ).await?;
                }
            }
            Ok(())
        }
    })
    .await;
}
