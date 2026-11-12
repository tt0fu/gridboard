use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:9988";

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Play { path: PathBuf },
    StopAll,
    GetVolume,
    SetVolume { volume: f32 },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok,
    Error(String),
    Volume(f32),
}
