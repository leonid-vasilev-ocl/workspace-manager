use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    pub path: PathBuf,
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    workspaces: Vec<Workspace>,
}

fn get_path() -> Result<PathBuf> {
    let path = std::env::home_dir()
        .ok_or(anyhow!("can't get home dir"))?
        .join(".config")
        .join("wsm")
        .join("config.json");

    Ok(path)
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = get_path()?;

        if !config_path.exists() {
            return Ok(Config { workspaces: vec![] });
        }

        let config_str = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = get_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let writer = fs::File::create(config_path)?;
        Ok(serde_json::to_writer(writer, &self)?)
    }

    pub fn has_ws(&self, path: &std::path::Path) -> bool {
        self.workspaces.iter().any(|ws| ws.path == path)
    }

    pub fn get_ws_all(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn add_ws<P: AsRef<Path>>(&mut self, path: P) {
        let p = path.as_ref();
        self.workspaces.push(Workspace {
            path: p.to_path_buf(),
        });
    }

    pub fn remove_ws(&mut self, path: &std::path::Path) -> bool {
        let original_len = self.workspaces.len();
        self.workspaces.retain(|ws| ws.path != path);
        original_len > self.workspaces.len()
    }
}
