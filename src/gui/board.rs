use crate::config::{Button, GUIConfig};
use eframe::egui::{Pos2, Rect, Response, Vec2};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Clone, Copy, Debug)]
pub struct GridRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl GridRect {
    pub fn intersects(self, other: Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    pub fn to_pixels(self, grid_origin: Pos2, cell: Vec2) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                grid_origin.x + self.x as f32 * cell.x,
                grid_origin.y + self.y as f32 * cell.y,
            ),
            Vec2::new(self.w as f32 * cell.x, self.h as f32 * cell.y),
        )
    }
}

fn clamp_rect(rect: &mut GridRect, grid: [i32; 2]) {
    rect.w = rect.w.clamp(1, grid[0]);
    rect.h = rect.h.clamp(1, grid[1]);
    rect.x = rect.x.clamp(0, grid[0] - rect.w);
    rect.y = rect.y.clamp(0, grid[1] - rect.h);
}

#[derive(Clone)]
pub struct BoardButton {
    pub id: u64,
    pub name: String,
    pub sound: PathBuf,
    pub image: Option<PathBuf>,
    pub rect: GridRect,
}

#[derive(Clone, Copy)]
enum Interaction {
    Move { index: usize, start: GridRect },
    Resize { index: usize, start: GridRect },
}

pub struct Board {
    pub grid: [i32; 2],
    pub buttons: Vec<BoardButton>,
    interaction: Option<Interaction>,
    next_id: u64,
}

impl Board {
    pub fn from_config(config: GUIConfig) -> Self {
        let grid = [config.grid_size[0].max(1), config.grid_size[1].max(1)];

        let buttons: Vec<BoardButton> = config
            .buttons
            .into_iter()
            .enumerate()
            .map(|(index, button)| {
                let mut rect = GridRect {
                    x: button.position[0],
                    y: button.position[1],
                    w: button.size[0].max(1),
                    h: button.size[1].max(1),
                };
                // Keep old configs usable even if a button no longer fits the grid.
                clamp_rect(&mut rect, grid);
                BoardButton {
                    id: index as u64,
                    name: button.name,
                    sound: button.sound,
                    image: button.image,
                    rect,
                }
            })
            .collect();

        Self {
            grid,
            next_id: buttons.len() as u64,
            buttons,
            interaction: None,
        }
    }

    pub fn clear_interaction(&mut self) {
        self.interaction = None;
    }

    pub fn delete(&mut self, id: u64) {
        self.buttons.retain(|button| button.id != id);
        self.interaction = None;
    }

    pub fn add_button(&mut self, mut rect: GridRect) {
        clamp_rect(&mut rect, self.grid);
        self.buttons.push(BoardButton {
            id: self.next_id,
            name: "Sound".to_owned(),
            sound: PathBuf::from("sound.mp3"),
            image: None,
            rect,
        });
        self.next_id += 1;
    }

    pub fn grow_top(&mut self) {
        self.grid[1] += 1;
        for button in &mut self.buttons {
            button.rect.y += 1;
        }
    }

    pub fn grow_bottom(&mut self) {
        self.grid[1] += 1;
    }

    pub fn grow_left(&mut self) {
        self.grid[0] += 1;
        for button in &mut self.buttons {
            button.rect.x += 1;
        }
    }

    pub fn grow_right(&mut self) {
        self.grid[0] += 1;
    }

    pub fn can_shrink_top(&self) -> bool {
        self.grid[1] > 1 && self.buttons.iter().all(|button| button.rect.y >= 1)
    }

    pub fn shrink_top(&mut self) {
        if self.can_shrink_top() {
            self.grid[1] -= 1;
            for button in &mut self.buttons {
                button.rect.y -= 1;
            }
        }
    }

    pub fn can_shrink_bottom(&self) -> bool {
        let new_height = self.grid[1] - 1;
        new_height >= 1
            && self
                .buttons
                .iter()
                .all(|button| button.rect.y + button.rect.h <= new_height)
    }

    pub fn shrink_bottom(&mut self) {
        if self.can_shrink_bottom() {
            self.grid[1] -= 1;
        }
    }

