// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::{Result, anyhow};
use gridboard::{
    audio_engine::AudioEngine,
    config::{self, Config},
    gui::run_gui,
    server::run_ipc_server,
};
use shellexpand::full;
use std::{
    env::args,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = args().collect();
    let mut cli_path = None;

    let default_config_path = config::default_config_path();

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("Usage: gridboard [path/to/config.jsonc] [-h | --help] [-v | --version]");
                match default_config_path {
                    Some(path) => println!(
                        "If no config path is provided, the app will get it's config from {}.",
                        path.to_string_lossy()
                    ),
                    None => println!(
                        "Failed to resolve the default config path. You must provide one as an argument.",
                    ),
                }
                return Ok(());
            }
            "-v" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ if !arg.starts_with('-') => {
                cli_path = Some(arg.as_str());
            }
            other => anyhow::bail!("Unknown argument: {}", other),
        }
    }

    let config = Config::from_jsonc(&match cli_path {
        Some(path) => PathBuf::from(full(path).unwrap().as_ref()),
        None => {
            let path = default_config_path.ok_or(anyhow!(
                "Failed to resolve the default config path. You must provide one as an argument.",
            ))?;
            if !fs::exists(path.clone())? {
                if let Some(parent) = path.as_path().parent() {
                    fs::create_dir_all(parent)?;
                }
                Config::default().to_jsonc(path.as_path())?
            }
            path
        }
    })?;

    let audio_engine = Arc::new(Mutex::new(AudioEngine::from_config(&config)?));

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
