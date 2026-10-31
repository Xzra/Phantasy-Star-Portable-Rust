//! Original cached scenery and shared interface styling. No gameplay collision lives here.
use macroquad::prelude::*;

pub const ACCENT: Color = Color::new(0.22, 0.84, 0.92, 1.0);
pub const PANEL: Color = Color::new(0.025, 0.055, 0.085, 0.96);

pub fn initialize_font() {
    let font = load_ttf_font_from_bytes(include_bytes!(
        "../../../assets/fonts/LiberationSans-Regular.ttf"
    ))
    .expect("bundled Liberation Sans is valid");
    set_default_font(font);
}

pub fn panel(rect: Rect, accent: Color) {
    draw_rectangle(
        rect.x + 3.0,
        rect.y + 4.0,
        rect.w,
        rect.h,
        Color::new(0.0, 0.0, 0.0, 0.24),
    );
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::new(0.25, 0.42, 0.5, 0.65),
    );
    draw_rectangle(rect.x, rect.y, 3.0, rect.h.min(28.0), accent);
    draw_line(
        rect.x + 3.0,
        rect.y,
        rect.x + rect.w * 0.4,
        rect.y,
        1.0,
        accent,
    );
}

#[derive(Default)]
struct Geometry {
    batches: Vec<Mesh>,
}
impl Geometry {
    fn triangle(&mut self, points: [Vec3; 3], color: Color) {
        self.colored_triangle(points, [color; 3]);
    }
    fn colored_triangle(&mut self, points: [Vec3; 3], colors: [Color; 3]) {
        if self
            .batches
            .last()
            .is_none_or(|m| m.vertices.len() + 3 > 4500)
        {
            self.batches.push(Mesh {
                vertices: Vec::new(),
                indices: Vec::new(),
                texture: None,
            });
        }
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        let light = 0.48 + normal.dot(vec3(-0.35, 0.85, 0.4).normalize()).abs() * 0.52;
        let mesh = self.batches.last_mut().unwrap();
        for (p, color) in points.into_iter().zip(colors) {
            let shaded = Color::new(color.r * light, color.g * light, color.b * light, color.a);
            mesh.indices.push(mesh.vertices.len() as u16);
            mesh.vertices.push(Vertex::new2(p, Vec2::ZERO, shaded));
        }
    }
    fn box_part(&mut self, center: Vec3, size: Vec3, color: Color) {
        let points = [
            vec3(-1., -1., -1.),
            vec3(1., -1., -1.),
            vec3(1., 1., -1.),
            vec3(-1., 1., -1.),
            vec3(-1., -1., 1.),
            vec3(1., -1., 1.),
            vec3(1., 1., 1.),
            vec3(-1., 1., 1.),
        ]
        .map(|p| center + p * size * 0.5);
        for [a, b, c, d] in [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 4, 7, 3],
            [1, 2, 6, 5],
            [3, 7, 6, 2],
            [0, 1, 5, 4],
        ] {
            self.triangle([points[a], points[b], points[c]], color);
            self.triangle([points[a], points[c], points[d]], color);
        }
    }
    fn crown(&mut self, center: Vec3, size: Vec3, color: Color) {
        let p =
            [Vec3::Y, -Vec3::Y, Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z].map(|p| center + p * size);
        for [a, b, c] in [
            [0, 2, 4],
            [0, 4, 3],
            [0, 3, 5],
            [0, 5, 2],
            [1, 4, 2],
            [1, 3, 4],
            [1, 5, 3],
            [1, 2, 5],
        ] {
            self.triangle([p[a], p[b], p[c]], color);
        }
    }
    fn draw(&self) {
        for mesh in &self.batches {
            draw_mesh(mesh);
        }
    }
}

