use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Button {
    pub name: String,
    pub sound: PathBuf,
    // pub image: Option<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub channels: u16,
    pub sample_rate: u32,
    pub buffer_size: u32,

    pub window_width: f32,
    pub window_height: f32,

    pub buttons: Vec<Vec<Button>>,
}

impl Config {
    pub fn from_jsonc(path: &Path) -> Self {
        serde_json::from_value(
            jsonc::parse_jsonc(&fs::read_to_string(path).expect("Failed to read config file"))
                .expect("Failed to parse config"),
        )
        .expect("Failed to deserealize paresd config")
    }
}