    pub fn can_shrink_left(&self) -> bool {
        self.grid[0] > 1 && self.buttons.iter().all(|button| button.rect.x >= 1)
    }

    pub fn shrink_left(&mut self) {
        if self.can_shrink_left() {
            self.grid[0] -= 1;
            for button in &mut self.buttons {
                button.rect.x -= 1;
            }
        }
    }

    pub fn can_shrink_right(&self) -> bool {
        let new_width = self.grid[0] - 1;
        new_width >= 1
            && self
                .buttons
                .iter()
                .all(|button| button.rect.x + button.rect.w <= new_width)
    }

    pub fn shrink_right(&mut self) {
        if self.can_shrink_right() {
            self.grid[0] -= 1;
        }
    }

    pub fn group_start(&self, selected: &BTreeSet<u64>) -> Vec<(u64, GridRect)> {
        self.buttons
            .iter()
            .filter(|button| selected.contains(&button.id))
            .map(|button| (button.id, button.rect))
            .collect()
    }

    pub fn move_group(&mut self, start: &[(u64, GridRect)], dx: i32, dy: i32) {
        let (Some(min_x), Some(max_x), Some(min_y), Some(max_y)) = (
            start.iter().map(|(_, rect)| rect.x).min(),
            start.iter().map(|(_, rect)| rect.x + rect.w).max(),
            start.iter().map(|(_, rect)| rect.y).min(),
            start.iter().map(|(_, rect)| rect.y + rect.h).max(),
        ) else {
            return;
        };

        let dx = dx.clamp(-min_x, self.grid[0] - max_x);
        let dy = dy.clamp(-min_y, self.grid[1] - max_y);

        for (id, rect) in start {
            if let Some(button) = self.buttons.iter_mut().find(|button| button.id == *id) {
                button.rect.x = rect.x + dx;
                button.rect.y = rect.y + dy;
            }
        }
    }

    pub fn to_config_buttons(&self) -> Vec<Button> {
        self.buttons
            .iter()
            .map(|button| Button {
                name: button.name.clone(),
                sound: button.sound.clone(),
                image: button.image.clone(),
                position: [button.rect.x, button.rect.y],
                size: [button.rect.w, button.rect.h],
            })
            .collect()
    }

    pub fn handle_move(&mut self, index: usize, response: &Response, cell: Vec2) {
        if response.drag_started() {
            self.interaction = Some(Interaction::Move {
                index,
                start: self.buttons[index].rect,
            });
        }

        if response.dragged() {
            if let Some(Interaction::Move {
                index: active,
                start,
            }) = self.interaction
            {
                if active == index {
                    let delta = response.total_drag_delta().unwrap_or_default();
                    let mut rect = start;
                    rect.x += (delta.x / cell.x.max(1.0)).round() as i32;
                    rect.y += (delta.y / cell.y.max(1.0)).round() as i32;
                    clamp_rect(&mut rect, self.grid);
                    self.buttons[index].rect = rect;
                }
            }
        }

        if response.drag_stopped() {
            if matches!(
                self.interaction,
                Some(Interaction::Move { index: active, .. }) if active == index
            ) {
                self.interaction = None;
            }
        }
    }

    pub fn handle_resize(&mut self, index: usize, response: &Response, cell: Vec2) {
        if response.drag_started() {
            self.interaction = Some(Interaction::Resize {
                index,
                start: self.buttons[index].rect,
            });
        }

        if response.dragged() {
            if let Some(Interaction::Resize {
                index: active,
                start,
            }) = self.interaction
            {
                if active == index {
                    let delta = response.total_drag_delta().unwrap_or_default();
                    let dx = (delta.x / cell.x.max(1.0)).round() as i32;
                    let dy = (delta.y / cell.y.max(1.0)).round() as i32;

                    self.buttons[index].rect.w = (start.w + dx).clamp(1, self.grid[0] - start.x);
                    self.buttons[index].rect.h = (start.h + dy).clamp(1, self.grid[1] - start.y);
                }
            }
        }

        if response.drag_stopped() {
            if matches!(
                self.interaction,
                Some(Interaction::Resize { index: active, .. }) if active == index
            ) {
                self.interaction = None;
            }
        }
    }
}
