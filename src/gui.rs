use std::sync::{Arc, Mutex};

use crate::{
    audio_engine::AudioEngine,
    config::{Button, GUIConfig},
};
use eframe::{Error, NativeOptions, egui, run_native};
use egui::{Slider, ViewportBuilder, vec2};
use egui_extras::install_image_loaders;

pub struct MyApp {
    buttons: Vec<Vec<Button>>,
    audio_engine: Arc<Mutex<AudioEngine>>,
}

impl MyApp {
    pub fn new(buttons: Vec<Vec<Button>>, audio_engine: Arc<Mutex<AudioEngine>>) -> Self {
        Self {
            buttons,
            audio_engine,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let available = ui.available_size();
        let spacing = ui.spacing().item_spacing;

        let rows = (self.buttons.len() + 1) as f32;
        let button_height = (available.y - spacing.y * (rows - 1.0)) / rows;

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let cols = 2.0;
                let button_width = (available.x - spacing.x * (cols - 1.0)) / cols;
                let button_size = vec2(button_width, button_height);
                if ui
                    .add_sized(button_size, egui::Button::new("stop all"))
                    .clicked()
                {
                    self.audio_engine
                        .lock()
                        .expect("Failed to aquire audio engine mutex")
                        .stop_all();
                }
                ui.spacing_mut().slider_width = button_width - 50.0 - spacing.x;
                if ui
                    .add(Slider::from_get_set(0.0..=3.0, |v: Option<f64>| {
                        if let Some(v) = v {
                            self.audio_engine
                                .lock()
                                .expect("Failed to aquire audio engine mutex")
                                .set_volume(v as f32);
                        }
                        self.audio_engine
                            .lock()
                            .expect("Failed to aquire audio engine mutex")
                            .get_volume() as f64
                    }))
                    .double_clicked()
                {
                    self.audio_engine
                        .lock()
                        .expect("Failed to aquire audio engine mutex")
                        .set_volume(1.0);
                }
            });
            for row in &self.buttons {
                ui.horizontal(|ui| {
                    let cols = row.len() as f32;
                    let button_width = (available.x - spacing.x * (cols - 1.0)) / cols;
                    let button_size = vec2(button_width, button_height);
                    for button in row {
                        if ui
                            .add_sized(
                                button_size,
                                // match &button.image {
                                //     Some(path) => Button::new(Image::from_bytes(
                                //         format!("bytes://{}", path.to_string_lossy()),
                                //         read(path).expect("Failed to open image file"),
                                //     )),
                                //     None => Button::new(&button.name),
                                // },
                                egui::Button::new(&button.name),
                            )
                            .clicked()
                        {
                            self.audio_engine
                                .lock()
                                .expect("Failed to aquire audio engine mutex")
                                .play(&button.sound)
                                .expect("Failed to play a sound");
                        }
                    }
                });
            }
        });
    }
}

pub fn run_gui(config: GUIConfig, audio_engine: Arc<Mutex<AudioEngine>>) -> Result<(), Error> {
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(vec2(config.window_width, config.window_height)),
        ..Default::default()
    };
    run_native(
        "Gridboard",
        options,
        Box::new(|cc| {
            install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(MyApp::new(config.buttons, audio_engine)))
        }),
    )
}
