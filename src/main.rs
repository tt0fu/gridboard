#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio_engine;
mod config;
mod load_audio;

use std::{env::args, path::PathBuf};

use crate::{app::MyApp, config::Config};
use directories::ProjectDirs;
use eframe::{NativeOptions, Result, egui, run_native};
use egui::ViewportBuilder;
use egui_extras::install_image_loaders;
use shellexpand::full;

fn main() -> Result {
    let args: Vec<String> = args().collect();
    let mut config_path_arg = None;
    let mut show_help = false;
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--help" | "-h" => {
                show_help = true;
                break;
            }
            _ if !arg.starts_with('-') => {
                config_path_arg = Some(arg.as_str());
            }
            _ => {}
        }
    }

    if show_help {
        println!("Usage: gridboard [path/to/config.jsonc] [--help | -h]");
        return Ok(());
    }

    let config = Config::from_jsonc(&match config_path_arg {
        Some(path) => PathBuf::from(full(path).unwrap().as_ref()),
        None => ProjectDirs::from("", "", "gridboard")
            .expect("Failed to resolve standard config directory")
            .config_dir()
            .join("config.jsonc"),
    });

    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    run_native(
        "Gridboard",
        options,
        Box::new(|cc| {
            install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(MyApp::new(config)))
        }),
    )
}
