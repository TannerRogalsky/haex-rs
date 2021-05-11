use super::{State, StateContext};
use crate::winit::event::{ElementState, MouseButton};
use crate::MouseEvent;
use solstice::{quad_batch::Quad, viewport::Viewport};
use solstice_2d::{solstice, Color, Draw, Rectangle, Vertex2D};

#[derive(Clone)]
struct Quads<'a> {
    metadata: &'a std::collections::HashMap<String, Quad<(f32, f32)>>,
    vertices: Vec<Vertex2D>,
    count: usize,
}

impl Quads<'_> {
    fn add(&mut self, position: solstice_2d::Rectangle, name: &str) {
        if let Some(uvs) = self.metadata.get(name) {
            let quad = uvs
                .zip(Quad::from(Viewport::new(
                    position.x,
                    position.y,
                    position.width,
                    position.height,
                )))
                .map(|((s, t), (x, y))| Vertex2D {
                    position: [x, y],
                    uv: [s, t],
                    ..Default::default()
                });
            self.vertices.extend_from_slice(&quad.vertices);
            self.count += 1;
        }
    }

    fn clear(&mut self) {
        self.count = 0;
        self.vertices.clear();
    }
}

impl From<Quads<'_>> for solstice_2d::Geometry<'_, Vertex2D> {
    fn from(quads: Quads<'_>) -> Self {
        let indices = (0..quads.count)
            .flat_map(|i| {
                let offset = i as u32 * 4;
                std::array::IntoIter::new([0, 1, 2, 2, 1, 3]).map(move |i| i + offset)
            })
            .collect::<Vec<_>>();
        solstice_2d::Geometry::new(quads.vertices, Some(indices))
    }
}

