use crate::geometry::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub default_color: String,
    pub default_thickness: f32,
    pub save_directory: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub capture: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub hotkey: HotkeyConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                default_color: "red".to_string(),
                default_thickness: 3.0,
                save_directory: String::new(),
            },
            hotkey: HotkeyConfig {
                capture: "Ctrl+Shift+S".to_string(),
            },
        }
    }
}

impl Config {
    /// Returns the config file path: `<config_dir>/hydroshot/config.toml`
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("hydroshot").join("config.toml"))
    }

    /// Load config from disk; creates default if missing; falls back to defaults on parse error.
    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => {
                tracing::warn!("Could not determine config directory, using defaults");
                return Self::default();
            }
        };

        if !path.exists() {
            tracing::info!("Config file not found, creating with defaults");
            let cfg = Self::default();
            if let Err(e) = cfg.save() {
                tracing::warn!("Failed to write default config: {e}");
            }
            return cfg;
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!("Failed to parse config, using defaults: {e}");
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read config file, using defaults: {e}");
                Self::default()
            }
        }
    }

    /// Write current config to disk as TOML.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let contents =
            toml::to_string(self).map_err(|e| format!("Failed to serialize config: {e}"))?;

        std::fs::write(&path, contents).map_err(|e| format!("Failed to write config file: {e}"))?;

        Ok(())
    }

    /// Parse the `default_color` string into a `Color`, falling back to red.
    pub fn default_color(&self) -> Color {
        match self.general.default_color.as_str() {
            "red" => Color::red(),
            "blue" => Color::blue(),
            "green" => Color::green(),
            "yellow" => Color::yellow(),
            "white" => Color::white(),
            "mauve" => Color::mauve(),
            "peach" => Color::peach(),
            "teal" => Color::teal(),
            "sky" => Color::sky(),
            "lavender" => Color::lavender(),
            other => {
                tracing::warn!("Unknown color '{other}', falling back to red");
                Color::red()
            }
        }
    }

    /// Clamp thickness to the range 1.0..=20.0.
    pub fn clamped_thickness(&self) -> f32 {
        self.general.default_thickness.clamp(1.0, 20.0)
    }

    /// Returns `Some(path)` if save_directory is non-empty and exists, else `None`.
    pub fn save_directory(&self) -> Option<PathBuf> {
        if self.general.save_directory.is_empty() {
            return None;
        }
        let path = PathBuf::from(&self.general.save_directory);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
}
