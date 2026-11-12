use eframe::egui::{self, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::board::GridRect;
use super::paint::{RADIUS, inset_button_rect};

#[derive(Clone, Copy)]
pub struct GridSelection {
    start: [i32; 2],
    current: [i32; 2],
}

impl GridSelection {
    pub fn new(cell: [i32; 2]) -> Self {
        Self {
            start: cell,
            current: cell,
        }
    }

    pub fn update(&mut self, cell: [i32; 2]) {
        self.current = cell;
    }

    pub fn rect(self) -> GridRect {
        let x = self.start[0].min(self.current[0]);
        let y = self.start[1].min(self.current[1]);
        GridRect {
            x,
            y,
            w: (self.start[0] - self.current[0]).abs() + 1,
            h: (self.start[1] - self.current[1]).abs() + 1,
        }
    }
}

pub fn cell_at(pos: Pos2, grid_origin: Pos2, cell: Vec2, grid: [i32; 2]) -> [i32; 2] {
    let x = ((pos.x - grid_origin.x) / cell.x.max(1.0)).floor() as i32;
    let y = ((pos.y - grid_origin.y) / cell.y.max(1.0)).floor() as i32;
    [x.clamp(0, grid[0] - 1), y.clamp(0, grid[1] - 1)]
}

pub fn paint_selection(
    ui: &egui::Ui,
    rect: GridRect,
    grid_origin: Pos2,
    cell: Vec2,
    inset_for_button: bool,
) {
    let mut preview = rect.to_pixels(grid_origin, cell);
    if inset_for_button {
        preview = inset_button_rect(preview);
    }

    let radius = CornerRadius::same(RADIUS as u8);
    let selection = ui.visuals().selection;

    ui.painter().rect_filled(preview, radius, selection.bg_fill);
    ui.painter().rect_stroke(
        preview,
        radius,
        Stroke::new(2.0, selection.stroke.color),
        StrokeKind::Inside,
    );
}

pub fn selection_preview_area(
    ctx: &egui::Context,
    board_rect: Rect,
    selection: GridSelection,
    grid_origin: Pos2,
    cell: Vec2,
) {
    egui::Area::new(egui::Id::new("grid_selection_preview"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .constrain(false)
        .fixed_pos(board_rect.min)
        .show(ctx, |ui| {
            paint_selection(ui, selection.rect(), grid_origin, cell, false);
        });
}
