use crate::{
    inventory_ui::{button, card},
    presentation,
};
use macroquad::prelude::*;
use psp_core::{
    inventory::WeaponId,
    shop::{StockItem, STOCK},
    weapon::PhotonArt,
    world::{World, SHOPKEEPER},
};

const GOLD: Color = Color::new(1.0, 0.73, 0.3, 1.0);
#[derive(Clone, Copy)]
enum Sale {
    Weapon(WeaponId),
    Disc(PhotonArt),
}
#[derive(Default)]
pub struct ShopMenu {
    selling: bool,
    selected: usize,
    notice: String,
}

pub fn draw_keeper(time: f32) {
    let p = vec3(SHOPKEEPER.x, 0.0, SHOPKEEPER.z - 1.0);
    let coat = Color::new(0.78, 0.82, 0.75, 1.0);
    let uniform = Color::new(0.13, 0.28, 0.31, 1.0);
    let skin = Color::new(0.64, 0.43, 0.3, 1.0);
    let bob = (time * 1.6).sin() * 0.012;
    for side in [-1.0, 1.0] {
        presentation::shaded_box(
            p + vec3(side * 0.16, 0.4, 0.),
            vec3(0.24, 0.8, 0.26),
            uniform,
        );
        presentation::shaded_box(
            p + vec3(side * 0.15, 0.08, 0.08),
            vec3(0.26, 0.16, 0.4),
            Color::new(0.08, 0.12, 0.15, 1.0),
        );
        presentation::shaded_box(
            p + vec3(side * 0.39, 1.2 + bob, 0.03),
            vec3(0.18, 0.5, 0.22),
            coat,
        );
        presentation::shaded_box(
            p + vec3(side * 0.38, 0.96 + bob, 0.18),
            vec3(0.16, 0.15, 0.24),
            skin,
        );
    }
    presentation::shaded_box(p + vec3(0., 1.12 + bob, 0.), vec3(0.63, 0.72, 0.36), coat);
    presentation::shaded_box(
        p + vec3(0., 1.2 + bob, 0.19),
        vec3(0.22, 0.46, 0.03),
        uniform,
    );
    presentation::shaded_box(p + vec3(0., 1.72 + bob, 0.), vec3(0.34, 0.4, 0.32), skin);
    presentation::shaded_box(
        p + vec3(0., 1.94 + bob, -0.015),
        vec3(0.4, 0.12, 0.37),
        Color::new(0.13, 0.09, 0.1, 1.0),
    );
    presentation::shaded_box(
        p + vec3(0., 1.77 + bob, 0.17),
        vec3(0.3, 0.065, 0.025),
        Color::new(0.09, 0.19, 0.24, 1.0),
    );
    presentation::shaded_box(
        p + vec3(0.18, 1.36 + bob, 0.2),
        vec3(0.1, 0.08, 0.025),
        GOLD,
    );
    presentation::shaded_box(
        vec3(6.0, 0.94, 5.0),
        vec3(3.7, 0.14, 1.4),
        Color::new(0.3, 0.25, 0.19, 1.0),
    );
    presentation::shaded_box(vec3(6.0, 0.65, 5.68), vec3(2.4, 0.12, 0.035), GOLD);
    for (i, color) in [GOLD, SKYBLUE, VIOLET].into_iter().enumerate() {
        presentation::shaded_box(
            vec3(5.2 + i as f32 * 0.75, 1.12, 5.05),
            vec3(0.4, 0.06, 0.18),
            color,
        );
    }
}

