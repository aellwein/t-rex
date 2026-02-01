//! The saved server address and token, kept in the user's config folder.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Address of the server, for example `https://t-rex.example.com`.
    pub server: Option<String>,
    /// The token this computer uses. Keep the file private.
    pub token: Option<String>,
    /// Id of the token on the server, used to revoke it on logout.
    pub token_id: Option<i64>,
}

/// Where the config file lives: `<config dir>/t-rex/config.toml`.
pub fn path() -> Result<PathBuf> {
    let dir = dirs::config_dir().ok_or_else(|| anyhow!("no config folder on this system"))?;
    Ok(dir.join("t-rex").join("config.toml"))
}

pub fn load() -> Result<Config> {
    let path = path()?;
    match fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).with_context(|| format!("reading {}", path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(err) => Err(err).with_context(|| format!("reading {}", path.display())),
    }
}

/// Writes the config. On Unix the file is only readable by its owner, because it holds a token.
pub fn save(config: &Config) -> Result<()> {
    let path = path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let text = toml::to_string_pretty(config).context("writing the config")?;
    fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("restricting {}", path.display()))?;
    }
    Ok(())
}
