use std::sync::Arc;
use teloxide::prelude::*;

use crate::config::Config;

pub async fn run(bot: Bot, config: Arc<Config>) {
    println!("Quick notes task started");
    // TODO: dispatcher logic for quick notes
}
