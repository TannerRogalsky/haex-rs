use solstice_2d::{solstice, Color, Draw};

const BLACK: Color = Color::new(0., 0., 0., 1.);

pub struct StateContext<'a> {
    pub resources: &'a crate::resources::LoadedResources,
    pub ctx: &'a mut solstice_2d::solstice::Context,
    pub gfx: &'a mut solstice_2d::Graphics,
    pub canvas: &'a solstice_2d::Canvas,
    pub input_state: &'a crate::InputState,
    pub time: std::time::Duration,
}

struct Map {
    pub map: crate::map::Map,
    pub batch: solstice::quad_batch::QuadBatch<solstice_2d::Vertex2D>,
    pub tile_size: (f32, f32),
}

impl Map {
    pub fn coord_to_mid_pixel(&self, coord: crate::map::Coord) -> (f32, f32) {
        self.scale((coord.0 as f32 + 0.5, coord.1 as f32 + 0.5))
    }

    fn scale(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (x * self.tile_size.0, y * self.tile_size.1)
    }

    pub fn pixel_to_coord(&self, (x, y): (f32, f32)) -> crate::map::Coord {
        let x = (x / self.tile_size.0).floor() as usize;
        let y = (y / self.tile_size.1).floor() as usize;
        (x, y)
    }
}

pub enum State {
    Menu(Menu),
    Main(Main),
    // Over,
}

impl State {
    pub fn new() -> Self {
        Self::Menu(Menu)
    }

    pub fn update(&mut self, dt: std::time::Duration, ctx: StateContext) {
        match self {
            State::Menu(_) => {}
            State::Main(main) => main.update(dt, ctx),
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        match self {
            State::Menu(menu) => menu.render(ctx),
            State::Main(main) => main.render(ctx),
        }
    }

    pub fn handle_mouse_event(&mut self, _ctx: StateContext, event: crate::MouseEvent) {
        match self {
            State::Menu(inner) => {
                if let Some(new_state) = inner.handle_mouse_event(event) {
                    *self = new_state;
                }
            }
            State::Main(_) => {}
        }
    }

    pub fn handle_key_event(
        &mut self,
        ctx: StateContext,
        state: crate::ElementState,
        key_code: crate::VirtualKeyCode,
    ) {
        match self {
            State::Menu(menu) => {
                if let Some(new_state) = menu.handle_key_event(ctx, state, key_code) {
                    *self = new_state;
                }
            }
            State::Main(_) => {}
        }
    }
}

pub struct Main {
    map: Map,
    player: crate::player::Player,
}

impl Main {
    pub fn update(&mut self, dt: std::time::Duration, ctx: StateContext) {
        use crate::map;
        let direction = if ctx.input_state.w {
            Some(map::Direction::N)
        } else if ctx.input_state.s {
            Some(map::Direction::S)
        } else if ctx.input_state.a {
            Some(map::Direction::W)
        } else if ctx.input_state.d {
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

    pub fn render(&mut self, ctx: StateContext) {
        let viewport = ctx.gfx.viewport().clone();
        let map = self.map.batch.unmap(ctx.ctx);

        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(BLACK);

        g.set_canvas(Some(ctx.canvas.clone()));
        g.clear(BLACK);

        g.image(map, &ctx.resources.sprites);

        {
            let (w, h) = self.map.tile_size;
            self.map.map.draw_graph(w, h, &mut g);
        }

        {
            let (x, y) = self.player.position();
            let rot = solstice_2d::Rad(ctx.time.as_secs_f32());
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

        // {
        //     let fovy = std::f32::consts::FRAC_PI_2;
        //     let aspect = viewport.width() as f32 / viewport.height() as f32;
        //     g.set_projection_mode(Some(solstice_2d::Projection::Perspective(Some(
        //         solstice_2d::Perspective {
        //             aspect,
        //             fovy,
        //             near: 0.1,
        //             far: 1000.0,
        //         },
        //     ))));
        //
        //     let d = 1.;
        //     let dist = d / 2. / fovy.tan();
        //
        //     let geometry = solstice_2d::Box::new(d, d, d, 1, 1, 1);
        //     let tx = solstice_2d::Transform3D::translation(0., 0., dist - d);
        //     // let pitch = solstice_2d::Rad(self.time.as_secs_f32());
        //     // let zero = solstice_2d::Rad(0.);
        //     // let tx = tx * solstice_2d::Transform3D::rotation(zero, pitch, zero);
        //
        //     g.image_with_transform(geometry, self.canvas.clone(), tx);
        //     g.set_projection_mode(None);
        // }
    }
}

pub struct Menu;

impl Menu {
    pub fn render(&mut self, ctx: StateContext) {
        let uvs = ctx
            .resources
            .sprites_metadata
            .get("boss_contrast.png")
            .unwrap();
        let quad = uvs
            .zip(solstice::quad_batch::Quad::from(
                solstice::viewport::Viewport::new(0., 0., 256., 256.),
            ))
            .map(|((s, t), (x, y))| solstice_2d::Vertex2D {
                position: [x, y],
                uv: [s, t],
                ..Default::default()
            });

        let mut vertices = Vec::with_capacity(4);
        vertices.extend_from_slice(&quad.vertices);
        let indices = Some(vec![0, 1, 2, 2, 1, 3]);
        let quad = solstice_2d::Geometry::new(vertices, indices);

        let viewport = ctx.gfx.viewport().clone();

        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(BLACK);

        g.set_canvas(Some(ctx.canvas.clone()));
        g.clear(BLACK);
        g.set_shader(Some(ctx.resources.shaders.menu.clone()));
        g.image(quad, &ctx.resources.sprites);

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

    pub fn handle_key_event(
        &mut self,
        ctx: StateContext,
        _state: crate::ElementState,
        _key_code: crate::VirtualKeyCode,
    ) -> Option<State> {
        let mut rng = {
            use rand::SeedableRng;
            rand::rngs::SmallRng::seed_from_u64(2)
        };

        let map = {
            let (width, height) = (10, 10);
            let tile_width = 256. / width as f32;
            let tile_height = 256. / height as f32;
            let map = crate::map::Map::new(width, height, &mut rng);
            let batch = crate::map::create_batch(
                tile_width,
                tile_height,
                &map,
                &ctx.resources.sprites_metadata,
            );
            let mut sp = solstice::quad_batch::QuadBatch::new(ctx.ctx, batch.len()).ok()?;
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
            crate::player::Player::new(x, y)
        };

        Some(State::Main(Main { map, player }))
    }

    pub fn handle_mouse_event(&mut self, _event: crate::MouseEvent) -> Option<State> {
        // Some(State::Main)
        None
    }
}
