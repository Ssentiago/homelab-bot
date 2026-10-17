#![allow(dead_code)]

use std::sync::Arc;
use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, ThreadId};
use tracing::{info, warn, instrument};

use crate::config::Config;
use crate::module::{Action, InputPayload, Mode, ModuleError, RouteKind, ThreadTarget};
use crate::modules::quick_notes::notes_module::NotesModule;
use crate::ui::screens::Screen;
use crate::ui::UiPanel;

pub struct Fsm {
    pub mode: Mode,
    pub config: Arc<Config>,
    pub pool: SqlitePool,
    pub ui: UiPanel,
    pub notes: NotesModule,
}

impl Fsm {
    pub async fn new(
        bot: &Bot,
        config: Arc<Config>,
        pool: SqlitePool,
        alive: Arc<std::sync::Mutex<tokio::time::Instant>>,
    ) -> Self {
        let ui = UiPanel::new(bot, &config, &pool).await;
        let notes = NotesModule::new(bot, config.clone(), pool.clone(), alive);
        Self {
            mode: Mode::MainMenu,
            config,
            pool,
            ui,
            notes,
        }
    }

    /// /start handler — force-reset FSM, delete old UI, send fresh panel
    #[instrument(skip(self, bot), fields(mode = ?self.mode))]
    pub async fn reset_to_main_menu(&mut self, bot: &Bot) {
        info!("reset to main menu from {:?}", self.mode);
        self.switch_mode_from(bot, Mode::MainMenu).await;
        self.ui.force_resend(&Screen::Main).await;
    }

    /// Единая точка входа — вызывается из Router после классификации мидлварью
    #[instrument(skip(self, bot), fields(mode = ?self.mode, chat_id = %chat_id))]
    pub async fn handle_routed(
        &mut self,
        bot: &Bot,
        chat_id: i64,
        route: RouteKind,
        msg_id: Option<i64>,
    ) {
        let result = match route {
            RouteKind::MenuNavigation { target } => {
                self.handle_menu_navigation(bot, target).await
            }
            RouteKind::ModuleInput { payload } => {
                self.handle_module_input(payload).await
            }
        };

        self.execute_actions(bot, chat_id, msg_id, result).await;
    }

    #[instrument(skip(self, bot), fields(target = ?target))]
    async fn handle_menu_navigation(
        &mut self,
        bot: &Bot,
        target: Mode,
    ) -> Result<Action, ModuleError> {
        match target {
            Mode::MainMenu => {
                self.switch_mode_from(bot, Mode::MainMenu).await;
                Ok(Action::EditUI { screen: Screen::Main })
            }
            Mode::Notes => {
                self.switch_mode_from(bot, Mode::Notes).await;
                Ok(Action::EditUI { screen: Screen::Notes })
            }
            Mode::Status => {
                self.switch_mode_from(bot, Mode::Status).await;
                Ok(Action::EditUI {
                    screen: Screen::Status {
                        chat_id: self.config.chat_id,
                        panel_topic: self.config.thread_ids.input.unwrap_or(0),
                        monitor_topic: self.config.thread_ids.output.unwrap_or(0),
                    },
                })
            }
            Mode::Music | Mode::Summary => {
                Ok(Action::EditUI { screen: Screen::ComingSoon })
            }
        }
    }

    async fn handle_module_input(
        &mut self,
        payload: InputPayload,
    ) -> Result<Action, ModuleError> {
        match payload {
            InputPayload::RawMessage(msg) => {
                match self.mode {
                    Mode::Notes => self.notes.handle_message(*msg).await,
                    _ => Ok(Action::Noop),
                }
            },
            InputPayload::RawCallback(q) => match self.mode {
                Mode::Notes => self.notes.handle_callback(*q).await,
                _ => Ok(Action::Noop),
            },
            // ponytail: old Text/Edit/Callback variants — kept for non-Notes modes
            InputPayload::Text(_) | InputPayload::Edit(_) | InputPayload::Callback(_) => {
                self.handle_legacy_input(payload).await
            }
        }
    }

    /// Legacy handler for Text/Edit/Callback payloads (non-Notes modes)
    async fn handle_legacy_input(
        &mut self,
        payload: InputPayload,
    ) -> Result<Action, ModuleError> {
        match payload {
            InputPayload::Text(_) => match self.mode {
                Mode::MainMenu | Mode::Status => Ok(Action::Noop),
                _ => Ok(Action::EditUI { screen: Screen::ComingSoon }),
            },
            InputPayload::Edit(_) => Ok(Action::Noop),
            InputPayload::Callback(_) => Ok(Action::Noop),
            _ => Ok(Action::Noop),
        }
    }

    #[instrument(skip(self, bot), fields(from = ?self.mode, to = ?new_mode))]
    async fn switch_mode_from(&mut self, bot: &Bot, new_mode: Mode) {
        match self.mode {
            Mode::Notes => {
                info!("exiting Notes mode");
                self.notes.exit(bot).await;
            }
            Mode::MainMenu | Mode::Music | Mode::Summary | Mode::Status => {}
        }
        self.mode = new_mode;
    }

    #[instrument(skip(self, bot), fields(mode = ?self.mode))]
    async fn execute_actions(
        &mut self,
        bot: &Bot,
        chat_id: i64,
        msg_id: Option<i64>,
        result: Result<Action, ModuleError>,
    ) {
        let action = match result {
            Ok(a) => {
                if let Some(id) = msg_id
                    && matches!(self.mode, Mode::MainMenu)
                {
                    info!("delete user message {} in MainMenu", id);
                    let Ok(mid) = i32::try_from(id) else { return; };
                    let _ = bot.delete_message(ChatId(chat_id), MessageId(mid)).await;
                }
                a
            }
            Err(ModuleError::Processing(e)) => {
                warn!("module processing error: {}", e);
                Action::EditUI {
                    screen: Screen::Error { message: e, fatal: false },
                }
            }
            Err(ModuleError::Fatal(e)) => {
                info!("module fatal error, resetting to MainMenu: {}", e);
                self.mode = Mode::MainMenu;
                Action::EditUI {
                    screen: Screen::Error { message: e, fatal: true },
                }
            }
        };

        match action {
            Action::EditUI { screen } => {
                self.ui.edit(&screen).await;
            }
            Action::SendMessage { text, thread } => {
                let thread_id = match thread {
                    ThreadTarget::Input => self.config.thread_ids.input,
                    ThreadTarget::Output => self.config.thread_ids.output,
                };
                let _ = bot
                    .send_message(ChatId(chat_id), text)
                    .message_thread_id(ThreadId(MessageId(thread_id.unwrap_or(0))))
                    .await;
            }
            Action::DeleteMessages(ids) => {
                for id in ids {
                    let Ok(mid) = i32::try_from(id) else { continue; };
                    let _ = bot.delete_message(ChatId(chat_id), MessageId(mid)).await;
                }
            }
            Action::SwitchMode(new_mode) => {
                self.switch_mode_from(bot, new_mode).await;
            }
            Action::Noop => {}
        }
    }
}
