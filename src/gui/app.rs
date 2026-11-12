use crate::{
    audio::AudioEngine,
    config::{Config, GUIConfig},
};
use anyhow::Result;
use eframe::egui::{
    self, Align, CornerRadius, CursorIcon, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use super::board::{Board, GridRect};
use super::creation::show_add_overlay;
use super::editor::show_button_editor;
use super::paint::{
    RADIUS, inset_button_rect, paint_button_background, paint_button_label, paint_grid,
    paint_resize_corner,
};
use super::selection::{GridSelection, cell_at, selection_preview_area};
use super::texture::ButtonImage;

const SPACING: f32 = 5.0;
const TOOLBAR_HEIGHT: f32 = 40.0;

struct State {
    config_path: PathBuf,
    config: Config,
    board: Board,
    images: HashMap<u64, ButtonImage>,
    edit_mode: bool,
    add_mode: bool,
    add_selection: Option<GridSelection>,
    selection_drag: Option<GridSelection>,
    selected: BTreeSet<u64>,
    group_drag: Option<Vec<(u64, GridRect)>>,
    audio_engine: Arc<Mutex<AudioEngine>>,
}

impl State {
    fn new(config_path: PathBuf, config: Config, audio_engine: Arc<Mutex<AudioEngine>>) -> Self {
        let gui_config = config.gui.clone().unwrap_or_default();

        Self {
            config_path,
            config,
            board: Board::from_config(gui_config),
            images: HashMap::new(),
            edit_mode: false,
            add_mode: false,
            add_selection: None,
            selection_drag: None,
            selected: BTreeSet::new(),
            group_drag: None,
            audio_engine,
        }
    }

    fn save_config(&mut self) {
        let window_width = self
            .config
            .gui
            .as_ref()
            .map(|gui| gui.window_width)
            .unwrap_or(600.0);
        let window_height = self
            .config
            .gui
            .as_ref()
            .map(|gui| gui.window_height)
            .unwrap_or(400.0);

        self.config.gui = Some(GUIConfig {
            window_width,
            window_height,
            grid_size: self.board.grid,
            buttons: self.board.to_config_buttons(),
        });

        if let Err(error) = self.config.to_jsonc(&self.config_path) {
            eprintln!(
                "Failed to save config {}: {error}",
                self.config_path.display()
            );
        }
    }

    fn cancel_add(&mut self) {
        self.add_mode = false;
        self.add_selection = None;
    }

    fn clear_selection(&mut self) {
        self.selection_drag = None;
        self.selected.clear();
        self.group_drag = None;
    }

    fn grid_size_controls(&mut self, ui: &mut egui::Ui) {
        let button_size = Vec2::new(58.0, 24.0);
        let mut changed = false;

        if ui
            .add_sized(button_size, egui::Button::new("+top"))
            .on_hover_text("Grow from the top: add a row and move all buttons down")
            .clicked()
        {
            self.board.grow_top();
            changed = true;
        }

        if ui
            .add_sized(button_size, egui::Button::new("+bottom"))
            .on_hover_text("Grow from the bottom: add a row")
            .clicked()
        {
            self.board.grow_bottom();
            changed = true;
        }

        if ui
            .add_sized(button_size, egui::Button::new("+left"))
            .on_hover_text("Grow from the left: add a column and move all buttons right")
            .clicked()
        {
            self.board.grow_left();
            changed = true;
        }

        if ui
            .add_sized(button_size, egui::Button::new("+right"))
            .on_hover_text("Grow from the right: add a column")
            .clicked()
        {
            self.board.grow_right();
            changed = true;
        }

        let shrink_top = ui
            .add_enabled_ui(self.board.can_shrink_top(), |ui| {
                ui.add_sized(button_size, egui::Button::new("-top"))
            })
            .inner
            .on_hover_text("Shrink from the top: remove the top row and move all buttons up")
            .on_disabled_hover_text("Can't shrink from the top: a button is in the top row");
        if shrink_top.clicked() {
            self.board.shrink_top();
            changed = true;
        }

        let shrink_bottom = ui
            .add_enabled_ui(self.board.can_shrink_bottom(), |ui| {
                ui.add_sized(button_size, egui::Button::new("-bottom"))
            })
            .inner
            .on_hover_text("Shrink from the bottom: remove the bottom row")
            .on_disabled_hover_text(
                "Can't shrink from the bottom: a button would extend outside the grid",
            );
        if shrink_bottom.clicked() {
            self.board.shrink_bottom();
            changed = true;
        }

        let shrink_left = ui
            .add_enabled_ui(self.board.can_shrink_left(), |ui| {
                ui.add_sized(button_size, egui::Button::new("-left"))
            })
            .inner
            .on_hover_text("Shrink from the left: remove the left column and move all buttons left")
            .on_disabled_hover_text("Can't shrink from the left: a button is in the left column");
        if shrink_left.clicked() {
            self.board.shrink_left();
            changed = true;
        }

        let shrink_right = ui
            .add_enabled_ui(self.board.can_shrink_right(), |ui| {
                ui.add_sized(button_size, egui::Button::new("-right"))
            })
            .inner
            .on_hover_text("Shrink from the right: remove the rightmost column")
            .on_disabled_hover_text(
                "Can't shrink from the right: a button would extend outside the grid",
            );
        if shrink_right.clicked() {
            self.board.shrink_right();
            changed = true;
        }

        if changed {
            self.board.clear_interaction();
            self.cancel_add();
            self.clear_selection();
        }
    }

    fn handle_selection_drag(
        &mut self,
        ui: &mut egui::Ui,
        board_rect: Rect,
        grid_origin: Pos2,
        cell: Vec2,
    ) {
        let response = ui.interact(
            board_rect,
            egui::Id::new("group_selection_drag"),
            Sense::drag(),
        );

        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }

        let pointer = response
            .interact_pointer_pos()
            .or_else(|| ui.ctx().input(|input| input.pointer.latest_pos()));

        if response.drag_started() {
            if let Some(pos) = pointer {
                self.selection_drag = Some(GridSelection::new(cell_at(
                    pos,
                    grid_origin,
                    cell,
                    self.board.grid,
                )));
            }
        }

        if response.dragged() {
            if let (Some(active), Some(pos)) = (self.selection_drag.as_mut(), pointer) {
                active.update(cell_at(pos, grid_origin, cell, self.board.grid));
            }
        }

        if response.drag_stopped() {
            if let Some(active) = self.selection_drag.take() {
                let rect = active.rect();
                self.selected = self
                    .board
                    .buttons
                    .iter()
                    .filter(|button| button.rect.intersects(rect))
                    .map(|button| button.id)
                    .collect();
                self.group_drag = None;
            }
        }

        if let Some(active) = self.selection_drag {
            selection_preview_area(ui.ctx(), board_rect, active, grid_origin, cell);
        }
    }

    fn handle_group_move(&mut self, response: &egui::Response, cell: Vec2) {
        if response.drag_started() {
            self.group_drag = Some(self.board.group_start(&self.selected));
            self.selection_drag = None;
        }

        if response.dragged() {
            if let Some(start) = &self.group_drag {
                let delta = response.total_drag_delta().unwrap_or_default();
                let dx = (delta.x / cell.x.max(1.0)).round() as i32;
                let dy = (delta.y / cell.y.max(1.0)).round() as i32;
                self.board.move_group(start, dx, dy);
            }
        }

        if response.drag_stopped() {
            self.group_drag = None;
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, toolbar_rect: Rect) {
        let mut volume = self
            .audio_engine
            .lock()
            .expect("Failed to acquire audio engine mutex")
            .get_volume();

        let toolbar_content = toolbar_rect.shrink2(Vec2::new(SPACING, 4.0));
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(toolbar_content)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = SPACING;

                let toggle = ui
                    .add_sized(
                        [120.0, 24.0],
                        egui::Checkbox::new(&mut self.edit_mode, "Edit mode"),
                    )
                    .on_hover_text(
                        "Edit buttons: move, resize, change title, sound, image, or delete. \
                         Drag empty grid space to select a group; Esc clears the selection.",
                    );
                if toggle.changed() {
                    self.board.clear_interaction();
                    if !self.edit_mode {
                        self.cancel_add();
                        self.clear_selection();
                        self.save_config();
                    }
                }

                if self.edit_mode {
                    let (label, tooltip) = if self.add_mode {
                        ("Cancel add", "Cancel adding a button")
                    } else {
                        (
                            "Add sound",
                            "Click, then drag over grid cells. Esc cancels.",
                        )
                    };
                    let add = ui
                        .add_sized(
                            [110.0, 24.0],
                            egui::Button::new(label).selected(self.add_mode),
                        )
                        .on_hover_text(tooltip);
                    if add.clicked() {
                        if self.add_mode {
                            self.cancel_add();
                        } else {
                            self.add_mode = true;
                            self.add_selection = None;
                            self.clear_selection();
                            self.board.clear_interaction();
                        }
                    }

                    self.grid_size_controls(ui);
                } else {
                    if ui
                        .add_sized([80.0, 24.0], egui::Button::new("stop all"))
                        .clicked()
                    {
                        self.audio_engine
                            .lock()
                            .expect("Failed to acquire audio engine mutex")
                            .stop_all();
                    }

                    if ui
                        .add_sized([105.0, 24.0], egui::Button::new("reset volume"))
                        .clicked()
                    {
                        self.audio_engine
                            .lock()
                            .expect("Failed to acquire audio engine mutex")
                            .set_volume(1.0);
                        volume = 1.0;
                    }

                    let value_width = 56.0;
                    let slider_width =
                        (ui.available_width() - value_width - ui.spacing().item_spacing.x)
                            .max(60.0);
                    let old_slider_width = ui.spacing().slider_width;
                    ui.spacing_mut().slider_width = slider_width;

                    let slider = ui.add(
                        egui::Slider::new(&mut volume, 0.0..=3.0)
                            .show_value(false)
                            .step_by(0.01),
                    );

                    ui.spacing_mut().slider_width = old_slider_width;

                    let value = ui.add_sized(
                        [value_width, 24.0],
                        egui::DragValue::new(&mut volume)
                            .range(0.0..=3.0)
                            .speed(0.01)
                            .fixed_decimals(2),
                    );

                    if slider.changed() || value.changed() {
                        self.audio_engine
                            .lock()
                            .expect("Failed to acquire audio engine mutex")
                            .set_volume(volume);
                    }
                }
            },
        );
    }
}

