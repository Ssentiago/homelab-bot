use std::collections::HashMap;
use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::ThreadId;
use tokio::sync::mpsc;
use tracing::{info, error};

type MessageSender = mpsc::Sender<Message>;
type MessageReceiver = mpsc::Receiver<Message>;

pub struct Router {
    routes: HashMap<ThreadId, MessageSender>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn register(&mut self, thread_id: ThreadId) -> MessageReceiver {
        let (tx, rx) = mpsc::channel(32);
        self.routes.insert(thread_id, tx);
        rx
    }

    pub async fn run(self, bot: Bot) {
        let routes = Arc::new(self.routes);

        let handler = move |_bot: Bot, msg: Message| {
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

        info!("Router started, dispatching messages");

        let update_handler = Update::filter_message()
            .chain(Update::filter_edited_message())
            .endpoint(handler);

        Dispatcher::builder(bot, update_handler)
            .build()
            .dispatch()
            .await;
    }
}
