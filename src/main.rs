#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio_engine;
mod config;
mod load_audio;

use std::{env::args, path::Path};

use crate::{app::MyApp, config::Config};
use eframe::{NativeOptions, Result, egui, run_native};
use egui::ViewportBuilder;
use egui_extras::install_image_loaders;

fn main() -> Result {
    if args().any(|a| a == "--help" || a == "-h") {
        println!(
            "{}",
            "Usage: gridboard <path/to/config.jsonc> [--help | -h]"
        );
        return Ok(());
    }
    let config_path = args()
        .skip(1)
        .find(|a| !a.starts_with("-"))
        .expect("No config file path provided in the arguments");
    let config = Config::from_jsonc(&Path::new(&config_path));

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
