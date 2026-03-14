use std::sync::Arc;

use anyhow::Result;

use crate::{config::Config, platform::telegram::TelegramBot, terminal::TerminalService};

pub async fn run(config: Config) -> Result<()> {
    let terminal = Arc::new(TerminalService::new(&config));
    let bot = TelegramBot::new(config, terminal);

    bot.run().await
}
