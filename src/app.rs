use crate::{audio_engine::AudioEngine, config::Config};
use eframe::egui;
use egui::{Button, vec2};

pub struct MyApp {
    config: Config,
    audio_engine: AudioEngine,
}

impl MyApp {
    pub fn new(config: Config) -> Self {
        Self {
            config: config.clone(),
            audio_engine: AudioEngine::from_parameters(
                config.channels,
                config.sample_rate,
                config.buffer_size,
            ),
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let available = ui.available_size();
        let spacing = ui.spacing().item_spacing;

        let rows = (self.config.buttons.len() + 1) as f32;
        let button_height = (available.y - spacing.y * (rows - 1.0)) / rows;

        ui.vertical(|ui| {
            if ui
                .add_sized(vec2(available.x, button_height), Button::new("stop all"))
                .clicked()
            {
                self.audio_engine.stop_all();
            }
            for row in &self.config.buttons {
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
                                Button::new(&button.name),
                            )
                            .clicked()
                        {
                            self.audio_engine.play(&button.sound);
                        }
                    }
                });
            }
        });
    }
}
