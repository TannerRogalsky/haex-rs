mod bad_end;
mod main;
// mod main_to_main;
mod menu;
mod rotate_transition;

pub struct StateContext<'a> {
    pub resources: &'a crate::resources::LoadedResources,
    pub ctx: &'a mut solstice_2d::solstice::Context,
    pub gfx: &'a mut solstice_2d::Graphics,
    pub aesthetic_canvas: &'a solstice_2d::Canvas,
    pub canvas: &'a solstice_2d::Canvas,
    pub input_state: &'a crate::InputState,
    pub audio_ctx: &'a mut crate::audio::AudioContext,
    pub cron: &'a mut crate::cron::Cron<crate::CronContext>,
    pub maps: &'a crate::MapProgression,
    pub time: std::time::Duration,
}

// pub struct MapSettings {
//     width: usize,
//     height: usize,
//     tile_width: f32,
//     tile_height: f32,
//     seed: u64,
// }

pub struct Graph {
    pub inner: crate::map::Graph,
    pub longest_path: Vec<crate::map::Coord>,
}

impl Graph {
    pub fn draw(&self, dx: f32, dy: f32, g: &mut solstice_2d::GraphicsLock) {
        let circle = solstice_2d::Circle {
            radius: dx * 0.2,
            segments: 6,
            ..Default::default()
        };
        let mut traversal = petgraph::visit::Bfs::new(&self.inner, (0, 0));
        while let Some((x, y)) = traversal.next(&self.inner) {
            let color = if self.longest_path.contains(&(x, y)) {
                [1., 1., 1., 1.]
            } else {
                [0., 0., 0., 1.]
            };

            let (tx, ty) = ((x as f32 + 0.5) * dx, (y as f32 + 0.5) * dy);
            for (nx, ny) in self.inner.neighbors((x, y)) {
                let (ntx, nty) = ((nx as f32 + 0.5) * dx, (ny as f32 + 0.5) * dy);
                g.line_2d(vec![
                    solstice_2d::LineVertex {
                        position: [tx, ty, 0.],
                        width: 2.,
                        color,
                    },
                    solstice_2d::LineVertex {
                        position: [ntx, nty, 0.],
                        width: 2.,
                        color,
                    },
                ]);
            }
            let transform = solstice_2d::Transform2D::translation(tx, ty);
            use solstice_2d::Draw;
            g.draw_with_color_and_transform(circle, color, transform);
        }
    }
}

pub struct NavigableMap {
    pub inner: Map,
    pub graph: Graph,
}

impl NavigableMap {
    pub fn with_map(map: Map) -> Self {
        let graph = map.grid.as_graph();

        fn find_dead_ends(
            graph: &crate::map::Graph,
        ) -> impl Iterator<Item = crate::map::Coord> + '_ {
            let traversal = petgraph::visit::Dfs::new(graph, (0, 0));
            petgraph::visit::Walker::iter(traversal, graph).filter_map(move |node| {
                if graph.neighbors(node).count() == 1 {
                    Some(node)
                } else {
                    None
                }
            })
        }

        let dead_ends = find_dead_ends(&graph).collect::<Vec<_>>();
        let mut longest_path = vec![];
        for from in dead_ends.iter().copied() {
            for to in dead_ends.iter().copied() {
                let path = petgraph::algo::astar(&graph, from, |node| node == to, |_| 1, |_| 0);
                if let Some((_cost, path)) = path {
                    if path.len() > longest_path.len() {
                        longest_path = path;
                    }
                }
            }
        }

        Self {
            inner: map,
            graph: Graph {
                inner: graph,
                longest_path,
            },
        }
    }
}

pub struct Map {
    pub grid: crate::map::Grid,
    pub batch: solstice_2d::solstice::quad_batch::QuadBatch<solstice_2d::Vertex2D>,
    pub tile_size: [f32; 2],
}

impl Map {
    pub fn with_seed(
        width: usize,
        height: usize,
        seed: u64,
        ctx: &mut StateContext,
    ) -> Result<Self, solstice_2d::GraphicsError> {
        let mut rng: rand::rngs::SmallRng = rand::SeedableRng::seed_from_u64(seed);
        Self::gen(width, height, ctx, &mut rng)
    }

    pub fn gen<R: rand::RngCore>(
        width: usize,
        height: usize,
        ctx: &mut StateContext,
        rng: &mut R,
    ) -> Result<Self, solstice_2d::GraphicsError> {
        let tile_width = 32.;
        let tile_height = 32.;
        let map = crate::map::Grid::new(width, height, rng);
        let batch = crate::map::create_batch(
            tile_width,
            tile_height,
            &map,
            &ctx.resources.sprites_metadata,
        );
        let mut sp = solstice_2d::solstice::quad_batch::QuadBatch::new(ctx.ctx, batch.len())?;
        for quad in batch {
            sp.push(quad);
        }
        Ok(Map {
            grid: map,
            batch: sp,
            tile_size: [tile_width, tile_height],
        })
    }

