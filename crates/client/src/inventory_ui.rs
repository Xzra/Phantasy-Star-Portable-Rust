//! Paused inventory: selecting an item is separate from equipping it.
use crate::presentation;
use macroquad::prelude::*;
use psp_core::{
    inventory::{WeaponId, INVENTORY_CAPACITY},
    world::World,
};

const ACCENT: Color = Color::new(0.15, 0.85, 0.95, 1.0);
const PANEL: Color = Color::new(0.035, 0.065, 0.11, 1.0);

/// Draw after the menu so the pointer stays visible above every panel.
/// The tip uses the same coordinates as the menu's click targets.
pub fn draw_menu_cursor() {
    let (x, y) = mouse_position();
    let tip = vec2(x, y);
    let left = tip + vec2(1.0, 23.0);
    let right = tip + vec2(17.0, 16.0);
    let shadow = vec2(2.0, 2.0);
    draw_triangle(tip + shadow, left + shadow, right + shadow, BLACK);
    draw_line(x + 8.0, y + 16.0, x + 14.0, y + 29.0, 6.0, BLACK);
    draw_triangle(tip, left, right, WHITE);
    draw_triangle_lines(tip, left, right, 1.5, ACCENT);
    draw_line(x + 7.0, y + 15.0, x + 12.0, y + 26.0, 3.0, WHITE);
}

pub struct InventoryMenu {
    selected: WeaponId,
    notice: String,
}
impl Default for InventoryMenu {
    fn default() -> Self {
        Self {
            selected: WeaponId(1),
            notice: String::new(),
        }
    }
}

