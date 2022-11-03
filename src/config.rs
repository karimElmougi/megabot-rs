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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feature {
    /// Pinning/unpinning messages through an emoji on behalf of contributors.
    Pins,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// ID of the #mod-messages channel.
    pub moderator_channel_id: ChannelId,

    /// List of role IDs that are allowed to pin/unpin using reactions.
    pub pin_roles: Vec<RoleId>,

    /// Currently enabled feature flags.
    pub enabled_features: Vec<Feature>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        toml::from_str(&contents).map_err(ConfigError::from)
    }
}
