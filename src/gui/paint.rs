use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::board::BoardButton;
use super::texture::{ButtonImage, cover_uv};

pub const RADIUS: f32 = 5.0;
const TITLE_HEIGHT: f32 = 30.0;
const BUTTON_GAP: f32 = 5.0;

pub fn inset_button_rect(rect: Rect) -> Rect {
    let inset = (BUTTON_GAP * 0.5)
        .min(rect.width() * 0.45)
        .min(rect.height() * 0.45);
    rect.shrink(inset)
}

pub fn paint_button_background(
    ui: &egui::Ui,
    rect: Rect,
    image: Option<&ButtonImage>,
    response: &egui::Response,
) {
    let radius = CornerRadius::same(RADIUS as u8);
    let painter = ui.painter();

    if let Some(texture) = image.and_then(ButtonImage::texture) {
        let aspect_ratio = image.map(ButtonImage::aspect_ratio).unwrap_or(1.0);
        egui::Image::from_texture(texture)
            .uv(cover_uv(aspect_ratio, rect))
            .fit_to_exact_size(rect.size())
            .corner_radius(radius)
            .paint_at(ui, rect);
    } else {
        painter.rect_filled(rect, radius, Color32::from_gray(38));
    }

    let visuals = ui.style().interact(response);
    painter.rect_stroke(
        rect.expand(visuals.expansion),
        radius,
        visuals.bg_stroke,
        StrokeKind::Inside,
    );
}

pub fn paint_button_label(
    ui: &egui::Ui,
    rect: Rect,
    button: &BoardButton,
    response: &egui::Response,
    has_image: bool,
) {
    let color = ui.style().interact(response).fg_stroke.color;
    let painter = ui.painter();

    let pad_x = 6.0_f32.min(rect.width() * 0.45);
    let pad_y = 4.0_f32.min(rect.height() * 0.45);
    let wrap_width = (rect.width() - 2.0 * pad_x).max(1.0);

    let max_strip_height = if has_image {
        (rect.height() * 0.5).max(TITLE_HEIGHT).min(rect.height())
    } else {
        rect.height()
    };
    let max_text_height = (max_strip_height - 2.0 * pad_y).max(1.0);

    let galley = layout_title(
        painter,
        &button.name,
        FontId::proportional(16.0),
        color,
        wrap_width,
        max_text_height,
    );

    if has_image {
        let strip_height = (galley.size().y + 2.0 * pad_y)
            .clamp(TITLE_HEIGHT.min(rect.height()), max_strip_height);
        let strip = Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - strip_height), rect.max);
        let strip_radius = CornerRadius {
            nw: 0,
            ne: 0,
            sw: RADIUS as u8,
            se: RADIUS as u8,
        };

        painter.rect_filled(strip, strip_radius, Color32::from_black_alpha(153));

        let text_area = Rect::from_min_max(
            Pos2::new(strip.left() + pad_x, strip.top() + pad_y),
            Pos2::new(strip.right() - pad_x, strip.bottom() - pad_y),
        );
        paint_centered_galley(painter, text_area, galley, color);
    } else {
        let text_area = Rect::from_min_max(
            Pos2::new(rect.left() + pad_x, rect.top() + pad_y),
            Pos2::new(rect.right() - pad_x, rect.bottom() - pad_y),
        );
        paint_centered_galley(painter, text_area, galley, color);
    }
}

fn layout_title(
    painter: &egui::Painter,
    text: &str,
    font_id: FontId,
    color: Color32,
    wrap_width: f32,
    max_height: f32,
) -> std::sync::Arc<egui::Galley> {
    let make_job = |max_rows: usize| {
        let mut job =
            egui::text::LayoutJob::simple(text.to_owned(), font_id.clone(), color, wrap_width);
        job.halign = egui::Align::Center;
        job.wrap.max_rows = max_rows;
        job
    };

    let galley = painter.layout_job(make_job(usize::MAX));
    let row_count = galley.rows.len().max(1);
    let row_height = (galley.size().y / row_count as f32).max(1.0);
    let max_rows = ((max_height / row_height).floor() as usize).max(1);

    if galley.rows.len() > max_rows {
        painter.layout_job(make_job(max_rows))
    } else {
        galley
    }
}

fn paint_centered_galley(
    painter: &egui::Painter,
    text_area: Rect,
    galley: std::sync::Arc<egui::Galley>,
    color: Color32,
) {
    let pos = Pos2::new(
        text_area.center().x,
        text_area.center().y - galley.size().y * 0.5,
    );
    painter.with_clip_rect(text_area).galley(pos, galley, color);
}

pub fn paint_grid(
    ui: &egui::Ui,
    grid_rect: Rect,
    cell: Vec2,
    grid_size: [i32; 2],
    edit_mode: bool,
) {
    if !edit_mode {
        return;
    }

    let visuals = ui.visuals();
    let painter = ui.painter();
    let radius = CornerRadius::same(RADIUS as u8);

    painter.rect_filled(grid_rect, radius, visuals.extreme_bg_color);
    painter.rect_stroke(
        grid_rect,
        radius,
        visuals.widgets.noninteractive.bg_stroke,
        StrokeKind::Inside,
    );

    let dot_color = visuals.widgets.noninteractive.bg_stroke.color;
    let dot_radius = (cell.x.min(cell.y) * 0.03).clamp(0.5, 1.5);
    for column in 0..=grid_size[0] {
        let x = grid_rect.left() + column as f32 * cell.x;
        for row in 0..=grid_size[1] {
            let y = grid_rect.top() + row as f32 * cell.y;
            painter.circle_filled(Pos2::new(x, y), dot_radius, dot_color);
        }
    }
}

pub fn paint_resize_corner(ui: &egui::Ui, rect: Rect, response: &egui::Response) {
    // Same look as egui's own window resize corner (diagonal lines in the
    // bottom-right corner), sized from the active egui style.
    let stroke = ui.style().interact(response).fg_stroke;
    let painter = ui.painter();

    let corner = rect.right_bottom();
    let mut width = 2.0;
    while width <= rect.width() && width <= rect.height() {
        painter.line_segment(
            [
                Pos2::new(corner.x - width, corner.y),
                Pos2::new(corner.x, corner.y - width),
            ],
            Stroke::new(1.0, stroke.color),
        );
        width += 4.0;
    }
}