    pub fn coord_to_mid_pixel(&self, coord: crate::map::Coord) -> (f32, f32) {
        self.scale((coord.0 as f32 + 0.5, coord.1 as f32 + 0.5))
    }

    fn scale(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (x * self.tile_size[0], y * self.tile_size[1])
    }

    pub fn pixel_to_coord(&self, (x, y): (f32, f32)) -> crate::map::Coord {
        let x = (x / self.tile_size[0]).floor() as usize;
        let y = (y / self.tile_size[1]).floor() as usize;
        (x, y)
    }
}

pub enum State {
    Menu(menu::Menu),
    Main(main::Main),
    MainToMain(rotate_transition::RotateTransition<main::Main, main::Main>),
    BadEnd(bad_end::BadEnd),
    MainToBadEnd(rotate_transition::RotateTransition<main::Main, bad_end::BadEnd>),
}

impl State {
    pub fn new() -> Self {
        Self::Menu(menu::Menu::new())
    }

    pub fn update(self, dt: std::time::Duration, ctx: StateContext) -> Self {
        let ty = std::mem::discriminant(&self);
        let next = match self {
            State::Main(main) => main.update(dt, ctx),
            State::MainToMain(inner) => inner.update(dt, ctx),
            State::BadEnd(inner) => inner.update(dt, ctx),
            State::MainToBadEnd(inner) => inner.update(dt, ctx),
            _ => self,
        };
        let next_ty = std::mem::discriminant(&next);
        if ty != next_ty {
            log::debug!("State Transition: {:?} => {:?}", ty, next_ty);
        }
        next
    }

    pub fn render(&mut self, ctx: StateContext) {
        match self {
            State::Menu(menu) => menu.render(ctx),
            State::Main(main) => main.render(ctx),
            State::MainToMain(inner) => inner.render(ctx),
            State::BadEnd(inner) => inner.render(ctx),
            State::MainToBadEnd(inner) => inner.render(ctx),
        }
    }

    pub fn handle_mouse_event(&mut self, ctx: StateContext, event: crate::MouseEvent) {
        match self {
            State::Menu(inner) => {
                if let Some(new_state) = inner.handle_mouse_event(ctx, event) {
                    *self = new_state;
                }
            }
            State::Main(_) => {}
            State::MainToMain(_) => {}
            State::BadEnd(_) => {}
            State::MainToBadEnd(_) => {}
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
            State::MainToMain(_) => {}
            State::BadEnd(_) => {}
            State::MainToBadEnd(_) => {}
        }
    }
}

use camera::Camera;
mod camera {
    pub struct Camera {
        pub screen_dimension: [f32; 2],
        pub projection: solstice_2d::Perspective,
        pub transform: solstice_2d::Transform3D,
    }

    impl Camera {
        pub fn new(screen_width: f32, screen_height: f32) -> Self {
            Self {
                screen_dimension: [screen_width, screen_height],
                projection: solstice_2d::Perspective {
                    aspect: screen_width / screen_height,
                    fovy: std::f32::consts::FRAC_PI_2,
                    near: 0.1,
                    far: 1000.0,
                },
                transform: Default::default(),
            }
        }

        pub fn viewport(&mut self, x: f32, y: f32, size: f32) {
            let [w, h] = self.screen_dimension;
            let screen_size = w.max(h);
            let ratio = size / screen_size;
            let z = -0.5 / ratio;
            let x = x / w / ratio;
            let y = y / h / ratio;

            let rot = solstice_2d::Transform3D::rotation(
                solstice_2d::Rad(0.),
                solstice_2d::Rad(0.),
                solstice_2d::Rad(std::f32::consts::PI),
            );
            self.transform = solstice_2d::Transform3D::translation(x, y, z) * rot;
        }

        pub fn for_map_with_scale(
            &mut self,
            map: &crate::state::Map,
            player: &crate::player::Player,
            scale: f32,
        ) {
            let [sw, sh] = self.screen_dimension;
            let [gw, gh] = map.grid.grid_size();
            let [tw, th] = map.tile_size;
            let [pw, ph] = [gw as f32 * tw * scale, gh as f32 * th * scale];
            let camera_should_follow = pw > sw || ph > sh;

            if camera_should_follow {
                let (player_x, player_y) = player.position();

                let x = player_x.clamp(sw / 2. - tw, pw - sw / 2. + tw);
                let y = player_y.clamp(sh / 2. - th, ph - sh / 2. + th);

                let x = x - pw / 2.;
                let y = y - ph / 2.;

                self.viewport(x, y, pw.max(ph));
            } else {
                self.viewport(0., 0., pw.max(ph));
            }
        }

        pub fn for_map(&mut self, map: &crate::state::Map, player: &crate::player::Player) {
            self.for_map_with_scale(map, player, 1.);
        }
    }
}
