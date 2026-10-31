//! Shared appearance inspector and new-save character creation flow.
use crate::{
    hunter::{self, Hunter},
    inventory_ui::draw_menu_cursor,
    presentation,
    scene::capture_frame,
    widgets::*,
};
use macroquad::prelude::*;
use psp_core::{character::Character, weapon::WeaponClass};

#[derive(Default)]
pub struct CharacterPanel {
    name_focus: bool,
}
impl CharacterPanel {
    pub fn draw(&mut self, character: &mut Character, rect: Rect) {
        let x = rect.x;
        let y = rect.y;
        label("CHARACTER NAME", x, y + 15.0, 15.0, LIGHTGRAY);
        let input = Rect::new(x, y + 24.0, rect.w, 34.0);
        if is_mouse_button_pressed(MouseButton::Left) {
            self.name_focus = input.contains(mouse_position().into());
        }
        draw_rectangle(
            input.x,
            input.y,
            input.w,
            input.h,
            Color::from_rgba(10, 22, 35, 255),
        );
        draw_rectangle_lines(
            input.x,
            input.y,
            input.w,
            input.h,
            1.0,
            if self.name_focus {
                presentation::ACCENT
            } else {
                GRAY
            },
        );
        if self.name_focus {
            while let Some(c) = get_char_pressed() {
                if character.name.len() < 20 && (c.is_ascii_alphanumeric() || " -'".contains(c)) {
                    character.name.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                character.name.pop();
            }
        }
        label(
            format!(
                "{}{}",
                character.name,
                if self.name_focus && (get_time() * 2.0) as u32 % 2 == 0 {
                    "|"
                } else {
                    ""
                }
            ),
            x + 10.0,
            y + 48.0,
            20.0,
            WHITE,
        );
        choice(
            "HEAD STYLE",
            &mut character.head,
            &["Helmet", "Cropped hair", "Spiky hair"],
            Rect::new(x, y + 54.0, rect.w, 56.0),
        );
        choice(
            "BODY PRESET",
            &mut character.build,
            &["Slim", "Balanced", "Broad"],
            Rect::new(x, y + 102.0, rect.w, 56.0),
        );
        swatches(
            "ARMOR",
            &mut character.armor,
            &(0..6).map(hunter::armor_color).collect::<Vec<_>>(),
            Rect::new(x, y + 154.0, rect.w, 50.0),
        );
        swatches(
            "PHOTON ACCENT",
            &mut character.accent,
            &(0..6).map(hunter::accent_color).collect::<Vec<_>>(),
            Rect::new(x, y + 190.0, rect.w, 50.0),
        );
        swatches(
            "SKIN",
            &mut character.skin,
            &(0..5).map(hunter::skin_color).collect::<Vec<_>>(),
            Rect::new(x, y + 226.0, rect.w, 50.0),
        );
        swatches(
            "HAIR",
            &mut character.hair,
            &(0..6).map(hunter::hair_color).collect::<Vec<_>>(),
            Rect::new(x, y + 262.0, rect.w, 50.0),
        );
    }
}
/// A viewport shared by the creator and editor. Appearance preview has no save side effects.
#[derive(Clone, Copy)]
pub struct Preview {
    pub class: WeaponClass,
    pub clip: usize,
    pub time: f32,
    pub energy: f32,
    pub yaw: f32,
    pub bones: bool,
}
pub fn draw_preview(hunter: &mut Hunter, character: &Character, preview: &Preview, viewport: Rect) {
    let Preview {
        class,
        clip,
        time,
        energy,
        yaw,
        bones,
    } = *preview;
    hunter.preview(character, class, clip, time, energy);
    set_camera(&Camera3D {
        position: vec3(yaw.sin() * 3.6, 1.65, yaw.cos() * 3.6),
        target: vec3(0.0, 0.95, 0.0),
        up: Vec3::Y,
        fovy: 40.0f32.to_radians(),
        aspect: Some(viewport.w / viewport.h),
        viewport: Some((
            viewport.x as i32,
            (screen_height() - viewport.y - viewport.h) as i32,
            viewport.w as i32,
            viewport.h as i32,
        )),
        ..Default::default()
    });
    presentation::shaded_box(
        vec3(0.0, -0.12, 0.0),
        vec3(2.4, 0.22, 2.4),
        Color::from_rgba(29, 48, 65, 255),
    );
    draw_grid(
        10,
        0.5,
        Color::from_rgba(25, 65, 78, 255),
        Color::from_rgba(18, 40, 51, 255),
    );
    hunter.draw(bones);
    set_default_camera();
}

pub async fn create_character(initial: Character) -> Option<Character> {
    let mut character = initial;
    let mut panel = CharacterPanel::default();
    let mut hunter = Hunter::default();
    let mut yaw = 0.35f32;
    let mut time = 0.0;
    let mut notice = String::new();
    let mut frames = 0;
    let smoke = std::env::var_os("PSP_CREATOR_SMOKE").is_some();
    loop {
        if is_quit_requested() || is_key_pressed(KeyCode::Escape) {
            show_mouse(true);
            return None;
        }
        let dt = get_frame_time().clamp(0.0, 0.1);
        time = (time + dt) % 2.4;
        clear_background(Color::from_rgba(8, 18, 29, 255));
        let width = screen_width();
        let height = screen_height();
        let left = Rect::new(24.0, 78.0, 300.0, height - 148.0);
        label("CREATE YOUR HUNTER", 28.0, 40.0, 28.0, WHITE);
        label(
            "New character / original appearance presets",
            28.0,
            62.0,
            16.0,
            LIGHTGRAY,
        );
        section(left, "APPEARANCE");
        panel.draw(
            &mut character,
            Rect::new(left.x + 16.0, left.y + 44.0, left.w - 32.0, left.h - 50.0),
        );
        let viewport = Rect::new(340.0, 78.0, width - 360.0, height - 150.0);
        if is_mouse_button_down(MouseButton::Middle) {
            yaw -= mouse_delta_position().x * 3.0;
        }
        if is_key_down(KeyCode::Left) {
            yaw += dt;
        }
        if is_key_down(KeyCode::Right) {
            yaw -= dt;
        }
        draw_preview(
            &mut hunter,
            &character,
            &Preview {
                class: WeaponClass::Sword,
                clip: 0,
                time,
                energy: 1.0,
                yaw,
                bones: false,
            },
            viewport,
        );
        label(
            "MMB drag / Left-Right: rotate preview",
            360.0,
            height - 88.0,
            16.0,
            LIGHTGRAY,
        );
        label(
            "Appearance is cosmetic. Your equipment controls combat.",
            28.0,
            height - 30.0,
            16.0,
            LIGHTGRAY,
        );
        if smoke && frames == 1 {
            character = Character {
                name: "Astra".into(),
                armor: 2,
                accent: 2,
                skin: 2,
                hair: 3,
                head: 2,
                build: 0,
            };
        }
        if button(
            "Begin adventure",
            Rect::new(width - 225.0, height - 65.0, 200.0, 38.0),
        ) || is_key_pressed(KeyCode::Enter)
            || (smoke && frames >= 5)
        {
            character.name = character.name.trim().to_owned();
            match character.validate() {
                Ok(()) => {
                    show_mouse(true);
                    return Some(character);
                }
                Err(e) => notice = e.into(),
            }
        }
        label(&notice, 350.0, height - 110.0, 16.0, ORANGE);
        show_mouse(false);
        set_cursor_grab(false);
        draw_menu_cursor();
        frames += 1;
        if frames == 3 {
            if let Some(path) = std::env::var_os("PSP_CREATOR_CAPTURE") {
                capture_frame(std::path::Path::new(&path)).expect("creator capture failed");
                if !smoke {
                    show_mouse(true);
                    return None;
                }
            }
        }
        next_frame().await;
    }
}