impl eframe::App for State {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let bounds = ui.max_rect();
        let toolbar_rect =
            Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), TOOLBAR_HEIGHT));
        let board_rect =
            Rect::from_min_max(Pos2::new(bounds.min.x, toolbar_rect.max.y), bounds.max);

        let radius = CornerRadius::same(RADIUS as u8);

        // Toolbar. The board starts directly below it and uses all remaining space.
        ui.painter()
            .rect_filled(toolbar_rect, radius, ui.visuals().faint_bg_color);
        self.toolbar(ui, toolbar_rect);

        // Keep the texture cache in sync with the current image paths.
        for button in &self.board.buttons {
            self.images
                .entry(button.id)
                .or_default()
                .sync(ui.ctx(), button.image.as_deref());
        }

        // The grid fills the whole area below the toolbar. Cells may be
        // rectangular, so width and height are computed independently.
        let cell = Vec2::new(
            (board_rect.width() / self.board.grid[0] as f32).max(1.0),
            (board_rect.height() / self.board.grid[1] as f32).max(1.0),
        );
        let grid_origin = board_rect.min;
        let grid_rect = board_rect;

        paint_grid(ui, grid_rect, cell, self.board.grid, self.edit_mode);

        if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.add_mode {
                self.cancel_add();
            } else {
                self.clear_selection();
            }
        }

        if self.edit_mode && !self.add_mode {
            self.handle_selection_drag(ui, board_rect, grid_origin, cell);
        }

        if self.add_mode
            && let Some(rect) = show_add_overlay(
                ui.ctx(),
                board_rect,
                grid_origin,
                cell,
                self.board.grid,
                &mut self.add_selection,
            )
        {
            self.board.add_button(rect);
            self.add_mode = false;
            self.board.clear_interaction();
            self.clear_selection();
        }

        // Iterate over stable ids so deleting a button mid-frame cannot leave
        // us with stale indices.
        let button_ids: Vec<u64> = self.board.buttons.iter().map(|button| button.id).collect();

        // Each button is its own Area, so overlapping buttons are drawn in
        // independent layers and get click-to-front behavior for free.
        for button_id in button_ids {
            let Some(index) = self
                .board
                .buttons
                .iter()
                .position(|button| button.id == button_id)
            else {
                continue;
            };

            let mut button = self.board.buttons[index].clone();
            let cell_rect = button.rect.to_pixels(grid_origin, cell);
            let rect = inset_button_rect(cell_rect);
            let image = self.images.get(&button_id).cloned().unwrap_or_default();
            let edit_mode = self.edit_mode;
            let group_selected =
                edit_mode && self.selected.len() > 1 && self.selected.contains(&button_id);

            let area = egui::Area::new(egui::Id::new(("sound_button", button_id)))
                .order(egui::Order::Middle)
                .constrain(false)
                .fixed_pos(rect.min)
                .show(ui.ctx(), |ui| {
                    if !edit_mode {
                        let (body_rect, response) =
                            ui.allocate_exact_size(rect.size(), Sense::click());
                        let has_image = image.texture().is_some();

                        paint_button_background(ui, body_rect, Some(&image), &response);
                        paint_button_label(ui, body_rect, &button, &response, has_image);

                        if response.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }

                        return (Some(response), None, None, false);
                    }

                    if group_selected {
                        let (body_rect, response) =
                            ui.allocate_exact_size(rect.size(), Sense::click_and_drag());
                        let has_image = image.texture().is_some();
                        paint_button_background(ui, body_rect, Some(&image), &response);

                        let selection = ui.visuals().selection;
                        let radius = CornerRadius::same(RADIUS as u8);
                        ui.painter()
                            .rect_filled(body_rect, radius, selection.bg_fill);
                        paint_button_label(ui, body_rect, &button, &response, has_image);
                        ui.painter().rect_stroke(
                            body_rect,
                            radius,
                            Stroke::new(2.0, selection.stroke.color),
                            StrokeKind::Inside,
                        );

                        if response.hovered() || response.dragged() {
                            ui.ctx().set_cursor_icon(if response.dragged() {
                                CursorIcon::Grabbing
                            } else {
                                CursorIcon::Grab
                            });
                        }

                        return (None, Some(response), None, false);
                    }

                    let (body_rect, body_response) =
                        ui.allocate_exact_size(rect.size(), Sense::hover());
                    paint_button_background(ui, body_rect, Some(&image), &body_response);

                    let editor = show_button_editor(ui, body_rect, &mut button, &image);

                    // Reuse egui's standard bottom-right resize affordance and
                    // its style-driven size / grab radius, but drive the actual
                    // size ourselves so it can snap to the grid.
                    let corner_size = Vec2::splat(ui.visuals().resize_corner_size);
                    let corner_rect = Rect::from_min_size(body_rect.max - corner_size, corner_size);

                    let grab_radius = ui.style().interaction.resize_grab_radius_corner;
                    let handle_rect = Rect::from_center_size(
                        body_rect.max,
                        Vec2::splat(2.0 * grab_radius.max(corner_size.x * 0.5)),
                    );
                    let handle = ui.interact(
                        handle_rect,
                        egui::Id::new(("resize", button_id)),
                        Sense::drag(),
                    );

                    if handle.hovered() || handle.dragged() {
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
                    }
                    paint_resize_corner(ui, corner_rect, &handle);

                    (None, editor.drag, Some(handle), editor.deleted)
                });

            let (body_response, drag_response, resize_response, deleted) = area.inner;

            if deleted {
                self.board.delete(button_id);
                self.images.remove(&button_id);
                self.selected.remove(&button_id);
                self.group_drag = None;
                continue;
            }

            if group_selected {
                if let Some(response) = drag_response {
                    self.handle_group_move(&response, cell);
                }
                continue;
            }

            if let Some(response) = body_response {
                if response.clicked() {
                    let mut engine = self
                        .audio_engine
                        .lock()
                        .expect("Failed to acquire audio engine mutex");
                    engine.play(&button.sound);
                }
            }

            if edit_mode {
                let target = &mut self.board.buttons[index];
                target.name = button.name;
                target.sound = button.sound;
                target.image = button.image;
            }

            if let Some(response) = drag_response {
                self.board.handle_move(index, &response, cell);
            }
            if let Some(response) = resize_response {
                self.board.handle_resize(index, &response, cell);
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.07, 0.07, 0.08, 1.0]
    }
}

pub fn run_gui(
    config_path: PathBuf,
    config: Config,
    audio_engine: Arc<Mutex<AudioEngine>>,
) -> Result<()> {
    let gui_config = config
        .gui
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("GUI config is missing"))?;
    let size = [gui_config.window_width, gui_config.window_height];

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
        Box::new(move |_cc| Ok(Box::new(State::new(config_path, config, audio_engine)))),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}
