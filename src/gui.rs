use crate::{audio_engine::AudioEngine, config::GUIConfig};
use anyhow::Result;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Pos2, Rect, Sense, Vec2,
};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

const RADIUS: f32 = 5.0;
const PADDING: f32 = 5.0;
const SPACING: f32 = 5.0;
const TITLE_HEIGHT: f32 = 30.0;

struct ButtonImage {
    texture: egui::TextureHandle,
    aspect_ratio: f32,
}

struct SoundButton {
    name: String,
    sound: PathBuf,
    image: Option<ButtonImage>,
}

struct State {
    buttons: Vec<Vec<SoundButton>>,
    audio_engine: Arc<Mutex<AudioEngine>>,
}

impl State {
    fn new(
        config: GUIConfig,
        audio_engine: Arc<Mutex<AudioEngine>>,
        context: &egui::Context,
    ) -> Self {
        let buttons = config
            .buttons
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|button| {
                        let image = button.image.and_then(|path| {
                            let bytes = match fs::read(&path) {
                                Ok(bytes) => bytes,
                                Err(err) => {
                                    eprintln!("Failed to load image {}: {err}", path.display());
                                    return None;
                                }
                            };

                            let decoded = match image::load_from_memory(&bytes) {
                                Ok(image) => image.to_rgba8(),
                                Err(err) => {
                                    eprintln!("Failed to decode image {}: {err}", path.display());
                                    return None;
                                }
                            };
                            let (width, height) = decoded.dimensions();
                            if width == 0 || height == 0 {
                                return None;
                            }

                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                [width as usize, height as usize],
                                decoded.as_raw(),
                            );
                            let texture = context.load_texture(
                                format!("preview://{}", path.display()),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );

                            Some(ButtonImage {
                                texture,
                                aspect_ratio: width as f32 / height as f32,
                            })
                        });
                        SoundButton {
                            name: button.name,
                            sound: button.sound,
                            image,
                        }
                    })
                    .collect()
            })
            .collect();
        Self {
            buttons,
            audio_engine,
        }
    }

    fn paint_sound_button(&self, ui: &egui::Ui, rect: Rect, id: Id, button: &SoundButton) {
        let response = ui.interact(rect, id, Sense::click());
        let widget_visuals = *ui.style().interact(&response);
        let painter = ui.painter();
        let radius = CornerRadius::same(RADIUS as u8);

        if let Some(image) = &button.image {
            egui::Image::from_texture(&image.texture)
                .uv(cover_uv(image.aspect_ratio, rect))
                .fit_to_exact_size(rect.size())
                .corner_radius(radius)
                .paint_at(ui, rect);
        } else {
            painter.rect_filled(rect, radius, Color32::from_gray(38));
        }

        painter.rect_stroke(
            rect.expand(widget_visuals.expansion),
            radius,
            widget_visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let text_color = widget_visuals.fg_stroke.color;
        if button.image.is_some() {
            let strip =
                Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - TITLE_HEIGHT), rect.max);
            let strip_radius = CornerRadius {
                nw: 0,
                ne: 0,
                sw: RADIUS as u8,
                se: RADIUS as u8,
            };

            painter.rect_filled(strip, strip_radius, Color32::from_black_alpha(153));
            painter.with_clip_rect(strip).text(
                strip.center(),
                Align2::CENTER_CENTER,
                &button.name,
                FontId::proportional(16.0),
                text_color,
            );
        } else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                &button.name,
                FontId::proportional(16.0),
                text_color,
            );
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        if response.clicked() {
            let mut engine = self
                .audio_engine
                .lock()
                .expect("Failed to aquire audio engine mutex");
            engine.play(&button.sound);
        }
    }
}

fn cover_uv(image_aspect_ratio: f32, rect: Rect) -> Rect {
    let target_aspect_ratio = rect.width() / rect.height();

    if target_aspect_ratio > image_aspect_ratio {
        let visible_height = image_aspect_ratio / target_aspect_ratio;
        let inset = (1.0 - visible_height) / 2.0;
        Rect::from_min_max(Pos2::new(0.0, inset), Pos2::new(1.0, 1.0 - inset))
    } else {
        let visible_width = target_aspect_ratio / image_aspect_ratio;
        let inset = (1.0 - visible_width) / 2.0;
        Rect::from_min_max(Pos2::new(inset, 0.0), Pos2::new(1.0 - inset, 1.0))
    }
}

