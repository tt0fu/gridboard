// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Result;
use directories::ProjectDirs;
use gridboard::{audio_engine::AudioEngine, config::Config, gui::run_gui, server::run_ipc_server};
use shellexpand::full;
use std::{
    env::args,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = args().collect();
    let mut config_path = None;
    let mut show_help = false;
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                show_help = true;
                break;
            }
            _ if !arg.starts_with('-') => {
                config_path = Some(arg.as_str());
            }
            other => anyhow::bail!("Unknown argument: {}", other),
        }
    }

    if show_help {
        println!("Usage: gridboard [path/to/config.jsonc] [-h | --help]");
        return Ok(());
    }

    let config = Config::from_jsonc(&match config_path {
        Some(path) => PathBuf::from(full(path).unwrap().as_ref()),
        None => ProjectDirs::from("", "", "gridboard")
            .expect("Failed to resolve standard config directory")
            .config_dir()
            .join("config.jsonc"),
    });

    let audio_engine = Arc::new(Mutex::new(AudioEngine::from_config(&config)));

    match config.gui {
        Some(gui_config) => {
            let engine = audio_engine.clone();
            tokio::spawn(async move {
                if let Err(e) = run_ipc_server(engine).await {
                    eprintln!("IPC server error: {}", e);
                }
            });
            run_gui(gui_config, audio_engine.clone())?;
        }
        None => run_ipc_server(audio_engine).await?,
    }
    Ok(())
}