pub struct Environment {
    lobby: Geometry,
    forest: Geometry,
}
impl Default for Environment {
    fn default() -> Self {
        let mut lobby = Geometry::default();
        for z in -6..6 {
            for x in -6..6 {
                let p = vec3(x as f32 * 2.0 + 1.0, -0.12, z as f32 * 2.0 + 1.0);
                let value = if (x + z) % 2 == 0 { 0.16 } else { 0.19 };
                lobby.box_part(
                    p,
                    vec3(1.98, 0.24, 1.98),
                    Color::new(value, value + 0.055, value + 0.09, 1.0),
                );
            }
        }
        // Architecture stays outside the walkable square, including camera clearance.
        for side in [-1.0, 1.0] {
            for z in [-11.0, -3.0, 5.0, 11.0] {
                lobby.box_part(
                    vec3(side * 13.3, 3.8, z),
                    vec3(0.8, 7.6, 0.9),
                    Color::new(0.09, 0.16, 0.23, 1.0),
                );
                lobby.box_part(
                    vec3(side * 13.0, 4.0, z + 0.47),
                    vec3(0.12, 5.8, 0.05),
                    ACCENT,
                );
                lobby.box_part(
                    vec3(side * 11.5, 8.0, z),
                    vec3(4.4, 0.5, 1.0),
                    Color::new(0.16, 0.24, 0.31, 1.0),
                );
            }
        }
        for x in [-9.0, 9.0] {
            lobby.box_part(vec3(x, 0.022, 0.0), vec3(0.035, 0.02, 23.0), ACCENT);
        }
        let mut forest = Geometry::default();
        for z in -20..20 {
            for x in -20..20 {
                let corner = |dx: f32, dz: f32| {
                    let px = (x as f32 + dx) * 4.0;
                    let pz = (z as f32 + dz) * 4.0;
                    let edge = px.abs().max(pz.abs());
                    vec3(
                        px,
                        if edge < 16.0 {
                            0.014
                        } else {
                            0.014 + (px * 0.12).sin() * (pz * 0.1).cos() * 0.6
                        },
                        pz,
                    )
                };
                let color = |p: Vec3| {
                    let trail = ((p.x - (p.z * 0.18).sin() * 1.4).abs() / 5.0).clamp(0.0, 1.0);
                    let clearing = (vec2(p.x, p.z + 4.0).length() / 8.0).clamp(0.0, 1.0);
                    let grass = trail.min(clearing);
                    let v = (p.x * 1.37 + p.z * 0.79).sin() * 0.008;
                    Color::new(
                        0.32 - grass * 0.09 + v,
                        0.32 + grass * 0.07 + v,
                        0.19 + v,
                        1.0,
                    )
                };
                for points in [
                    [corner(0., 0.), corner(0., 1.), corner(1., 0.)],
                    [corner(1., 0.), corner(0., 1.), corner(1., 1.)],
                ] {
                    forest.colored_triangle(points, points.map(color));
                }
            }
        }
        for i in 0..92 {
            let a = i as f32 * 2.39996;
            let r = 22.0 + (i % 7) as f32 * 4.2;
            let p = vec3(a.cos() * r, 0.0, a.sin() * r);
            let h = 5.5 + (i % 5) as f32 * 0.85;
            let haze = ((r - 22.0) / 50.0).min(0.5);
            forest.box_part(
                p + Vec3::Y * h * 0.35,
                vec3(0.4, h * 0.7, 0.45),
                Color::new(0.2, 0.17, 0.11, 1.0),
            );
            for j in 0..3 {
                let offset = vec3(
                    (j as f32 * 2.1).cos() * 1.1,
                    h * (0.68 + j as f32 * 0.09),
                    (j as f32 * 2.1).sin(),
                );
                forest.crown(
                    p + offset,
                    vec3(2.7, 2.2, 2.6),
                    Color::new(
                        0.12 + haze * 0.25,
                        0.29 + haze * 0.2,
                        0.17 + haze * 0.3,
                        1.0,
                    ),
                );
            }
        }
        // Low foliage only: it cannot hide combat telegraphs or imply a new obstacle.
        for i in 0..280 {
            let x = (i as f32 * 7.17).sin() * 14.6;
            let z = (i as f32 * 3.13).cos() * 14.6;
            if x.abs() < 4.2 {
                continue;
            }
            let p = vec3(x, 0.02, z);
            for j in 0..3 {
                let d = vec3(
                    (j as f32 * 2.1).cos() * 0.12,
                    0.,
                    (j as f32 * 2.1).sin() * 0.12,
                );
                forest.triangle(
                    [p - d, p + Vec3::Y * (0.16 + (i % 3) as f32 * 0.035), p + d],
                    Color::new(0.29, 0.43, 0.19, 1.0),
                );
            }
        }
        Self { lobby, forest }
    }
}
impl Environment {
    pub fn draw(&self, lobby: bool) {
        if lobby {
            self.lobby.draw();
        } else {
            self.forest.draw();
        }
    }
}

pub fn shaded_box(center: Vec3, size: Vec3, color: Color) {
    let mut geometry = Geometry::default();
    geometry.box_part(center, size, color);
    geometry.draw();
}

pub fn cube(center: Vec3, size: Vec3, _texture: Option<&Texture2D>, color: Color) {
    shaded_box(center, size, color);
}
