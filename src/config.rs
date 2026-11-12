use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Button {
    pub name: String,
    pub sound: PathBuf,
    pub image: Option<PathBuf>,
    pub position: [i32; 2],
    pub size: [i32; 2],
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GUIConfig {
    pub window_width: f32,
    pub window_height: f32,
    pub grid_size: [i32; 2],
    pub buttons: Vec<Button>,
}

pub fn default_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gridboard").map(|p| p.config_dir().join("config.jsonc"))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub channels: u16,
    pub sample_rate: u32,
    pub buffer_size: u32,

    pub gui: Option<GUIConfig>,
}

impl Config {
    pub fn from_jsonc(path: &Path) -> Result<Self> {
        Ok(serde_json::from_value(jsonc::parse_jsonc(
            &fs::read_to_string(path)?,
        )?)?)
    }
    pub fn to_jsonc(&self, path: &Path) -> Result<()> {
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let message = format!(
            "Welcome to gridboard! This button does nothing because it isn't tied to an existing sound. Edit {} or enter the edit mode in the top left corner.",
            default_config_path()
                .expect(
                    "Failed to resolve the default config path. You must provide one as an argument."
                )
                .to_string_lossy()
        );
        Self {
            channels: 2,
            sample_rate: 48000,
            buffer_size: 4096,
            gui: Some(GUIConfig {
                window_width: 600.0,
                window_height: 400.0,
                grid_size: [8, 6],
                buttons: vec![Button {
                    name: message,
                    sound: PathBuf::from("/path/to/your/sound.mp3"),
                    image: None,
                    position: [2, 2],
                    size: [4, 2],
                }],
            }),
        }
    }
}
