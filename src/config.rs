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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShortcutsConfig {
    pub select: String,
    pub arrow: String,
    pub rectangle: String,
    pub circle: String,
    pub rounded_rect: String,
    pub line: String,
    pub pencil: String,
    pub highlight: String,
    pub spotlight: String,
    pub text: String,
    pub pixelate: String,
    pub step_marker: String,
    pub eyedropper: String,
    pub measurement: String,
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            select: "v".into(),
            arrow: "a".into(),
            rectangle: "r".into(),
            circle: "c".into(),
            rounded_rect: "o".into(),
            line: "l".into(),
            pencil: "p".into(),
            highlight: "h".into(),
            spotlight: "f".into(),
            text: "t".into(),
            pixelate: "b".into(),
            step_marker: "n".into(),
            eyedropper: "i".into(),
            measurement: "m".into(),
        }
    }
}

impl ShortcutsConfig {
    /// Ordered list of (label, current key) for UI display.
    pub fn entries(&self) -> Vec<(&'static str, &str)> {
        vec![
            ("Select", &self.select),
            ("Arrow", &self.arrow),
            ("Rectangle", &self.rectangle),
            ("Circle", &self.circle),
            ("Rounded Rect", &self.rounded_rect),
            ("Line", &self.line),
            ("Pencil", &self.pencil),
            ("Highlight", &self.highlight),
            ("Spotlight", &self.spotlight),
            ("Text", &self.text),
            ("Pixelate", &self.pixelate),
            ("Step Marker", &self.step_marker),
            ("Eyedropper", &self.eyedropper),
            ("Measurement", &self.measurement),
        ]
    }

    /// Set shortcut by index (0-13).
    pub fn set_by_index(&mut self, index: usize, key: String) {
        match index {
            0 => self.select = key,
            1 => self.arrow = key,
            2 => self.rectangle = key,
            3 => self.circle = key,
            4 => self.rounded_rect = key,
            5 => self.line = key,
            6 => self.pencil = key,
            7 => self.highlight = key,
            8 => self.spotlight = key,
            9 => self.text = key,
            10 => self.pixelate = key,
            11 => self.step_marker = key,
            12 => self.eyedropper = key,
            13 => self.measurement = key,
            _ => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub hotkey: HotkeyConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
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
            shortcuts: ShortcutsConfig::default(),
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
