use ::std::fs;
use ::std::path::Path;

use ::anyhow::{Context, Result};
use ::serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub default_interval: Option<String>,
    pub default_log_file: Option<String>,
    pub environment: Option<Vec<(String, String)>>,
    pub max_history_entries: Option<usize>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(path)
            .context(format!("Failed to read config file at {}", path.display()))?;

        serde_yml::from_str(&config_str)
            .context(format!("Failed to parse config file at {}", path.display()))
    }
}
