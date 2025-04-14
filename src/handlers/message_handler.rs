use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageEntityKind},
    RequestError,
};

use mongodb::bson::DateTime;
use crate::database_actions::DatabaseService;

pub async fn message_handler(
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
                    InlineKeyboardButton::callback("❌ Не опоздал", "unlate"),
                ],
                vec![InlineKeyboardButton::callback("📊 Статистика", "stats")],
            ]);

            bot.send_message(
                msg.chat.id,
                format!(
                    "👋 Добро пожаловать в бот учета опозданий!\n\n\
                🕒 Здесь вы можете голосовать, опоздал ли сегодня {}.\n\n\
                Команды:\n\
                /late - голосовать за опоздание\n\
                /unlate - голосовать против опоздания\n\
                /stats - посмотреть статистику\n\
                /get_chat_id - получить ID текущего чата\n\
                /get_user_id @username - информация о получении ID пользователя\n\
                /my_id - получить свой ID\n\n\
                ⚠️ Голосовать можно только один раз в день!",
                    target_name
                ),
            )
            .reply_markup(keyboard)
            .await?;
        }
        Some("/late") | Some("/unlate") => {
            let user_id = msg.from.as_ref().unwrap().id;
            let _username = msg
                .from
                .as_ref()
                .unwrap()
                .username
                .clone()
                .unwrap_or_else(|| "anonymous".to_string());
            let is_late = msg.text() == Some("/late");

            let _ = database_service.check_today_document().await;

            match database_service.vote(user_id.0 as i64, is_late).await {
                Ok(_) => {
                    let vote_type = if is_late {
                        "за опоздание"
                    } else {
                        "против опоздания"
                    };
                    bot.send_message(
                        msg.chat.id,
                        format!("✅ Ваш голос {} успешно зарегистрирован!", vote_type),
                    )
                    .await?;

                    if is_late {
                        if let Ok(total_late_days) = database_service.check_today_document().await {
                            if total_late_days.votes_yes.len() % 5 == 0 && notification_chat_id != 0
                            {
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
                        "❌ Произошла ошибка при регистрации голоса. Пожалуйста, попробуйте позже.",
                    )
                    .await?;
                }
            }
        }
        Some("/stats") => {
            if let Ok(today_document) = database_service.check_today_document().await {
                let user_id = msg.from.as_ref().unwrap().id.0 as i64;
                let user_vote = if today_document.votes_yes.contains(&user_id) {
                    "✅ Вы сегодня голосовали ЗА опоздание"
                } else if today_document.votes_no.contains(&user_id) {
                    "❌ Вы сегодня голосовали ПРОТИВ опоздания"
                } else {
                    "⚠️ Вы сегодня еще не голосовали"
                };

                let votes_yes = today_document.votes_yes.len();
                let votes_no = today_document.votes_no.len();

                let result_position = if votes_yes > votes_no {
                    "🟢 Сейчас побеждает позиция: ОПОЗДАЛ"
                } else if votes_no > votes_yes {
                    "🔴 Сейчас побеждает позиция: НЕ ОПОЗДАЛ"
                } else {
                    "🟡 Сейчас ничья в голосовании"
                };

                let stats_message = format!(
                    "📊 Статистика за сегодня:\n\n\
                    За опоздание: {} голосов\n\
                    Против опоздания: {} голосов\n\n\
                    Всего проголосовало: {} человек\n\
                    {}\n\n\
                    {}",
                    votes_yes,
                    votes_no,
                    votes_yes + votes_no,
                    result_position,
                    user_vote
                );

                let keyboard = InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback("✅ Опоздал", "late"),
                    InlineKeyboardButton::callback("❌ Не опоздал", "unlate"),
                ]]);

                bot.send_message(msg.chat.id, stats_message)
                    .reply_markup(keyboard)
                    .await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Произошла ошибка при получении статистики. Пожалуйста, попробуйте позже.",
                )
                .await?;
            }
        }
        Some("/get_chat_id") => {
            bot.send_message(msg.chat.id, format!("ID этого чата: {}", msg.chat.id))
                .await?;
        }
        Some("/my_id") => {
            if let Some(user) = &msg.from {
                bot.send_message(
                    msg.chat.id,
                    format!("Ваш ID: {}", user.id.0)
                ).await?;
            } else {
                bot.send_message(msg.chat.id, "Не удалось определить ваш ID").await?;
            }
        }
        _ => {
            bot.send_message(
                msg.chat.id,
                "Используйте /start для информации, /late для голосования за опоздание, /unlate для голосования против, /stats для статистики за сегодня, /get_chat_id для получения ID чата, /my_id для получения своего ID"
            ).await?;
        }
    }
    Ok(())
}
