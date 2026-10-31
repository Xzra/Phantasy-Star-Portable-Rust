//! Small shared controls used by character creation and the project editor.
use crate::presentation::{self, ACCENT};
use macroquad::prelude::*;

pub fn label(text: impl AsRef<str>, x: f32, y: f32, size: f32, color: Color) {
    draw_text(text.as_ref(), x, y, size, color);
}
pub fn button(text: &str, rect: Rect) -> bool {
    let hover = rect.contains(mouse_position().into());
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hover {
            Color::from_rgba(28, 74, 91, 255)
        } else {
            Color::from_rgba(15, 35, 49, 255)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if hover {
            ACCENT
        } else {
            Color::from_rgba(66, 96, 113, 255)
        },
    );
    let size = 17.0;
    label(
        text,
        rect.x + 10.0,
        rect.y + rect.h * 0.5 + size * 0.33,
        size,
        WHITE,
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}
pub fn choice(title: &str, value: &mut u8, names: &[&str], rect: Rect) {
    label(title, rect.x, rect.y + 14.0, 15.0, LIGHTGRAY);
    let row = Rect::new(rect.x, rect.y + 20.0, rect.w, 26.0);
    if button("<", Rect::new(row.x, row.y, 28.0, row.h)) {
        *value = (*value + names.len() as u8 - 1) % names.len() as u8;
    }
    if button(">", Rect::new(row.x + row.w - 28.0, row.y, 28.0, row.h)) {
        *value = (*value + 1) % names.len() as u8;
    }
    label(
        names[*value as usize],
        row.x + 38.0,
        row.y + 22.0,
        18.0,
        WHITE,
    );
}
pub fn slider(title: &str, value: &mut f32, min: f32, max: f32, rect: Rect) {
    label(
        format!("{title}   {value:.2}"),
        rect.x,
        rect.y + 14.0,
        15.0,
        LIGHTGRAY,
    );
    let track = Rect::new(rect.x, rect.y + 24.0, rect.w, 20.0);
    if is_mouse_button_down(MouseButton::Left) && track.contains(mouse_position().into()) {
        *value = (min + (mouse_position().0 - track.x) / track.w * (max - min)).clamp(min, max);
    }
    draw_rectangle(track.x, track.y + 8.0, track.w, 4.0, DARKGRAY);
    let k = (*value - min) / (max - min);
    draw_circle(track.x + k * track.w, track.y + 10.0, 7.0, ACCENT);
}
pub fn swatches(title: &str, value: &mut u8, colors: &[[f32; 3]], rect: Rect) {
    label(title, rect.x, rect.y + 14.0, 15.0, LIGHTGRAY);
    let width = ((rect.w - 10.0) / colors.len() as f32).min(40.0);
    for (i, c) in colors.iter().enumerate() {
        let r = Rect::new(rect.x + i as f32 * width, rect.y + 19.0, width - 6.0, 17.0);
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(c[0], c[1], c[2], 1.0));
        if *value as usize == i {
            draw_rectangle_lines(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0, 2.0, WHITE);
        }
        if r.contains(mouse_position().into()) && is_mouse_button_pressed(MouseButton::Left) {
            *value = i as u8;
        }
    }
}
pub fn section(rect: Rect, title: &str) {
    presentation::panel(rect, ACCENT);
    label(title, rect.x + 14.0, rect.y + 26.0, 20.0, WHITE);
}
