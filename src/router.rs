use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, MessageId, ThreadId};
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;

use crate::fsm::Fsm;
use crate::module::{InputPayload, RouteKind, Mode};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
}

/// Классификация текстовых сообщений (чистая функция)
fn classify_message(msg: &Message, input_thread_id: i32) -> Option<RouteKind> {
    if msg.thread_id != Some(ThreadId(MessageId(input_thread_id))) {
        return None;
    }
    msg.text().map(|_| RouteKind::ModuleInput {
        payload: InputPayload::RawMessage(Box::new(msg.clone())),
    })
}

/// Классификация callback-запросов (чистая функция)
fn classify_callback(q: &CallbackQuery) -> Option<RouteKind> {
    let data = q.data.as_deref()?;
    if data.starts_with("menu:") {
        let target = match data {
            "menu:notes" => Mode::Notes,
            "menu:music" => Mode::Music,
            "menu:summary" => Mode::Summary,
            "menu:status" => Mode::Status,
            "menu:main" => Mode::MainMenu,
            _ => return None,
        };
        Some(RouteKind::MenuNavigation { target })
    } else {
        Some(RouteKind::ModuleInput {
            payload: InputPayload::RawCallback(Box::new(q.clone())),
        })
    }
}

pub fn create_dispatcher(
    bot: Bot,
    fsm: Arc<Mutex<Fsm>>,
    input_thread_id: i32,
) -> Dispatcher<Bot, Box<dyn std::error::Error + Send + Sync>, teloxide::dispatching::DefaultKey> {
    let fsm_cmd = fsm.clone();
    let fsm_msg = fsm.clone();
    let fsm_edit = fsm.clone();
    let fsm_cb = fsm.clone();
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                    let fsm = fsm_cmd.clone();
                    async move {
                        if matches!(cmd, Command::Start) {
                            let mut guard = fsm.lock().await;
                            guard.reset_to_main_menu(&bot).await;
                            // Delete the /start command message
                            let _ = bot.delete_message(msg.chat.id, msg.id).await;
                        }
                        Ok(())
                    }
                }),
        )
        .branch(
            Update::filter_message()
                .filter_map(move |msg: Message| classify_message(&msg, input_thread_id))
                .endpoint(move |bot: Bot, msg: Message, route: RouteKind| {
                    let fsm = fsm_msg.clone();
                    async move {
                        let mut guard = fsm.lock().await;
                        guard.handle_routed(&bot, msg.chat.id.0, route, Some(msg.id.0 as i64)).await;
                        Ok(())
                    }
                }),
        )
        .branch(
            Update::filter_edited_message()
                .filter_map(move |msg: Message| classify_message(&msg, input_thread_id))
                .endpoint(move |bot: Bot, msg: Message, route: RouteKind| {
                    let fsm = fsm_edit.clone();
                    async move {
                        // ponyail: edited messages pass msg_id=0 to avoid deletion
                        let mut guard = fsm.lock().await;
                        guard.handle_routed(&bot, msg.chat.id.0, route, Some(msg.id.0 as i64)).await;
                        Ok(())
                    }
                }),
        )
        .branch(
            Update::filter_callback_query()
                .endpoint(move |bot: Bot, q: CallbackQuery| {
                    let fsm = fsm_cb.clone();
                    async move {
                        // гасим анимацию на кнопке сразу; не зависит от исхода обработки
                        let _ = bot.answer_callback_query(q.id.clone()).await;

                        let Some(route) = classify_callback(&q) else {
                            return Ok(());
                        };
                        let chat_id = q.message.as_ref().map(|m| m.chat().id.0).unwrap_or(0);
                        let mut guard = fsm.lock().await;
                        guard.handle_routed(&bot, chat_id, route, None).await;
                        Ok(())
                    }
                }),
        );

    Dispatcher::builder(bot, handler)
        .default_handler(|_| async {})
        .build()
}
