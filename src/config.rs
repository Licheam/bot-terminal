use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};

#[derive(Clone, Debug)]
pub struct Config {
    pub telegram_bot_token: String,
    pub allowed_user_ids: HashSet<u64>,
    pub working_dir: PathBuf,
    pub command_timeout: Duration,
    pub max_output_chars: usize,
    pub env_file_path: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let current_dir =
            env::current_dir().context("failed to resolve current working directory")?;
        let telegram_bot_token = required_env("TELEGRAM_BOT_TOKEN")?;
        let allowed_user_ids = parse_user_ids(env::var("BOT_ALLOWED_USER_IDS").ok())?;
        let working_dir = parse_workdir(env::var("BOT_WORKDIR").ok())?;
        let command_timeout = Duration::from_secs(parse_u64_env("BOT_COMMAND_TIMEOUT_SECS", 20)?);
        let max_output_chars = parse_usize_env("BOT_MAX_OUTPUT_CHARS", 3000)?;
        let env_file_path = current_dir.join(".env");

        Ok(Self {
            telegram_bot_token,
            allowed_user_ids,
            working_dir,
            command_timeout,
            max_output_chars,
            env_file_path,
        })
    }
}

fn required_env(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("{key} is required but missing"))
}

fn parse_user_ids(raw: Option<String>) -> Result<HashSet<u64>> {
    let mut ids = HashSet::new();

    let Some(raw) = raw else {
        return Ok(ids);
    };

    for item in raw.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }

        let user_id = trimmed
            .parse::<u64>()
            .with_context(|| format!("failed to parse Telegram user id: {trimmed}"))?;

        ids.insert(user_id);
    }

    Ok(ids)
}

fn parse_workdir(raw: Option<String>) -> Result<PathBuf> {
    let path = match raw {
        Some(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => env::current_dir().context("failed to resolve current working directory")?,
    };

    if !path.exists() {
        bail!("BOT_WORKDIR does not exist: {}", path.display());
    }

    if !path.is_dir() {
        bail!("BOT_WORKDIR is not a directory: {}", path.display());
    }

    path.canonicalize()
        .with_context(|| format!("failed to canonicalize BOT_WORKDIR: {}", path.display()))
}

fn parse_u64_env(key: &str, default: u64) -> Result<u64> {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<u64>()
            .with_context(|| format!("{key} must be a positive integer")),
        Ok(_) | Err(env::VarError::NotPresent) => Ok(default),
        Err(err) => Err(anyhow!("failed to read {key}: {err}")),
    }
}

fn parse_usize_env(key: &str, default: usize) -> Result<usize> {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<usize>()
            .with_context(|| format!("{key} must be a positive integer")),
        Ok(_) | Err(env::VarError::NotPresent) => Ok(default),
        Err(err) => Err(anyhow!("failed to read {key}: {err}")),
    }
}

pub fn validate_workdir(raw: &str) -> Result<PathBuf> {
    parse_workdir(Some(raw.to_owned()))
}

pub fn write_env_value(env_file_path: &Path, key: &str, value: &str) -> Result<()> {
    let existing = match fs::read_to_string(env_file_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to read env file: {}", env_file_path.display()));
        }
    };

    let mut lines = Vec::new();
    let mut updated = false;

    for line in existing.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&format!("{key}=")) {
            lines.push(format!("{key}={value}"));
            updated = true;
        } else {
            lines.push(line.to_owned());
        }
    }

    if !updated {
        lines.push(format!("{key}={value}"));
    }

    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }

    fs::write(env_file_path, content)
        .with_context(|| format!("failed to write env file: {}", env_file_path.display()))
}