impl ShopMenu {
    pub fn draw(&mut self, world: &mut World) -> bool {
        if !world.can_shop() {
            return true;
        }
        let width = (screen_width() - 32.0).min(1000.0);
        let height = (screen_height() - 32.0).min(550.0);
        let x = (screen_width() - width) * 0.5;
        let y = (screen_height() - height) * 0.5;
        presentation::panel(Rect::new(x, y, width, height), GOLD);
        draw_text("MIRA / FIELD SUPPLY", x + 18., y + 30., 25., WHITE);
        draw_text(
            "Welcome back, Hunter. Ready for your next deployment?",
            x + 18.,
            y + 54.,
            16.,
            LIGHTGRAY,
        );
        let wallet = format!("{} MESETA", world.inventory().meseta());
        draw_text(
            &wallet,
            x + width - measure_text(&wallet, None, 22, 1.0).width - 18.,
            y + 30.,
            22.,
            GOLD,
        );
        for (i, label) in ["BUY STOCK", "SELL ITEMS"].into_iter().enumerate() {
            let rect = Rect::new(x + 18. + i as f32 * 160., y + 72., 150., 32.);
            if card(rect, self.selling == (i == 1)) {
                self.selling = i == 1;
                self.selected = 0;
                self.notice.clear();
            }
            draw_text(label, rect.x + 12., rect.y + 22., 17., WHITE);
        }
        let sales: Vec<_> = world
            .inventory()
            .weapons()
            .iter()
            .map(|w| Sale::Weapon(w.id))
            .chain(world.inventory().discs().iter().copied().map(Sale::Disc))
            .collect();
        let count = if self.selling {
            sales.len()
        } else {
            STOCK.len()
        };
        self.selected = self.selected.min(count.saturating_sub(1));
        if is_key_pressed(KeyCode::Up) {
            self.selected = self.selected.saturating_sub(1);
        }
        if is_key_pressed(KeyCode::Down) {
            self.selected = (self.selected + 1).min(count.saturating_sub(1));
        }
        let rows = ((height - 210.) / 42.).floor().max(2.) as usize;
        let start = self.selected / rows * rows;
        let left = width * 0.57;
        for i in start..(start + rows).min(count) {
            let (item, id) = if self.selling {
                match sales[i] {
                    Sale::Weapon(id) => (
                        StockItem::Weapon(world.inventory().weapon(id).unwrap().loadout.kind),
                        Some(id),
                    ),
                    Sale::Disc(a) => (StockItem::Disc(a), None),
                }
            } else {
                (STOCK[i], None)
            };
            let rect = Rect::new(
                x + 18.,
                y + 118. + (i - start) as f32 * 42.,
                left - 30.,
                36.,
            );
            if card(rect, self.selected == i) {
                self.selected = i;
                self.notice.clear();
            }
            let label = if let Some(id) = id {
                format!("{} #{}", item.name(), id.0)
            } else {
                item.name().to_owned()
            };
            draw_text(label, rect.x + 10., rect.y + 24., 18., WHITE);
            let price = if self.selling {
                item.price() / 2
            } else {
                item.price()
            };
            let text = format!("{} M", price);
            draw_text(
                &text,
                rect.x + rect.w - measure_text(&text, None, 17, 1.0).width - 10.,
                rect.y + 24.,
                17.,
                GOLD,
            );
        }
        if count > 0 {
            let sale = if self.selling {
                Some(sales[self.selected])
            } else {
                None
            };
            let item = match sale {
                Some(Sale::Weapon(id)) => {
                    StockItem::Weapon(world.inventory().weapon(id).unwrap().loadout.kind)
                }
                Some(Sale::Disc(a)) => StockItem::Disc(a),
                None => STOCK[self.selected],
            };
            let right = x + left + 12.;
            draw_text(item.name(), right, y + 142., 23., WHITE);
            match item {
                StockItem::Weapon(w) => {
                    draw_text(
                        format!("SWORD / ATK +{}", w.attack_bonus()),
                        right,
                        y + 173.,
                        18.,
                        GOLD,
                    );
                    draw_text(w.description(), right, y + 204., 16., LIGHTGRAY);
                    draw_text(
                        "One PA slot. Use inventory to equip.",
                        right,
                        y + 230.,
                        15.,
                        LIGHTGRAY,
                    );
                }
                StockItem::Disc(a) => {
                    draw_text(
                        format!("PA DISC / {:.0} PP", a.spec().pp),
                        right,
                        y + 173.,
                        18.,
                        GOLD,
                    );
                    draw_text(
                        "Assign this disc to a sword in inventory.",
                        right,
                        y + 204.,
                        15.,
                        LIGHTGRAY,
                    );
                    draw_text(
                        if self.selling {
                            "Selling removes it from its weapon slot."
                        } else if world.inventory().discs().contains(&a) {
                            "Already owned / one copy per PA."
                        } else {
                            "Not owned / available to purchase."
                        },
                        right,
                        y + 230.,
                        15.,
                        LIGHTGRAY,
                    );
                }
            }
            draw_text(
                format!("Bag: {} / 32 items", world.inventory().item_count()),
                right,
                y + 267.,
                16.,
                LIGHTGRAY,
            );
            let price = if self.selling {
                item.price() / 2
            } else {
                item.price()
            };
            let label = format!(
                "{} for {} M / Enter",
                if self.selling { "Sell" } else { "Buy" },
                price
            );
            if button(
                &label,
                Rect::new(right, y + height - 110., width - left - 30., 38.),
            ) || is_key_pressed(KeyCode::Enter)
            {
                let result = match sale {
                    Some(Sale::Weapon(id)) => world.sell_weapon(id).map(|_| ()),
                    Some(Sale::Disc(a)) => world.sell_disc(a).map(|_| ()),
                    None => world.buy(item),
                };
                self.notice = match result {
                    Ok(()) => format!(
                        "{} {} for {} Meseta.",
                        if self.selling { "Sold" } else { "Purchased" },
                        item.name(),
                        price
                    ),
                    Err(e) => e.to_owned(),
                };
            }
        }
        draw_text(&self.notice, x + 18., y + height - 55., 16., GOLD);
        draw_text(
            "Up/Down: select   Enter: trade   Escape: close",
            x + 18.,
            y + height - 22.,
            15.,
            LIGHTGRAY,
        );
        button(
            "Leave shop",
            Rect::new(x + width - 160., y + height - 42., 140., 30.),
        )
    }
}
