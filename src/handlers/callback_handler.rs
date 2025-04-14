use teloxide::{
    prelude::*,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup},
    RequestError,
};

use crate::database_actions::DatabaseService;

pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    database_service: DatabaseService,
) -> Result<(), RequestError> {
    if let Some(data) = q.data {
        match data.as_str() {
            "late" | "unlate" => {
                let user_id = q.from.id.0 as i64;
                let is_late = data == "late";

                match database_service.vote(user_id, is_late).await {
                    Ok(_) => {
                        let vote_type = if is_late {
                            "за опоздание"
                        } else {
                            "против опоздания"
                        };
                        bot.answer_callback_query(q.id)
                            .text(format!(
                                "✅ Ваш голос {} успешно зарегистрирован!",
                                vote_type
                            ))
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
