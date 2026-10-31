use crate::{hunter::Hunter, presentation};
use macroquad::prelude::*;
use psp_core::{
    world::{EnemyAction, World},
    Location, QuestState, TRIAL_WAVES,
};
const PANEL: Color = Color::new(0.025, 0.07, 0.13, 0.9);
const CYAN: Color = Color::new(0.18, 0.86, 1.0, 1.0);
struct Popup {
    point: Vec3,
    text: String,
    color: Color,
    age: f32,
}
#[derive(Default)]
pub struct Feedback {
    popups: Vec<Popup>,
    trail: Vec<([Vec3; 2], f32)>,
}
pub fn project(camera: &Camera3D, p: Vec3) -> Option<Vec2> {
    let p = camera.matrix() * p.extend(1.0);
    if p.w <= 0.0 {
        return None;
    }
    let ndc = p.truncate() / p.w;
    if ndc.x.abs() > 1.2 || ndc.y.abs() > 1.2 {
        return None;
    }
    Some(vec2(
        (ndc.x + 1.0) * screen_width() * 0.5,
        (1.0 - ndc.y) * screen_height() * 0.5,
    ))
}
impl Feedback {
    pub fn damage(&mut self, world: &World, damage: u32) {
        let p = world.enemy_position();
        let attack = world.attack();
        self.popups.push(Popup {
            point: vec3(p.x, 1.8, p.z),
            text: damage.to_string(),
            color: if attack.is_some_and(|a| a.perfect) {
                ORANGE
            } else {
                WHITE
            },
            age: 0.0,
        });
        if self.popups.len() > 24 {
            self.popups.remove(0);
        }
    }
    pub fn heal(&mut self, world: &World, amount: u32) {
        let p = world.position();
        self.popups.push(Popup {
            point: vec3(p.x, world.height() + 1.4, p.z),
            text: format!("+{}", amount),
            color: GREEN,
            age: 0.0,
        });
    }
    pub fn tick(&mut self, delta: f32, hunter: &Hunter, world: &World) {
        for p in &mut self.popups {
            p.age += delta;
        }
        self.popups.retain(|p| p.age < 0.85);
        for (_, age) in &mut self.trail {
            *age += delta;
        }
        self.trail.retain(|(_, age)| *age < 0.11);
        if world
            .attack()
            .is_some_and(|a| (0.18..0.78).contains(&a.progress()))
        {
            self.trail.push((hunter.blade, 0.0));
            if self.trail.len() > 10 {
                self.trail.remove(0);
            }
        }
    }
    pub fn draw_world(&self, world: &World) {
        for pair in self.trail.windows(2) {
            let alpha = (1.0 - pair[0].1 / 0.11).max(0.0);
            let color = if world.attack().is_some_and(|a| a.photon) {
                Color::new(0.65, 0.3, 1.0, alpha)
            } else {
                Color::new(0.2, 0.9, 1.0, alpha)
            };
            draw_line_3d(pair[0].0[1], pair[1].0[1], color);
            draw_line_3d(pair[0].0[0].lerp(pair[0].0[1], 0.45), pair[1].0[1], color);
            draw_line_3d(pair[1].0[0], pair[1].0[1], Color::new(1.0, 1.0, 1.0, alpha));
        }
        if world.dodging() {
            let p = world.position();
            draw_cylinder_wires(vec3(p.x, 0.12, p.z), 0.55, 0.7, 0.2, None, CYAN);
        }
    }
    pub fn draw_screen(&self, world: &World, camera: &Camera3D) {
        for popup in &self.popups {
            if let Some(p) = project(camera, popup.point + Vec3::Y * popup.age * 1.3) {
                let color = Color::new(
                    popup.color.r,
                    popup.color.g,
                    popup.color.b,
                    (1.0 - popup.age / 0.85).min(1.0),
                );
                draw_text(&popup.text, p.x + 12.0, p.y, 32.0, color);
            }
        }
        if world.locked_on() {
            let p = world.enemy_position();
            if let Some(p) = project(camera, vec3(p.x, 1.1, p.z)) {
                let radius = 28.0;
                let angle = world.time() * 0.45;
                for i in 0..4 {
                    let a = angle + i as f32 * std::f32::consts::FRAC_PI_2;
                    let b = a + std::f32::consts::FRAC_PI_2;
                    let v = p + vec2(a.cos(), a.sin()) * radius;
                    let w = p + vec2(b.cos(), b.sin()) * radius;
                    draw_line(v.x, v.y, w.x, w.y, 2.0, ORANGE);
                }
                draw_text("LOCK", p.x - 19.0, p.y - radius - 9.0, 16.0, ORANGE);
            }
        }
        if let Some(attack) = world.attack().filter(|a| !a.photon && a.progress() > 0.48) {
            let p = world.position();
            if let Some(p) = project(camera, vec3(p.x, world.height() + 1.1, p.z)) {
                let radius = 52.0 - (attack.progress() - 0.48) * 65.0;
                let color = if attack.timing_window() {
                    ORANGE
                } else {
                    Color::new(0.2, 0.7, 1.0, 0.7)
                };
                draw_circle_lines(p.x, p.y, radius.max(14.0), 2.0, color);
                if attack.timing_window() {
                    draw_text("TIMED ATTACK", p.x - 51.0, p.y + 70.0, 17.0, ORANGE);
                }
            }
        }
    }
}
fn text_right(text: &str, right: f32, y: f32, size: u16, color: Color) {
    draw_text(
        text,
        right - measure_text(text, None, size, 1.0).width,
        y,
        size as f32,
        color,
    );
}
pub fn hud(
    world: &World,
    message: &str,
    message_time: f32,
    projection: bool,
    monster_motion: &str,
) {
    let w = screen_width();
    let h = screen_height();
    if world.hurt() {
        let color = Color::new(1.0, 0.08, 0.02, 0.22);
        draw_rectangle(0.0, 0.0, 12.0, h, color);
        draw_rectangle(w - 12.0, 0.0, 12.0, h, color);
        draw_rectangle(0.0, 0.0, w, 12.0, color);
        draw_rectangle(0.0, h - 12.0, w, 12.0, color);
    }
    let hunter = world.session().hunter();
    presentation::panel(Rect::new(18.0, 16.0, 300.0, 43.0), CYAN);
    draw_rectangle(18.0, 16.0, 3.0, 43.0, CYAN);
    draw_text(
        format!(
            "{} / {}",
            world.character().name,
            world.weapon().kind.class().name()
        ),
        30.0,
        36.0,
        17.0,
        WHITE,
    );
    draw_text(
        format!(
            "LV {}   XP {} / {}   ATK {}   {} M",
            hunter.level,
            hunter.experience,
            hunter.level * 100,
            world.weapon_power(),
            world.inventory().meseta()
        ),
        30.0,
        52.0,
        15.0,
        LIGHTGRAY,
    );
    if world.session().location() == Location::TrainingField
        && world.session().quest() == QuestState::Active
    {
        presentation::panel(Rect::new(18.0, 69.0, 256.0, 55.0), ORANGE);
        draw_text(
            if projection {
                if world.session().wave() == 3 {
                    "DAGAN / ENHANCED"
                } else {
                    "DAGAN / COMBAT TRIAL"
                }
            } else {
                "FOREST CRAWLER"
            },
            30.0,
            90.0,
            18.0,
            WHITE,
        );
        draw_rectangle(30.0, 101.0, 230.0, 6.0, DARKGRAY);
        draw_rectangle(
            30.0,
            101.0,
            230.0 * world.session().enemy_health() as f32
                / world.session().enemy_max_health() as f32,
            6.0,
            ORANGE,
        );
        draw_text(
            format!(
                "{} m  /  Q: {}",
                world.position().distance(world.enemy_position()).round(),
                if world.locked_on() {
                    "unlock"
                } else {
                    "lock on"
                }
            ),
            30.0,
            120.0,
            15.0,
            LIGHTGRAY,
        );
    }
    if world.session().location() == Location::TrainingField && world.session().enemy_health() > 0 {
        draw_text(
            monster_motion,
            22.0,
            141.0,
            13.0,
            if monster_motion.starts_with("Basic") {
                ORANGE
            } else {
                CYAN
            },
        );
    }
    let lobby = world.session().location() == Location::Lobby;
    text_right(
        if lobby {
            "ORBITAL HUB"
        } else {
            "HUNTER COMBAT TRIAL"
        },
        w - 22.0,
        35.0,
        22,
        WHITE,
    );
    text_right(
        match (world.session().quest(), lobby) {
            (QuestState::Available, _) => "Mission terminal / E to deploy",
            (QuestState::Active, _) => "Clear all three waves",
            (_, true) => "E at terminal / Deploy again",
            _ => "Return pad / E to extract",
        },
        w - 22.0,
        58.0,
        17,
        CYAN,
    );
    if !lobby && world.session().quest() == QuestState::Active {
        text_right(
            &format!("WAVE {} / {}", world.session().wave(), TRIAL_WAVES),
            w - 22.0,
            82.0,
            20,
            if world.session().wave() == 3 {
                ORANGE
            } else {
                WHITE
            },
        );
        if world.wave_delay() > 0.0 {
            text_right(
                &format!("Next wave in {:.1}s", world.wave_delay()),
                w - 22.0,
                105.0,
                17,
                CYAN,
            );
        }
    }
    if !lobby && world.session().quest() == QuestState::Complete {
        results(world);
    }
    let x = 18.0;
    let y = h - 134.0;
    presentation::panel(Rect::new(x, y, 256.0, 111.0), CYAN);
    draw_text(
        format!("HP   {} / {}", hunter.health, hunter.max_health()),
        x + 12.0,
        y + 22.0,
        19.0,
        WHITE,
    );
    draw_rectangle(x + 12.0, y + 31.0, 230.0, 10.0, DARKGRAY);
    draw_rectangle(
        x + 12.0,
        y + 31.0,
        230.0 * hunter.health as f32 / hunter.max_health() as f32,
        10.0,
        if world.hurt() {
            RED
        } else {
            Color::new(0.2, 0.8, 0.72, 1.0)
        },
    );
    draw_text(
        format!("PP   {:03.0} / 100", world.pp()),
        x + 12.0,
        y + 65.0,
        19.0,
        WHITE,
    );
    draw_rectangle(x + 12.0, y + 74.0, 230.0, 7.0, DARKGRAY);
    draw_rectangle(x + 12.0, y + 74.0, 230.0 * world.pp() / 100.0, 7.0, CYAN);
    draw_text(
        format!("H: recovery x{}   Space: jump", world.session().mates()),
        x + 12.0,
        y + 99.0,
        15.0,
        LIGHTGRAY,
    );
    let center = w * 0.5 + 35.0;
    let art = world.weapon().photon_art;
    let art_label = art.map_or("EMPTY SLOT".to_owned(), |a| {
        format!("{} / {:.0} PP", a.spec().short_name, a.spec().pp)
    });
    for (i, (key, label, ready)) in [
        ("LMB / J", world.weapon().kind.class().name(), true),
        (
            "RMB / F",
            art_label.as_str(),
            art.is_some_and(|a| world.pp() >= a.spec().pp),
        ),
        ("X", "STEP", world.dodge_ready()),
    ]
    .into_iter()
    .enumerate()
    {
        let x = center - 175.0 + i as f32 * 121.0;
        let y = h - 94.0;
        let active = match i {
            0 => world.attack().is_some_and(|a| !a.photon),
            1 => world.attack().is_some_and(|a| a.photon),
            _ => world.dodging(),
        };
        draw_rectangle(
            x,
            y,
            116.0,
            69.0,
            if active {
                Color::new(0.12, 0.38, 0.48, 0.96)
            } else {
                PANEL
            },
        );
        draw_rectangle_lines(
            x,
            y,
            116.0,
            69.0,
            if active { 2.0 } else { 1.0 },
            if ready { CYAN } else { GRAY },
        );
        draw_text(key, x + 8.0, y + 24.0, 18.0, WHITE);
        draw_text(
            label,
            x + 7.0,
            y + 48.0,
            14.0,
            if ready { CYAN } else { GRAY },
        );
        if i == 0 {
            if let Some(a) = world.attack() {
                draw_text(
                    format!("{:01}", a.stage.max(1)),
                    x + 74.0,
                    y + 24.0,
                    19.0,
                    ORANGE,
                );
            }
        }
    }
    text_right("MMB / arrows: camera", w - 18.0, h - 68.0, 15, LIGHTGRAY);
    text_right(
        "Wheel: zoom   Esc: pause",
        w - 18.0,
        h - 46.0,
        15,
        LIGHTGRAY,
    );
    text_right(
        "Tab / I: inventory   F2: bones",
        w - 18.0,
        h - 24.0,
        15,
        LIGHTGRAY,
    );
    if world.defeated() {
        draw_rectangle(0.0, h * 0.37, w, 100.0, Color::new(0.1, 0.01, 0.03, 0.9));
        let label = "INCAPACITATED / E TO RETRY";
        let size = 27;
        draw_text(
            label,
            (w - measure_text(label, None, size, 1.0).width) * 0.5,
            h * 0.37 + 58.0,
            size as f32,
            WHITE,
        );
    } else if let Some(prompt) = world.interaction_prompt() {
        let width = measure_text(prompt, None, 21, 1.0).width;
        draw_rectangle(
            (w - width) * 0.5 - 16.0,
            h - 170.0,
            width + 32.0,
            35.0,
            PANEL,
        );
        draw_text(prompt, (w - width) * 0.5, h - 146.0, 21.0, CYAN);
    } else if world.enemy_action() == EnemyAction::Windup && !lobby {
        let label = "INCOMING / X TO STEP";
        draw_text(
            label,
            (w - measure_text(label, None, 20, 1.0).width) * 0.5,
            h - 150.0,
            20.0,
            ORANGE,
        );
    }
    if message_time > 0.0 && !world.defeated() {
        let size = 19;
        let width = measure_text(message, None, size, 1.0).width;
        draw_text(message, (w - width) * 0.5, h - 198.0, size as f32, YELLOW);
    }
}

fn results(world: &World) {
    let s = world.stats();
    let x = screen_width() - 318.0;
    let y = 84.0;
    draw_rectangle(x, y, 300.0, 226.0, PANEL);
    draw_rectangle_lines(x, y, 300.0, 226.0, 1.0, CYAN);
    draw_text("TRIAL COMPLETE", x + 16.0, y + 29.0, 23.0, WHITE);
    draw_text(s.rank(), x + 250.0, y + 40.0, 38.0, ORANGE);
    for (i, line) in [
        format!("Score {} / {:.1}s", s.score(), s.duration),
        format!("Enemies cleared   {} / {}", s.defeats, TRIAL_WAVES),
        format!("Timed hits {} / Evades {}", s.perfect_attacks, s.evades),
        format!("Damage {} / Recovery {}", s.damage_taken, s.heals_used),
        "Reward   +100 XP / +300 Meseta".to_owned(),
        "Return pad / E to extract".to_owned(),
    ]
    .iter()
    .enumerate()
    {
        draw_text(
            line,
            x + 16.0,
            y + 65.0 + i as f32 * 26.0,
            17.0,
            if i == 4 { YELLOW } else { LIGHTGRAY },
        );
    }
}
