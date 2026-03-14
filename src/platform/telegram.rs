use std::{sync::Arc, time::Duration};

use anyhow::Result;
use teloxide::{prelude::*, respond};
use tracing::{error, info};

use crate::{
    config::Config,
    terminal::{CommandResult, RuntimeSettings, TerminalError, TerminalService},
};

const TELEGRAM_MESSAGE_LIMIT: usize = 3900;

pub struct TelegramBot {
    config: Config,
    terminal: Arc<TerminalService>,
}

impl TelegramBot {
    pub fn new(config: Config, terminal: Arc<TerminalService>) -> Self {
        Self { config, terminal }
    }

    pub async fn run(self) -> Result<()> {
        info!(
            workdir = %self.terminal.working_dir().display(),
            authorized_users = self.terminal.authorized_user_count(),
            timeout_secs = self.terminal.timeout().as_secs(),
            "starting telegram bot"
        );

        let bot = Bot::new(self.config.telegram_bot_token);
        let terminal = Arc::clone(&self.terminal);

        teloxide::repl(bot, move |bot: Bot, msg: Message| {
            let terminal = Arc::clone(&terminal);

            async move {
                if let Err(err) = handle_message(bot, msg, terminal).await {
                    error!(?err, "telegram message handling failed");
                }

                respond(())
            }
        })
        .await;

        Ok(())
    }
}

async fn handle_message(bot: Bot, msg: Message, terminal: Arc<TerminalService>) -> Result<()> {
    let Some(text) = msg.text() else {
        return Ok(());
    };

    let request = parse_request(text);
    if matches!(request, BotRequest::Ignore) {
        return Ok(());
    }

    let reply = match request {
        BotRequest::Help => build_help_text(terminal.as_ref()),
        BotRequest::ShowConfig => {
            let Some(user) = msg.from.as_ref() else {
                return Ok(());
            };

            if !terminal.is_user_allowed(user.id.0) {
                format_command_error(TerminalError::Unauthorized)
            } else {
                format_runtime_settings(terminal.current_settings())
            }
        }
        BotRequest::SetWorkdir(path) => {
            let Some(user) = msg.from.as_ref() else {
                return Ok(());
            };

            match terminal.update_workdir_for_user(user.id.0, &path) {
                Ok(settings) => format_setting_updated("BOT_WORKDIR", settings),
                Err(err) => format_command_error(err),
            }
        }
        BotRequest::SetTimeout(seconds) => {
            let Some(user) = msg.from.as_ref() else {
                return Ok(());
            };

            match terminal.update_timeout_for_user(user.id.0, &seconds) {
                Ok(settings) => format_setting_updated("BOT_COMMAND_TIMEOUT_SECS", settings),
                Err(err) => format_command_error(err),
            }
        }
        BotRequest::SetMaxOutput(chars) => {
            let Some(user) = msg.from.as_ref() else {
                return Ok(());
            };

            match terminal.update_max_output_for_user(user.id.0, &chars) {
                Ok(settings) => format_setting_updated("BOT_MAX_OUTPUT_CHARS", settings),
                Err(err) => format_command_error(err),
            }
        }
        BotRequest::Run(command) => {
            let Some(user) = msg.from.as_ref() else {
                return Ok(());
            };

            match terminal.execute_for_user(user.id.0, &command).await {
                Ok(result) => format_command_result(&command, result),
                Err(err) => format_command_error(err),
            }
        }
        BotRequest::Ignore => return Ok(()),
    };

    bot.send_message(msg.chat.id, truncate_chars(&reply, TELEGRAM_MESSAGE_LIMIT))
        .await?;

    Ok(())
}

enum BotRequest {
    Help,
    ShowConfig,
    SetWorkdir(String),
    SetTimeout(String),
    SetMaxOutput(String),
    Run(String),
    Ignore,
}

