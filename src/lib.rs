mod map;
mod player;
pub mod resources;
mod state;
#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
use glutin::event::{ElementState, MouseButton, VirtualKeyCode};
#[cfg(target_arch = "wasm32")]
use winit::event::{ElementState, MouseButton, VirtualKeyCode};

use solstice_2d::{
    solstice::{self, Context},
    Color, Draw, Rectangle, Vertex2D,
};

pub enum MouseEvent {
    Button(ElementState, MouseButton),
    Moved(f32, f32),
}

#[derive(Default)]
struct InputState {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
    prev_mouse_position: (f32, f32),
    mouse_position: (f32, f32),
}

struct Map {
    map: map::Map,
    batch: solstice::quad_batch::QuadBatch<Vertex2D>,
    tile_size: (f32, f32),
}

impl Map {
    pub fn coord_to_mid_pixel(&self, coord: map::Coord) -> (f32, f32) {
        self.scale((coord.0 as f32 + 0.5, coord.1 as f32 + 0.5))
    }

    fn scale(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (x * self.tile_size.0, y * self.tile_size.1)
    }

    pub fn pixel_to_coord(&self, (x, y): (f32, f32)) -> map::Coord {
        let x = (x / self.tile_size.0).floor() as usize;
        let y = (y / self.tile_size.1).floor() as usize;
        (x, y)
    }
}

pub struct Game {
    ctx: Context,
    gfx: solstice_2d::Graphics,
    batch: solstice::quad_batch::QuadBatch<Vertex2D>,
    canvas: solstice_2d::Canvas,
    game_state: state::State,
    map: Map,
    player: player::Player,
    input_state: InputState,
    resources: resources::LoadedResources,
    time: std::time::Duration,
    cron: cron::Cron<()>,
}

impl Game {
    pub fn new(
        mut ctx: Context,
        time: std::time::Duration,
        width: f32,
        height: f32,
        resources: resources::Resources,
    ) -> eyre::Result<Self> {
        let mut gfx = solstice_2d::Graphics::new(&mut ctx, width, height)?;
        let resources = resources.try_into_loaded(&mut ctx, &mut gfx)?;

        let batch = solstice::quad_batch::QuadBatch::new(&mut ctx, 10000)?;
        let canvas = solstice_2d::Canvas::new(&mut ctx, 256., 256.)?;

        let cron = cron::Cron::default();

        let mut rng = {
            use rand::SeedableRng;
            rand::rngs::SmallRng::seed_from_u64(2)
        };

        let map = {
            let (width, height) = (10, 10);
            let tile_width = 256. / width as f32;
            let tile_height = 256. / height as f32;
            let map = map::Map::new(width, height, &mut rng);
            let batch =
                map::create_batch(tile_width, tile_height, &map, &resources.sprites_metadata);
            let mut sp = solstice::quad_batch::QuadBatch::new(&mut ctx, batch.len())?;
            for quad in batch {
                sp.push(quad);
            }
            Map {
                map,
                batch: sp,
                tile_size: (tile_width, tile_height),
            }
        };

        let player = {
            let start = map.map.path()[0];
            let (x, y) = map.coord_to_mid_pixel(start);
            player::Player::new(x, y)
        };

        Ok(Self {
            ctx,
            gfx,
            resources,
            input_state: InputState::default(),
            time,
            cron,
            batch,
            canvas,
            game_state: state::State::new(),
            map,
            player,
        })
    }

