use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{process::Command, time::timeout};

use crate::config::Config;

#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("the current user is not authorized")]
    Unauthorized,
    #[error("command must not be empty")]
    EmptyCommand,
    #[error("command timed out after {0:?}")]
    TimedOut(Duration),
    #[error("failed to start command")]
    Spawn(#[source] std::io::Error),
    #[error("failed to wait for command")]
    Wait(#[source] std::io::Error),
}

#[derive(Debug)]
pub struct TerminalService {
    allowed_user_ids: HashSet<u64>,
    working_dir: PathBuf,
    timeout: Duration,
    max_output_chars: usize,
}

impl TerminalService {
    pub fn new(config: &Config) -> Self {
        Self {
            allowed_user_ids: config.allowed_user_ids.clone(),
            working_dir: config.working_dir.clone(),
            timeout: config.command_timeout,
            max_output_chars: config.max_output_chars,
        }
    }

    pub async fn execute_for_user(
        &self,
        user_id: u64,
        command: &str,
    ) -> Result<CommandResult, TerminalError> {
        if !self.allowed_user_ids.contains(&user_id) {
            return Err(TerminalError::Unauthorized);
        }

        let command = command.trim();
        if command.is_empty() {
            return Err(TerminalError::EmptyCommand);
        }

        let mut child = Command::new("sh");
        child
            .arg("-lc")
            .arg(command)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = child.spawn().map_err(TerminalError::Spawn)?;
        let output = timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| TerminalError::TimedOut(self.timeout))?
            .map_err(TerminalError::Wait)?;

        Ok(CommandResult {
            exit_code: output.status.code(),
            stdout: truncate_output(
                &String::from_utf8_lossy(&output.stdout),
                self.max_output_chars,
            ),
            stderr: truncate_output(
                &String::from_utf8_lossy(&output.stderr),
                self.max_output_chars,
            ),
        })
    }

    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn authorized_user_count(&self) -> usize {
        self.allowed_user_ids.len()
    }
}

fn truncate_output(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }

    let mut truncated: String = text.chars().take(limit).collect();
    truncated.push_str("\n...[truncated]");
    truncated
}
