use std::collections::HashMap;
use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, ThreadId};
use tokio::sync::mpsc;
use tracing::{info, error};

type MessageSender = mpsc::Sender<Message>;
type MessageReceiver = mpsc::Receiver<Message>;

pub struct Router {
    routes: HashMap<ThreadId, MessageSender>,
    callback_sender: mpsc::Sender<CallbackQuery>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            callback_sender: mpsc::channel(32).0,
        }
    }

    pub fn register(&mut self, thread_id: ThreadId) -> MessageReceiver {
        let (tx, rx) = mpsc::channel(32);
        self.routes.insert(thread_id, tx);
        rx
    }

    pub fn register_callback(&mut self) -> mpsc::Receiver<CallbackQuery> {
        let (tx, rx) = mpsc::channel(32);
        self.callback_sender = tx;
        rx
    }

    pub async fn run(self, bot: Bot) {
        let routes = Arc::new(self.routes);
        let callback_sender = Arc::new(self.callback_sender);

        let message_handler = move |_bot: Bot, msg: Message| {
            let routes = routes.clone();
            async move {
                if let Some(thread_id) = msg.thread_id
                    && let Some(sender) = routes.get(&thread_id)
                    && let Err(e) = sender.send(msg).await
                {
                    error!("Failed to route message to thread {:?}: {}", thread_id, e);
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }
        };

        let callback_handler = move |_bot: Bot, query: CallbackQuery| {
            let sender = callback_sender.clone();
            async move {
                if let Err(e) = sender.send(query).await {
                    error!("Failed to route callback query: {}", e);
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }
        };

        info!("Router started, dispatching messages");

        let update_handler = dptree::entry()
            .branch(Update::filter_message().endpoint(message_handler.clone()))
            .branch(Update::filter_edited_message().endpoint(message_handler))
            .branch(Update::filter_callback_query().endpoint(callback_handler));

        Dispatcher::builder(bot, update_handler)
            .build()
            .dispatch()
            .await;
    }
}