pub(crate) fn card(rect: Rect, selected: bool) -> bool {
    let hover = rect.contains(vec2(mouse_position().0, mouse_position().1));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if selected {
            Color::new(0.1, 0.3, 0.38, 1.0)
        } else {
            PANEL
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if selected { 2.0 } else { 1.0 },
        if selected || hover { ACCENT } else { GRAY },
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}
pub(crate) fn button(label: &str, rect: Rect) -> bool {
    let clicked = card(rect, false);
    draw_text(label, rect.x + 12.0, rect.y + 26.0, 18.0, WHITE);
    clicked
}

impl InventoryMenu {
    pub fn draw(&mut self, world: &mut World) -> bool {
        let width = (screen_width() - 32.0).min(1040.0);
        let height = (screen_height() - 32.0).min(540.0);
        let x = (screen_width() - width) * 0.5;
        let y = (screen_height() - height) * 0.5;
        presentation::panel(
            Rect::new(x - 8.0, y - 8.0, width + 16.0, height + 16.0),
            ACCENT,
        );
        draw_line(
            x + 12.0,
            y + 62.0,
            x + width - 12.0,
            y + 62.0,
            1.0,
            Color::new(0.2, 0.4, 0.48, 0.6),
        );
        draw_text(
            "EQUIPMENT  /  WEAPONS & PHOTON ARTS",
            x + 12.0,
            y + 28.0,
            23.0,
            WHITE,
        );
        draw_text(
            format!(
                "{} / {} items  |  {} Meseta  |  Character inventory",
                world.inventory().item_count(),
                INVENTORY_CAPACITY,
                world.inventory().meseta()
            ),
            x + 12.0,
            y + 52.0,
            16.0,
            LIGHTGRAY,
        );

        let weapons = world.inventory().weapons().to_vec();
        let mut index = weapons
            .iter()
            .position(|w| w.id == self.selected)
            .unwrap_or(0);
        let left_width = (width * 0.32).min(300.0);
        let mouse_on_weapons = Rect::new(x, y + 65.0, left_width, height - 170.0)
            .contains(vec2(mouse_position().0, mouse_position().1));
        if is_key_pressed(KeyCode::Up) || (mouse_on_weapons && mouse_wheel().1 > 0.0) {
            index = index.saturating_sub(1);
        }
        if is_key_pressed(KeyCode::Down) || (mouse_on_weapons && mouse_wheel().1 < 0.0) {
            index = (index + 1).min(weapons.len() - 1);
        }
        self.selected = weapons[index].id;
        draw_text("OWNED WEAPONS", x + 12.0, y + 80.0, 18.0, WHITE);
        let visible = ((height - 245.0) / 64.0).floor().max(2.0) as usize;
        let start = index / visible * visible;
        for (row, item) in weapons.iter().enumerate().skip(start).take(visible) {
            let rect = Rect::new(
                x + 8.0,
                y + 94.0 + (row - start) as f32 * 64.0,
                left_width - 16.0,
                58.0,
            );
            if card(rect, self.selected == item.id) {
                self.selected = item.id;
                self.notice.clear();
            }
            draw_text(
                format!("{} #{}", item.loadout.kind.name(), item.id.0),
                rect.x + 9.0,
                rect.y + 24.0,
                19.0,
                WHITE,
            );
            let status = if world.inventory().equipped_id() == item.id {
                "EQUIPPED"
            } else {
                "OWNED"
            };
            draw_text(
                format!("{}  /  ATK +{}", status, item.loadout.kind.attack_bonus()),
                rect.x + 9.0,
                rect.y + 46.0,
                15.0,
                ACCENT,
            );
        }
        let selected = *world.inventory().weapon(self.selected).unwrap();
        let detail_y = y + height - 130.0;
        draw_text(
            format!("Weapon {} / {}", index + 1, weapons.len()),
            x + 12.0,
            detail_y - 38.0,
            15.0,
            LIGHTGRAY,
        );
        draw_text(
            format!(
                "Attack power: {}",
                ((world.session().hunter().attack_power() + selected.loadout.kind.attack_bonus())
                    as f32
                    * world.settings().weapon(selected.loadout.kind.class()).power)
                    .round() as u32
            ),
            x + 12.0,
            detail_y - 15.0,
            18.0,
            WHITE,
        );
        draw_text(
            selected.loadout.kind.description(),
            x + 12.0,
            detail_y + 8.0,
            14.0,
            LIGHTGRAY,
        );

        let right = x + left_width + 12.0;
        let right_width = width - left_width - 24.0;
        draw_text(
            format!(
                "{} / COMPATIBLE OWNED PA DISCS",
                selected.loadout.kind.class().name()
            ),
            right,
            y + 80.0,
            17.0,
            WHITE,
        );
        let discs: Vec<_> = world
            .inventory()
            .discs()
            .iter()
            .copied()
            .filter(|a| selected.loadout.kind.accepts(*a))
            .collect();
        if discs.is_empty() {
            draw_text(
                "Buy a compatible PA disc from Mira in the lobby.",
                right,
                y + 118.0,
                17.0,
                LIGHTGRAY,
            );
        }
        let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];
        for (i, art) in discs.into_iter().enumerate() {
            let spec = art.spec();
            let rect = Rect::new(right, y + 94.0 + i as f32 * 74.0, right_width, 68.0);
            let active = selected.loadout.photon_art == Some(art);
            let assign = card(rect, active) || is_key_pressed(keys[i]);
            draw_text(
                format!(
                    "{}  {}  /  {:.0} PP  /  {}%",
                    i + 1,
                    spec.name,
                    spec.pp,
                    spec.power_percent
                ),
                rect.x + 10.0,
                rect.y + 23.0,
                19.0,
                WHITE,
            );
            let owner = world
                .inventory()
                .disc_owner(art)
                .and_then(|id| world.inventory().weapon(id));
            let status = owner.map_or("In bag".to_owned(), |w| {
                format!("Slotted: {} #{}", w.loadout.kind.name(), w.id.0)
            });
            draw_text(status, rect.x + 10.0, rect.y + 43.0, 15.0, ACCENT);
            draw_text(
                spec.description,
                rect.x + 10.0,
                rect.y + 60.0,
                14.0,
                LIGHTGRAY,
            );
            if assign {
                self.notice = match world.assign_photon_art(self.selected, Some(art)) {
                    Ok(()) => format!(
                        "{} assigned to {}.",
                        spec.name,
                        selected.loadout.kind.name()
                    ),
                    Err(error) => error.to_owned(),
                };
            }
        }
        if button(
            "Equip weapon / E",
            Rect::new(x + 8.0, y + height - 85.0, left_width - 16.0, 36.0),
        ) || is_key_pressed(KeyCode::E)
        {
            self.notice = match world.equip_weapon(self.selected) {
                Ok(()) => format!(
                    "{} equipped. RMB activates its slotted PA.",
                    selected.loadout.kind.name()
                ),
                Err(error) => error.to_owned(),
            };
        }
        if button(
            "Remove PA / 0",
            Rect::new(right, y + height - 85.0, 190.0, 36.0),
        ) || is_key_pressed(KeyCode::Key0)
        {
            self.notice = match world.assign_photon_art(self.selected, None) {
                Ok(()) => "PA disc returned to your bag.".to_owned(),
                Err(error) => error.to_owned(),
            };
        }
        draw_text(&self.notice, x + 12.0, y + height - 104.0, 15.0, ACCENT);
        draw_text(
            "Up/Down: select weapon   1/2/3: assign PA   E: equip   Tab / I: close",
            x + 12.0,
            y + height - 20.0,
            15.0,
            LIGHTGRAY,
        );
        button(
            "Return",
            Rect::new(x + width - 130.0, y + height - 85.0, 118.0, 36.0),
        )
    }
}
