use std::{collections::HashSet, path::PathBuf, process::Stdio, sync::RwLock, time::Duration};

use thiserror::Error;
use tokio::{process::Command, time::timeout};

use crate::config::{Config, validate_workdir, write_env_value};

#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug)]
pub struct RuntimeSettings {
    pub working_dir: PathBuf,
    pub timeout: Duration,
    pub max_output_chars: usize,
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
    #[error("{0}")]
    InvalidSetting(String),
    #[error("failed to persist configuration: {0}")]
    Persist(String),
}

#[derive(Debug)]
pub struct TerminalService {
    allowed_user_ids: HashSet<u64>,
    settings: RwLock<RuntimeSettings>,
    env_file_path: PathBuf,
}

impl TerminalService {
    pub fn new(config: &Config) -> Self {
        Self {
            allowed_user_ids: config.allowed_user_ids.clone(),
            settings: RwLock::new(RuntimeSettings {
                working_dir: config.working_dir.clone(),
                timeout: config.command_timeout,
                max_output_chars: config.max_output_chars,
            }),
            env_file_path: config.env_file_path.clone(),
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

        let settings = self.current_settings();
        let mut child = Command::new("sh");
        child
            .arg("-lc")
            .arg(command)
            .current_dir(&settings.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = child.spawn().map_err(TerminalError::Spawn)?;
        let output = timeout(settings.timeout, child.wait_with_output())
            .await
            .map_err(|_| TerminalError::TimedOut(settings.timeout))?
            .map_err(TerminalError::Wait)?;

        Ok(CommandResult {
            exit_code: output.status.code(),
            stdout: truncate_output(
                &String::from_utf8_lossy(&output.stdout),
                settings.max_output_chars,
            ),
            stderr: truncate_output(
                &String::from_utf8_lossy(&output.stderr),
                settings.max_output_chars,
            ),
        })
    }

    pub fn working_dir(&self) -> PathBuf {
        self.settings
            .read()
            .expect("terminal settings lock poisoned")
            .working_dir
            .clone()
    }

    pub fn timeout(&self) -> Duration {
        self.settings
            .read()
            .expect("terminal settings lock poisoned")
            .timeout
    }

    pub fn authorized_user_count(&self) -> usize {
        self.allowed_user_ids.len()
    }

    pub fn max_output_chars(&self) -> usize {
        self.settings
            .read()
            .expect("terminal settings lock poisoned")
            .max_output_chars
    }

    pub fn current_settings(&self) -> RuntimeSettings {
        self.settings
            .read()
            .expect("terminal settings lock poisoned")
            .clone()
    }

    pub fn is_user_allowed(&self, user_id: u64) -> bool {
        self.allowed_user_ids.contains(&user_id)
    }

    pub fn update_workdir_for_user(
        &self,
        user_id: u64,
        raw_path: &str,
    ) -> Result<RuntimeSettings, TerminalError> {
        self.ensure_user_allowed(user_id)?;

        let working_dir = validate_workdir(raw_path)
            .map_err(|err| TerminalError::InvalidSetting(err.to_string()))?;
        write_env_value(
            &self.env_file_path,
            "BOT_WORKDIR",
            &working_dir.to_string_lossy(),
        )
        .map_err(|err| TerminalError::Persist(err.to_string()))?;

        let mut settings = self
            .settings
            .write()
            .expect("terminal settings lock poisoned");
        settings.working_dir = working_dir;

        Ok(settings.clone())
    }

    pub fn update_timeout_for_user(
        &self,
        user_id: u64,
        seconds: &str,
    ) -> Result<RuntimeSettings, TerminalError> {
        self.ensure_user_allowed(user_id)?;

        let parsed = seconds.trim().parse::<u64>().map_err(|_| {
            TerminalError::InvalidSetting("timeout must be a positive integer".to_owned())
        })?;
        if parsed == 0 {
            return Err(TerminalError::InvalidSetting(
                "timeout must be greater than 0".to_owned(),
            ));
        }

        write_env_value(
            &self.env_file_path,
            "BOT_COMMAND_TIMEOUT_SECS",
            &parsed.to_string(),
        )
        .map_err(|err| TerminalError::Persist(err.to_string()))?;

        let mut settings = self
            .settings
            .write()
            .expect("terminal settings lock poisoned");
        settings.timeout = Duration::from_secs(parsed);

        Ok(settings.clone())
    }

    pub fn update_max_output_for_user(
        &self,
        user_id: u64,
        chars: &str,
    ) -> Result<RuntimeSettings, TerminalError> {
        self.ensure_user_allowed(user_id)?;

        let parsed = chars.trim().parse::<usize>().map_err(|_| {
            TerminalError::InvalidSetting("max output chars must be a positive integer".to_owned())
        })?;
        if parsed == 0 {
            return Err(TerminalError::InvalidSetting(
                "max output chars must be greater than 0".to_owned(),
            ));
        }

        write_env_value(
            &self.env_file_path,
            "BOT_MAX_OUTPUT_CHARS",
            &parsed.to_string(),
        )
        .map_err(|err| TerminalError::Persist(err.to_string()))?;

        let mut settings = self
            .settings
            .write()
            .expect("terminal settings lock poisoned");
        settings.max_output_chars = parsed;

        Ok(settings.clone())
    }

    fn ensure_user_allowed(&self, user_id: u64) -> Result<(), TerminalError> {
        if self.is_user_allowed(user_id) {
            Ok(())
        } else {
            Err(TerminalError::Unauthorized)
        }
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