pub struct Menu {
    volume_clicked: bool,
    music: Option<crate::audio::Sink>,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            volume_clicked: false,
            music: None,
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        let mut quads = Quads {
            metadata: &ctx.resources.sprites_metadata,
            vertices: Vec::with_capacity(4 * 10),
            count: 0,
        };
        quads.add(
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: 256.,
                height: 256.,
            },
            "boss_contrast.png",
        );

        let viewport = ctx.gfx.viewport().clone();
        const BLACK: Color = Color::new(0., 0., 0., 1.);

        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(BLACK);

        g.set_canvas(Some(ctx.canvas.clone()));
        g.clear(BLACK);
        g.set_shader(Some(ctx.resources.shaders.menu.clone()));
        g.image(
            solstice_2d::Geometry::from(quads.clone()),
            &ctx.resources.sprites,
        );

        g.set_shader(None);
        {
            let indices = [0, 2, 5, 3, 1, 4];
            let radius = 64.;
            let interval = std::f32::consts::TAU / indices.len() as f32;
            g.line_2d(
                (1..=(indices.len()))
                    .map(|index| {
                        let v = indices[index % indices.len()] as f32;
                        let x = (interval * v).cos() * radius + 256. / 2.;
                        let y = (interval * v).sin() * radius + 256. / 2.;
                        solstice_2d::LineVertex {
                            position: [x, y, 0.],
                            width: 6.0,
                            color: [0., 0., 0., 200. / 255.],
                        }
                    })
                    .collect::<Vec<_>>(),
            );
        }

        {
            let segments = 50;
            let radius = 50.;
            let interval = std::f32::consts::PI / segments as f32;
            let (x, y) = (256. / 2., 256. * 0.825);
            g.line_2d(
                (0..=segments)
                    .map(|index| {
                        let ratio = index as f32 / segments as f32;
                        let phi = interval * index as f32 + std::f32::consts::PI;
                        let (s, c) = phi.sin_cos();
                        let x = c * radius + x;
                        let y = s * radius + y;
                        solstice_2d::LineVertex {
                            position: [x, y, 0.],
                            width: 1.0 + ratio * 2.,
                            ..Default::default()
                        }
                    })
                    .collect::<Vec<_>>(),
            );

            let mouse_pos = Self::mouse_on_canvas(ctx.input_state.mouse_position, viewport);
            let volume = ctx.audio_ctx.global_volume();
            let volume_collider = self.volume_collider(volume, radius, (x, y));
            let color = if self.volume_clicked || collides(mouse_pos, &volume_collider) {
                [1., 0., 0., 1.]
            } else {
                [1., 1., 1., 1.]
            };
            let vertices = std::array::IntoIter::new(volume_collider)
                .map(|[x, y]| solstice_2d::Vertex2D {
                    position: [x, y],
                    color,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let indices = vec![0, 1, 2, 1, 2, 3];
            g.draw(solstice_2d::Geometry::new(vertices, Some(indices)));
        }

        quads.clear();
        quads.add(
            Rectangle {
                x: 256. / 2. - 256. / 4. / 2.,
                y: 256. * 0.65,
                width: 256. / 4.,
                height: 256. * 0.2,
            },
            "title_title.png",
        );
        quads.add(
            Rectangle {
                x: 256. / 2. - 256. / 4. / 2.,
                y: 256. * 0.825,
                width: 256. / 4.,
                height: 256. * 0.075,
            },
            "title_arrow.png",
        );
        let names = [
            "title_kyle_white.png",
            "title_ryan_white.png",
            "title_tanner_white.png",
        ];
        for (i, name) in std::array::IntoIter::new(names).enumerate() {
            let w = 256. / 3.;
            let x = w * i as f32;
            quads.add(
                Rectangle {
                    x: x,
                    y: 256. * 0.91,
                    width: w,
                    height: 256. * 0.1,
                },
                name,
            );
        }
        g.image(solstice_2d::Geometry::from(quads), &ctx.resources.sprites);

        g.set_canvas(None);
        g.set_shader(Some({
            let mut shader = ctx.resources.shaders.aesthetic.clone();
            shader.send_uniform("blockThreshold", 0.073f32);
            shader.send_uniform("lineThreshold", 0.23f32);
            shader.send_uniform("randomShiftScale", 0.002f32);
            shader.send_uniform("radialScale", 0.1f32);
            shader.send_uniform("radialBreathingScale", 0.01f32);
            let unit = 1;
            shader.bind_texture_at_location(&ctx.resources.noise, (unit as usize).into());
            shader.send_uniform("tex1", unit);
            shader
        }));

        {
            let d = viewport.width().min(viewport.height()) as f32;
            let x = viewport.width() as f32 / 2. - d / 2.;
            g.image(
                solstice_2d::Rectangle {
                    x,
                    y: 0.0,
                    width: d,
                    height: d,
                },
                ctx.canvas,
            );
        }
    }

    fn mouse_on_canvas(mouse: (f32, f32), viewport: Viewport<i32>) -> [f32; 2] {
        let d = viewport.width().min(viewport.height()) as f32;
        let x = viewport.width() as f32 / 2. - d / 2.;
        let d_ratio = 256. / d;
        let (mouse_x, mouse_y) = mouse;
        [(mouse_x - x) * d_ratio, mouse_y * d_ratio]
    }

    fn volume_collider(&self, volume: f32, radius: f32, position: (f32, f32)) -> Rect {
        let (w, h) = (3., radius * 0.15);
        let (w2, h2) = (w / 2., h / 2.);
        let (x, y) = position;
        let phi = volume * -std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
        let tx = solstice_2d::Transform2D::translation(x, y);
        let tx = tx * solstice_2d::Transform2D::rotation(solstice_2d::Rad(phi));
        let tx = tx * solstice_2d::Transform2D::translation(0., radius);
        let p0 = tx.transform_point(-w2, -h2);
        let p1 = tx.transform_point(w2, -h2);
        let p2 = tx.transform_point(-w2, h2);
        let p3 = tx.transform_point(w2, h2);
        [p0, p1, p2, p3]
    }

    pub fn handle_key_event(
        &mut self,
        mut ctx: StateContext,
        state: crate::ElementState,
        key_code: crate::VirtualKeyCode,
    ) -> Option<State> {
        if state == crate::ElementState::Released {
            match key_code {
                crate::VirtualKeyCode::W
                | crate::VirtualKeyCode::A
                | crate::VirtualKeyCode::S
                | crate::VirtualKeyCode::D => {
                    let settings = ctx.maps.clone();
                    let main = super::main::Main::new(&mut ctx, settings).ok()?;
                    Some(State::Main(main))
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn handle_mouse_event(
        &mut self,
        ctx: StateContext,
        event: crate::MouseEvent,
    ) -> Option<State> {
        match event {
            MouseEvent::Button(state, button) => {
                if state == ElementState::Pressed {
                    match button {
                        MouseButton::Left => {
                            if self.music.is_none() {
                                self.music =
                                    ctx.audio_ctx.play_new(ctx.resources.music.clone()).ok();
                                ctx.audio_ctx.set_global_volume(0.);
                            }
                            let viewport = ctx.gfx.viewport();
                            let mouse =
                                Self::mouse_on_canvas(ctx.input_state.mouse_position, *viewport);
                            let radius = 50.;
                            let volume = ctx.audio_ctx.global_volume();
                            let vc =
                                self.volume_collider(volume, radius, (256. / 2., 256. * 0.825));
                            if collides(mouse, &vc) {
                                self.volume_clicked = true;
                            }
                        }
                        _ => {}
                    }
                } else {
                    self.volume_clicked = false;
                }
            }
            MouseEvent::Moved(x, y) => {
                if self.volume_clicked {
                    let viewport = ctx.gfx.viewport();
                    let [x, _] = Self::mouse_on_canvas((x, y), *viewport);
                    let radius = 50.;
                    let center = 256. / 2.;
                    let min_x = center - radius;
                    let max_x = center + radius;
                    let x = x.clamp(min_x, max_x) - min_x;
                    ctx.audio_ctx.set_global_volume(x / (radius * 2.));
                }
            }
        }
        // Some(State::Main)
        None
    }
}

type Rect = [[f32; 2]; 4];

fn collides(p: [f32; 2], rect: &Rect) -> bool {
    type Point = [f32; 2];
    fn vec(a: Point, b: Point) -> Point {
        [b[0] - a[0], b[1] - a[1]]
    }

    fn dot(u: Point, v: Point) -> f32 {
        u[0] * v[0] + u[1] * v[1]
    }

    let ab = vec(rect[0], rect[1]);
    let am = vec(rect[0], p);
    let bc = vec(rect[1], rect[2]);
    let bm = vec(rect[1], p);

    let dot_abam = dot(ab, am);
    let dot_abab = dot(ab, ab);
    let dot_bcbm = dot(bc, bm);
    let dot_bcbc = dot(bc, bc);

    0. <= dot_abam && dot_abam <= dot_abab && 0. <= dot_bcbm && dot_bcbm <= dot_bcbc
}
