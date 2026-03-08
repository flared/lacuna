use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Logging {
    #[serde(default = "Logging::default_format")]
    pub format: LogFormat,

    #[serde(default = "Logging::default_level")]
    pub level: String,
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            format: Self::default_format(),
            level: Self::default_level(),
        }
    }
}

impl Logging {
    fn default_format() -> LogFormat {
        LogFormat::Console
    }

    fn default_level() -> String {
        "info".to_owned()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Console,
    Json,
}

pub fn init(logging: &Logging) -> Result<(), anyhow::Error> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_new(&logging.level)
        .map_err(|e| anyhow::anyhow!("invalid log level '{}': {e}", logging.level))?;

    let registry = tracing_subscriber::registry().with(filter);

    match logging.format {
        LogFormat::Json => registry.with(fmt::layer().json()).init(),
        LogFormat::Console => registry.with(fmt::layer()).init(),
    }

    Ok(())
}