    pub fn update(&mut self, time: std::time::Duration) {
        let dt = time - self.time;
        for callback in self.cron.update(dt) {
            (callback)(&mut ())
        }
        self.time = time;

        {
            let direction = if self.input_state.w {
                Some(map::Direction::N)
            } else if self.input_state.s {
                Some(map::Direction::S)
            } else if self.input_state.a {
                Some(map::Direction::W)
            } else if self.input_state.d {
                Some(map::Direction::E)
            } else {
                None
            };

            if let Some(direction) = direction {
                let start = self.map.pixel_to_coord(self.player.position());
                if let Some(end) = self.map.map.valid_move(start, direction) {
                    let (x, y) = self.map.coord_to_mid_pixel(end);
                    let time = std::time::Duration::from_secs_f32(0.2);
                    self.player.try_move(x, y, time);
                }
            }

            self.player.update(dt);
        }

        let (width, height) = {
            let vw = self.gfx.viewport();
            (vw.width() as f32, vw.height() as f32)
        };

        self.batch.clear();
        let uvs = self
            .resources
            .sprites_metadata
            .get("boss_contrast.png")
            .unwrap();
        let quad = uvs
            .zip(solstice::quad_batch::Quad::from(
                solstice::viewport::Viewport::new(0., 0., 256., 256.),
            ))
            .map(|((s, t), (x, y))| Vertex2D {
                position: [x, y],
                uv: [s, t],
                ..Default::default()
            });
        self.batch.push(quad);
        let geometry = self.batch.unmap(&mut self.ctx);

        for shader in self.resources.shaders.iter_mut() {
            shader.send_uniform("elapsed", self.time.as_secs_f32());
        }

        let map = self.map.batch.unmap(&mut self.ctx);

        let mut g = self.gfx.lock(&mut self.ctx);
        let black = Color::new(0., 0., 0., 1.);
        g.clear(black);

        g.set_canvas(Some(self.canvas.clone()));
        g.clear(black);
        g.set_shader(Some(self.resources.shaders.menu.clone()));
        g.image(geometry, &self.resources.sprites);

        g.set_shader(None);
        g.image(map, &self.resources.sprites);
        self.map.map.draw_graph(256., 256., &mut g);

        {
            let (x, y) = self.player.position();
            let rot = solstice_2d::Rad(self.time.as_secs_f32());
            let tx = solstice_2d::Transform2D::translation(x, y);
            let tx = tx * solstice_2d::Transform2D::rotation(rot);
            g.draw_with_color_and_transform(
                solstice_2d::Circle {
                    x: 0.,
                    y: 0.,
                    radius: 5.,
                    segments: 4,
                },
                [0.6, 1., 0.4, 1.0],
                tx,
            );
        }

        g.set_canvas(None);

        g.set_shader(Some({
            let mut shader = self.resources.shaders.aesthetic.clone();
            shader.send_uniform("blockThreshold", 0.073f32);
            shader.send_uniform("lineThreshold", 0.23f32);
            shader.send_uniform("randomShiftScale", 0.002f32);
            shader.send_uniform("radialScale", 0.1f32);
            shader.send_uniform("radialBreathingScale", 0.01f32);
            let unit = 1;
            shader.bind_texture_at_location(&self.resources.noise, (unit as usize).into());
            shader.send_uniform("tex1", solstice::shader::RawUniformValue::SignedInt(unit));
            shader
        }));

        {
            let d = width.min(height);
            let x = width / 2. - d / 2.;
            g.image(
                Rectangle {
                    x,
                    y: 0.0,
                    width: d,
                    height: d,
                },
                self.canvas.clone(),
            );
        }
    }

    pub fn handle_key_event(&mut self, state: ElementState, key_code: VirtualKeyCode) {
        let pressed = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        match key_code {
            VirtualKeyCode::W => self.input_state.w = pressed,
            VirtualKeyCode::A => self.input_state.a = pressed,
            VirtualKeyCode::S => self.input_state.s = pressed,
            VirtualKeyCode::D => self.input_state.d = pressed,
            _ => {}
        };
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Button(_, _) => {}
            MouseEvent::Moved(x, y) => {
                let mut is = &mut self.input_state;
                if is.mouse_position == is.prev_mouse_position && is.mouse_position == (0., 0.) {
                    is.prev_mouse_position = (x, y);
                    is.mouse_position = (x, y);
                } else {
                    is.prev_mouse_position = is.mouse_position;
                    is.mouse_position = (x, y);
                }
            }
        }

        self.game_state.handle_mouse_event(event);
    }

    pub fn handle_resize(&mut self, width: f32, height: f32) {
        self.ctx.set_viewport(0, 0, width as _, height as _);
        self.gfx.set_width_height(width, height);
    }
}

mod cron {
    struct Every<T> {
        t: std::time::Duration,
        running: std::time::Duration,
        callback: Box<dyn FnMut(&mut T)>,
    }

    struct After<T> {
        triggered: bool,
        t: std::time::Duration,
        running: std::time::Duration,
        callback: Box<dyn FnMut(&mut T)>,
    }

    pub struct Cron<T> {
        t: std::time::Duration,
        every_callbacks: Vec<Every<T>>,
        after_callbacks: Vec<After<T>>,
    }

    impl<T> std::default::Default for Cron<T> {
        fn default() -> Self {
            Self {
                t: Default::default(),
                every_callbacks: vec![],
                after_callbacks: vec![],
            }
        }
    }

    impl<T> Cron<T> {
        #[allow(unused)]
        pub fn every<F>(&mut self, t: std::time::Duration, callback: F)
        where
            F: FnMut(&mut T) + 'static,
        {
            self.every_callbacks.push(Every {
                t,
                running: Default::default(),
                callback: Box::new(callback),
            });
        }

        #[allow(unused)]
        pub fn after<F>(&mut self, t: std::time::Duration, callback: F)
        where
            F: FnMut(&mut T) + 'static,
        {
            self.after_callbacks.push(After {
                triggered: false,
                t,
                running: Default::default(),
                callback: Box::new(callback),
            });
        }

        pub fn update(
            &mut self,
            dt: std::time::Duration,
        ) -> impl Iterator<Item = &mut (dyn FnMut(&mut T) + 'static)> + '_ {
            self.t += dt;

            self.every_callbacks
                .iter_mut()
                .filter_map(move |every| {
                    every.running += dt;
                    if every.running >= every.t {
                        every.running -= every.t;
                        Some(&mut *every.callback)
                    } else {
                        None
                    }
                })
                .chain(self.after_callbacks.iter_mut().filter_map(move |after| {
                    if after.triggered {
                        None
                    } else {
                        after.running += dt;
                        if after.running >= after.t {
                            after.triggered = true;
                            Some(&mut *after.callback)
                        } else {
                            None
                        }
                    }
                }))
        }
    }
}
