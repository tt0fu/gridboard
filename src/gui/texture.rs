use eframe::egui::{self, Rect, TextureHandle};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct ButtonImage {
    path: Option<PathBuf>,
    synced: bool,
    texture: Option<TextureHandle>,
    aspect_ratio: f32,
    error: Option<String>,
}

impl Default for ButtonImage {
    fn default() -> Self {
        Self {
            path: None,
            synced: false,
            texture: None,
            aspect_ratio: 1.0,
            error: None,
        }
    }
}

impl ButtonImage {
    pub fn sync(&mut self, ctx: &egui::Context, path: Option<&Path>) {
        let new_path = path.map(Path::to_path_buf);
        if self.synced && self.path == new_path {
            return;
        }

        self.synced = true;
        self.path = new_path;
        self.texture = None;
        self.aspect_ratio = 1.0;
        self.error = None;

        let Some(path) = self.path.as_deref() else {
            return;
        };

        match load_texture(ctx, path) {
            Ok((texture, aspect_ratio)) => {
                self.texture = Some(texture);
                self.aspect_ratio = aspect_ratio;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

fn load_texture(ctx: &egui::Context, path: &Path) -> Result<(TextureHandle, f32), String> {
    let bytes = fs::read(path).map_err(|error| format!("Failed to read image: {error}"))?;
    let decoded = image::load_from_memory(&bytes)
        .map_err(|error| format!("Failed to decode image: {error}"))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return Err("Image has no pixels".to_owned());
    }

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        decoded.as_raw(),
    );
    let texture = ctx.load_texture(
        format!("button-image://{}", path.display()),
        color_image,
        egui::TextureOptions::LINEAR,
    );

    Ok((texture, width as f32 / height as f32))
}

pub fn cover_uv(image_aspect_ratio: f32, rect: Rect) -> Rect {
    let target_aspect_ratio = rect.width() / rect.height();

    if target_aspect_ratio > image_aspect_ratio {
        let visible_height = image_aspect_ratio / target_aspect_ratio;
        let inset = (1.0 - visible_height) / 2.0;
        Rect::from_min_max(
            egui::Pos2::new(0.0, inset),
            egui::Pos2::new(1.0, 1.0 - inset),
        )
    } else {
        let visible_width = target_aspect_ratio / image_aspect_ratio;
        let inset = (1.0 - visible_width) / 2.0;
        Rect::from_min_max(
            egui::Pos2::new(inset, 0.0),
            egui::Pos2::new(1.0 - inset, 1.0),
        )
    }
}
