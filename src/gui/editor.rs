use eframe::egui::{
    self, Align, Color32, CornerRadius, CursorIcon, Layout, Rect, RichText, Sense, StrokeKind,
    TextStyle, UiBuilder, Vec2,
};
use std::path::PathBuf;

use super::board::BoardButton;
use super::paint::RADIUS;
use super::texture::ButtonImage;

pub struct EditorResponse {
    pub drag: Option<egui::Response>,
    pub deleted: bool,
}

pub fn show_button_editor(
    ui: &mut egui::Ui,
    rect: Rect,
    button: &mut BoardButton,
    image: &ButtonImage,
) -> EditorResponse {
    let radius = CornerRadius::same(RADIUS as u8);
    ui.painter()
        .rect_filled(rect, radius, Color32::from_black_alpha(210));
    ui.painter().rect_stroke(
        rect,
        radius,
        ui.visuals().selection.stroke,
        StrokeKind::Inside,
    );

    let mut drag = None;
    let mut deleted = false;
    let inner = rect.shrink(4.0);

    ui.scope_builder(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.set_clip_rect(inner.intersect(ui.clip_rect()));
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 2.0);
            ui.spacing_mut().button_padding = Vec2::new(3.0, 1.0);
            ui.spacing_mut().interact_size.y = 16.0;
            ui.style_mut().override_text_style = Some(TextStyle::Small);

            ui.horizontal(|ui| {
                let delete_width = 18.0;
                let spacing = ui.spacing().item_spacing.x;
                let drag_width = (ui.available_width() - delete_width - spacing).max(10.0);

                let drag_response = ui.add_sized(
                    [drag_width, 16.0],
                    egui::Button::new("move").small().sense(Sense::drag()),
                );
                if drag_response.hovered() || drag_response.dragged() {
                    ui.ctx().set_cursor_icon(if drag_response.dragged() {
                        CursorIcon::Grabbing
                    } else {
                        CursorIcon::Grab
                    });
                }
                drag = Some(drag_response);

                let delete_response =
                    ui.add_sized([delete_width, 16.0], egui::Button::new("x").small());
                if delete_response.on_hover_text("Delete button").clicked() {
                    deleted = true;
                }
            });

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), 16.0],
                        egui::TextEdit::singleline(&mut button.name).hint_text("title"),
                    );

                    let mut sound_text = button.sound.to_string_lossy().into_owned();
                    ui.horizontal(|ui| {
                        let browse_width = 18.0;
                        let spacing = ui.spacing().item_spacing.x;
                        let edit_width = (ui.available_width() - browse_width - spacing).max(12.0);

                        let response = ui.add_sized(
                            [edit_width, 16.0],
                            egui::TextEdit::singleline(&mut sound_text).hint_text("sound path"),
                        );
                        if response.changed() {
                            button.sound = PathBuf::from(&sound_text);
                        }

                        if ui
                            .add_sized([browse_width, 16.0], egui::Button::new("...").small())
                            .on_hover_text("Choose sound file")
                            .clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("Audio", &["wav", "mp3", "flac", "ogg", "m4a", "aac"])
                                .pick_file()
                        {
                            button.sound = path;
                        }
                    });

                    let mut image_text = button
                        .image
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        let button_width = 18.0;
                        let spacing = ui.spacing().item_spacing.x;
                        let edit_width =
                            (ui.available_width() - 2.0 * button_width - 2.0 * spacing).max(12.0);

                        let response = ui.add_sized(
                            [edit_width, 16.0],
                            egui::TextEdit::singleline(&mut image_text)
                                .hint_text("image path (optional)"),
                        );
                        if response.changed() {
                            button.image = if image_text.trim().is_empty() {
                                None
                            } else {
                                Some(PathBuf::from(&image_text))
                            };
                        }

                        if ui
                            .add_sized([button_width, 16.0], egui::Button::new("...").small())
                            .on_hover_text("Choose image file")
                            .clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp"])
                                .pick_file()
                        {
                            button.image = Some(path);
                        }

                        if ui
                            .add_sized([button_width, 16.0], egui::Button::new("x").small())
                            .on_hover_text("Clear image")
                            .clicked()
                        {
                            button.image = None;
                        }
                    });

                    if button.image.is_some()
                        && let Some(error) = image.error()
                    {
                        ui.label(
                            RichText::new(error)
                                .small()
                                .color(ui.visuals().error_fg_color),
                        );
                    }
                });
        },
    );

    EditorResponse { drag, deleted }
}
