use serde::{Deserialize, Serialize};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub auto_enabled: bool,
    pub poll_interval_ms: u64,
    pub turn_off_when_netflix_closes: bool,
    pub selected_targets: Vec<String>,
    pub process_names: Vec<String>,
    pub detect_window_titles: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_enabled: true,
            poll_interval_ms: 1500,
            turn_off_when_netflix_closes: true,
            selected_targets: Vec::new(),
            process_names: vec!["Netflix.exe".to_string()],
            detect_window_titles: true,
        }
    }
}

impl Config {
    pub fn load_or_create() -> io::Result<(Self, PathBuf)> {
        let path = config_path();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let config = Self::default();
            fs::write(&path, to_pretty_toml(&config)?)?;
            return Ok((config, path));
        }

        let raw = fs::read_to_string(&path)?;
        let mut config = toml::from_str::<Self>(&raw).unwrap_or_else(|_| Self::default());
        if config.normalize() {
            config.save(&path)?;
        }
        Ok((config, path))
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        fs::write(path, to_pretty_toml(self)?)
    }

    fn normalize(&mut self) -> bool {
        let before = self.process_names.clone();
        self.process_names.retain(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("netflix")
        });
        if self.process_names.is_empty() {
            self.process_names.push("Netflix.exe".to_string());
        }
        before != self.process_names
    }
}

fn to_pretty_toml(config: &Config) -> io::Result<String> {
    toml::to_string_pretty(config).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn config_path() -> PathBuf {
    let base = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join("HDRNetflix").join("config.toml")
}