fn parse_request(text: &str) -> BotRequest {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return BotRequest::Ignore;
    }

    if let Some(command) = trimmed.strip_prefix("!run") {
        return BotRequest::Run(command.trim().to_owned());
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let head = parts.next().unwrap_or_default();
    let tail = parts.next().unwrap_or_default().trim();
    let normalized_head = head.split('@').next().unwrap_or(head);

    match normalized_head {
        "/start" | "/help" => BotRequest::Help,
        "/config" => BotRequest::ShowConfig,
        "/set_workdir" => BotRequest::SetWorkdir(tail.to_owned()),
        "/set_timeout" => BotRequest::SetTimeout(tail.to_owned()),
        "/set_max_output" => BotRequest::SetMaxOutput(tail.to_owned()),
        "/run" => BotRequest::Run(tail.to_owned()),
        _ => BotRequest::Ignore,
    }
}

fn build_help_text(terminal: &TerminalService) -> String {
    let working_dir = terminal.working_dir();

    format!(
        concat!(
            "bot-terminal\n\n",
            "Commands:\n",
            "/help - show this message\n",
            "/config - show current runtime settings\n",
            "/run <command> - execute a shell command\n\n",
            "/set_workdir <path> - update BOT_WORKDIR\n",
            "/set_timeout <seconds> - update BOT_COMMAND_TIMEOUT_SECS\n",
            "/set_max_output <chars> - update BOT_MAX_OUTPUT_CHARS\n\n",
            "Current limits:\n",
            "- workdir: {}\n",
            "- timeout: {} seconds\n",
            "- max output chars: {}\n",
            "- allowed users: {}\n\n",
            "Example:\n",
            "/run pwd"
        ),
        working_dir.display(),
        terminal.timeout().as_secs(),
        terminal.max_output_chars(),
        terminal.authorized_user_count()
    )
}

fn format_command_result(command: &str, result: CommandResult) -> String {
    let status = match result.exit_code {
        Some(code) => code.to_string(),
        None => "terminated by signal".to_owned(),
    };

    let stdout = if result.stdout.trim().is_empty() {
        "(empty)".to_owned()
    } else {
        result.stdout
    };

    let stderr = if result.stderr.trim().is_empty() {
        "(empty)".to_owned()
    } else {
        result.stderr
    };

    format!(
        concat!("$ {}\n", "exit: {}\n\n", "stdout:\n{}\n\n", "stderr:\n{}"),
        command, status, stdout, stderr
    )
}

fn format_command_error(error: TerminalError) -> String {
    match error {
        TerminalError::Unauthorized => {
            "you are not allowed to run commands with this bot".to_owned()
        }
        TerminalError::EmptyCommand => "usage: /run <command>".to_owned(),
        TerminalError::TimedOut(timeout) => {
            format!("command timed out after {} seconds", duration_secs(timeout))
        }
        TerminalError::Spawn(err) => format!("failed to start command: {err}"),
        TerminalError::Wait(err) => format!("failed while waiting for command: {err}"),
        TerminalError::InvalidSetting(message) => message,
        TerminalError::Persist(message) => message,
    }
}

fn format_runtime_settings(settings: RuntimeSettings) -> String {
    format!(
        concat!(
            "Current runtime settings:\n",
            "- BOT_WORKDIR={}\n",
            "- BOT_COMMAND_TIMEOUT_SECS={}\n",
            "- BOT_MAX_OUTPUT_CHARS={}"
        ),
        settings.working_dir.display(),
        settings.timeout.as_secs(),
        settings.max_output_chars
    )
}

fn format_setting_updated(key: &str, settings: RuntimeSettings) -> String {
    format!(
        concat!(
            "{} updated successfully.\n\n",
            "Current runtime settings:\n",
            "- BOT_WORKDIR={}\n",
            "- BOT_COMMAND_TIMEOUT_SECS={}\n",
            "- BOT_MAX_OUTPUT_CHARS={}"
        ),
        key,
        settings.working_dir.display(),
        settings.timeout.as_secs(),
        settings.max_output_chars
    )
}

fn truncate_chars(text: &str, limit: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= limit {
        return text.to_owned();
    }

    let mut truncated: String = text.chars().take(limit).collect();
    truncated.push_str("\n...[truncated]");
    truncated
}

fn duration_secs(duration: Duration) -> u64 {
    duration.as_secs()
}