impl eframe::App for State {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut volume = self
            .audio_engine
            .lock()
            .expect("Failed to aquire audio engine mutex")
            .get_volume();

        let bounds = ui.max_rect().shrink(PADDING);
        let row_count = self.buttons.len();
        let portions = 1 + 3 * row_count;
        let content_height = bounds.height() - SPACING * row_count as f32;
        let unit = content_height / portions as f32;
        let toolbar_height = unit;
        let row_height = 3.0 * unit;

        let toolbar = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), toolbar_height));

        let volume_label_width = 64.0;
        let toolbar_spacing = SPACING * 3.0;
        let toolbar_units = (toolbar.width() - volume_label_width - toolbar_spacing) / 7.0;

        let stop_rect = Rect::from_min_size(
            toolbar.min,
            Vec2::new(toolbar_units * 2.0, toolbar.height()),
        );
        let reset_rect = Rect::from_min_size(
            Pos2::new(stop_rect.max.x + SPACING, toolbar.min.y),
            Vec2::new(toolbar_units * 2.0, toolbar.height()),
        );
        let slider_rect = Rect::from_min_size(
            Pos2::new(reset_rect.max.x + SPACING, toolbar.min.y),
            Vec2::new(toolbar_units * 3.0, toolbar.height()),
        );
        let volume_rect = Rect::from_min_size(
            Pos2::new(slider_rect.max.x + SPACING, toolbar.min.y),
            Vec2::new(volume_label_width, toolbar.height()),
        );

        let radius = CornerRadius::same(RADIUS as u8);
        let button_fill = Color32::from_gray(38);

        let stop = ui.put(
            stop_rect,
            egui::Button::new("stop all")
                .corner_radius(radius)
                .fill(button_fill),
        );
        if stop.clicked() {
            self.audio_engine
                .lock()
                .expect("Failed to aquire audio engine mutex")
                .stop_all();
        }

        let reset = ui.put(
            reset_rect,
            egui::Button::new("reset volume")
                .corner_radius(radius)
                .fill(button_fill),
        );
        if reset.clicked() {
            self.audio_engine
                .lock()
                .expect("Failed to aquire audio engine mutex")
                .set_volume(1.0);
            volume = 1.0;
        }

        let previous_slider_width = ui.spacing().slider_width;
        ui.spacing_mut().slider_width = slider_rect.width();
        let slider = ui.put(
            slider_rect,
            egui::Slider::new(&mut volume, 0.0..=3.0)
                .show_value(false)
                .step_by(0.01),
        );
        ui.spacing_mut().slider_width = previous_slider_width;

        let value = ui.put(
            volume_rect,
            egui::DragValue::new(&mut volume)
                .range(0.0..=3.0)
                .speed(0.01)
                .fixed_decimals(2),
        );

        if slider.changed() || value.changed() {
            self.audio_engine
                .lock()
                .expect("Failed to aquire audio engine mutex")
                .set_volume(volume);
        }

        for (row_index, row) in self.buttons.iter().enumerate() {
            if row.is_empty() {
                continue;
            }

            let top =
                bounds.top() + toolbar_height + SPACING + row_index as f32 * (row_height + SPACING);
            let column_count = row.len();
            let column_width = (bounds.width() - SPACING * (column_count.saturating_sub(1)) as f32)
                / column_count as f32;

            for (column_index, button) in row.iter().enumerate() {
                let left = bounds.left() + column_index as f32 * (column_width + SPACING);
                let rect =
                    Rect::from_min_size(Pos2::new(left, top), Vec2::new(column_width, row_height));

                self.paint_sound_button(
                    ui,
                    rect,
                    Id::new(("sound_button", row_index, column_index)),
                    button,
                );
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

pub fn run_gui(config: GUIConfig, audio_engine: Arc<Mutex<AudioEngine>>) -> Result<()> {
    let size = [config.window_width, config.window_height];

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_title("gridboard")
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "gridboard",
        options,
        Box::new(move |cc| Ok(Box::new(State::new(config, audio_engine, &cc.egui_ctx)))),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}
