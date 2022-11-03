use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serenity::model::prelude::{ChannelId, RoleId};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Unable to read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unable to deserialize config file: {0}")]
    Deserialize(#[from] toml::de::Error),
}

#[cfg(target_os = "windows")]
pub const DEFAULT_PATH: &str = r"C:\Program Files\megabot\config.toml";

#[cfg(target_family = "unix")]
pub const DEFAULT_PATH: &str = "/etc/megabot/config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub moderator_channel_id: ChannelId,
    pub pin_roles: Vec<RoleId>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        toml::from_str(&contents).map_err(ConfigError::from)
    }
}
