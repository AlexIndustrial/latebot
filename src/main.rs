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
    let notification_chat_id = env::var("NOTIFICATION_CHAT_ID").unwrap_or_else(|_| "0".to_string()).parse::<i64>().unwrap_or(0);
    let ping_user = env::var("PING_USER").unwrap_or_else(|_| "@Test".to_string());


    let database_service = database_actions::DatabaseServiceInner::new("mongodb://10.10.10.10:27017/").await;
    // Подключение к MongoDB
    

    let bot = Bot::from_env();

    


//     return;

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let target_name = target_name.clone();
        let ping_user = ping_user.clone();
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
                        /stats - посмотреть статистику\n\
                        /get_chat_id - получить ID текущего чата\n\n\
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

                            // Проверяем количество опозданий, если голос был за опоздание
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

                    // Парсим дату в формате YYYY-MM-DD
                    let date_parts: Vec<&str> = args[1].split('-').collect();
                    if date_parts.len() != 3 {
                        bot.send_message(
                            msg.chat.id,
                            "❌ Неверный формат даты. Используйте YYYY-MM-DD"
                        ).await?;
                        return Ok(());
                    }

                    let year = date_parts[0].parse::<i32>().unwrap_or(0);
                    let month = date_parts[1].parse::<u8>().unwrap_or(0);
                    let day = date_parts[2].parse::<u8>().unwrap_or(0);

                    if year == 0 || month == 0 || day == 0 {
                        bot.send_message(
                            msg.chat.id,
                            "❌ Неверный формат даты. Используйте YYYY-MM-DD"
                        ).await?;
                        return Ok(());
                    }

                    // Создаем DateTime на начало указанного дня
                    let date = DateTime::from_millis(
                        chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
                            .unwrap_or_else(|| chrono::NaiveDate::from_ymd(1970, 1, 1))
                            .and_hms_opt(0, 0, 0)
                            .unwrap()
                            .timestamp_millis()
                    );

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
    })
    .await;
}
