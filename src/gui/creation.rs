use eframe::egui::{self, CursorIcon, Id, Order, Pos2, Rect, Sense, Vec2};

use super::board::GridRect;
use super::selection::{GridSelection, cell_at, paint_selection};

/// Shows the full-board drag capture layer while adding a button.
/// Returns the selected grid rectangle once the user releases the mouse.
pub fn show_add_overlay(
    ctx: &egui::Context,
    board_rect: Rect,
    grid_origin: Pos2,
    cell: Vec2,
    grid: [i32; 2],
    selection: &mut Option<GridSelection>,
) -> Option<GridRect> {
    let mut created = None;

    egui::Area::new(Id::new("add_sound_overlay"))
        .order(Order::Foreground)
        .constrain(false)
        .fixed_pos(board_rect.min)
        .show(ctx, |ui| {
            let (_rect, response) = ui.allocate_exact_size(board_rect.size(), Sense::drag());

            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
            }

            let pointer = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().input(|input| input.pointer.latest_pos()));

            if response.drag_started() {
                if let Some(pos) = pointer {
                    *selection = Some(GridSelection::new(cell_at(pos, grid_origin, cell, grid)));
                }
            }

            if response.dragged() {
                if let (Some(active), Some(pos)) = (selection.as_mut(), pointer) {
                    active.update(cell_at(pos, grid_origin, cell, grid));
                }
            }

            if response.drag_stopped() {
                created = selection.take().map(GridSelection::rect);
            }

            if let Some(active) = *selection {
                paint_selection(ui, active.rect(), grid_origin, cell, true);
            }
        });

    created
}
