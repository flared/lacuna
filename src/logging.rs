use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Logging {
    pub format: LogFormat,
    pub level: LogLevel,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Console,
    Json,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
        }
    }
}

pub fn init(logging: &Logging) -> anyhow::Result<()> {
    use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*};

    let filter = LevelFilter::from_level(logging.level.into());

    let registry = tracing_subscriber::registry().with(filter);

    match logging.format {
        LogFormat::Json => registry.with(fmt::layer().json()).init(),
        LogFormat::Console => registry.with(fmt::layer()).init(),
    }

    Ok(())
}
