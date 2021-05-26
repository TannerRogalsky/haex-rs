mod bad_end;
mod main;
mod menu;
mod rotate_transition;

use crate::player::Player;
use camera::Camera;

pub struct StateContext<'a, 'b, 'c> {
    pub resources: &'a crate::resources::LoadedResources,
    pub g: solstice_2d::GraphicsLock<'b, 'c>,
    pub aesthetic_canvas: &'a solstice_2d::Canvas,
    pub canvas: &'a solstice_2d::Canvas,
    pub input_state: &'a crate::InputState,
    pub audio_ctx: &'a mut crate::audio::AudioContext,
    pub cron: &'a mut crate::cron::Cron<crate::CronContext>,
    pub maps: &'a crate::MapProgression,
    pub time: std::time::Duration,
    pub audio_sinks: &'a mut Option<crate::AudioSinks>,
}

impl StateContext<'_, '_, '_> {
    pub fn sinks(&mut self) -> &crate::AudioSinks {
        let audio_ctx = &self.audio_ctx;
        let resources = &self.resources.audio;
        self.audio_sinks.get_or_insert_with(|| crate::AudioSinks {
            agent_smith_laugh: audio_ctx
                .play_new(resources.agent_smith_laugh.clone())
                .unwrap(),
            last_level_drone: audio_ctx
                .play_new(resources.last_level_drone.clone())
                .unwrap(),
            level_finish: audio_ctx.play_new(resources.level_finish.clone()).unwrap(),
            music: audio_ctx.play_new(resources.music.clone()).unwrap(),
            quote: audio_ctx.play_new(resources.quote.clone()).unwrap(),
        })
    }
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

pub trait DrawableMap {
    fn render<'a>(&'a self, player: &'a crate::player::Player, ctx: &mut StateContext<'_, '_, 'a>);
    fn render_player<'a>(
        &'a self,
        player: &'a crate::player::Player,
        ctx: &mut StateContext<'_, '_, 'a>,
    );
    fn render_overlay<'a>(
        &'a self,
        player: &'a crate::player::Player,
        view_distance: i32,
        ctx: &mut StateContext<'_, '_, 'a>,
    );
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

fn overlay<'a>(
    ctx: &mut StateContext<'_, '_, 'a>,
    map: &'a Map,
    player: &'a crate::player::Player,
    view_distance: i32,
) {
    use solstice_2d::Draw;

    let [tw, th] = map.tile_size;
    let [half_w, half_h] = [tw / 2., th / 2.];

    let uv_bounds = ctx.resources.sprites_metadata.boss_contrast;
    let (u1, v1) = (uv_bounds.uvs.x, uv_bounds.uvs.y);
    let (uw, uh) = (uv_bounds.uvs.width, uv_bounds.uvs.height);

    let u = uw / map.grid.width as f32;
    let v = uh / map.grid.height as f32;

    let (px, py) = map.pixel_to_coord(player.position());

    let vertices = map
        .seen
        .iter()
        .filter_map(|(seen, (x, y))| {
            use solstice_2d::solstice::{quad_batch::Quad, viewport::Viewport};
            if *seen {
                let d = (px as i32 - x as i32).abs() + (py as i32 - y as i32).abs();
                if d <= view_distance {
                    None
                } else {
                    let (px, py) = map.coord_to_mid_pixel((x, y));
                    let vertices = Quad::from(Viewport::new(px - half_w, py - half_h, tw, th)).map(
                        |(x, y)| solstice_2d::Vertex2D {
                            position: [x, y],
                            color: [0.2, 0.2, 0.2, 0.7],
                            uv: [u1, v1],
                        },
                    );
                    Some(std::array::IntoIter::new(vertices.vertices))
                }
            } else {
                let (px, py) = map.coord_to_mid_pixel((x, y));
                let positions = Quad::from(Viewport::new(px - half_w, py - half_h, tw, th));
                let uvs = Quad::from(Viewport::new(u1 + u * x as f32, v1 + v * y as f32, u, v));
                let vertices = positions
                    .zip(uvs)
                    .map(|((x, y), (u, v))| solstice_2d::Vertex2D {
                        position: [x, y],
                        color: [1., 1., 1., 1.],
                        uv: [u, v],
                    });
                Some(std::array::IntoIter::new(vertices.vertices))
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    let indices = (0..(vertices.len() / 4))
        .flat_map(|i| {
            let offset = i as u32 * 4;
            std::array::IntoIter::new(solstice_2d::solstice::quad_batch::INDICES)
                .map(move |i| i as u32 + offset)
        })
        .collect::<Vec<_>>();
    let geometry = solstice_2d::Geometry::new(vertices, Some(indices));
    ctx.g.image(geometry, &ctx.resources.sprites);
}

impl DrawableMap for NavigableMap {
    fn render<'a>(
        &'a self,
        _player: &'a crate::player::Player,
        ctx: &mut StateContext<'_, '_, 'a>,
    ) {
        self.inner.draw(ctx);

        if cfg!(debug_assertions) {
            let [w, h] = self.inner.tile_size;
            self.graph.draw(w, h, &mut ctx.g);
        }

        if let Some(end) = self.graph.longest_path.last().copied() {
            use solstice_2d::Draw;
            let (x, y) = self.inner.coord_to_mid_pixel(end);
            ctx.g.draw_with_color(
                solstice_2d::Circle {
                    x,
                    y,
                    radius: self.inner.tile_size[0] / 4.,
                    segments: 20,
                },
                [0.3, 0.2, 0.8, 1.],
            );
        }
    }

    fn render_player<'a>(&'a self, player: &'a Player, ctx: &mut StateContext<'_, '_, 'a>) {
        use solstice_2d::Draw;
        let (x, y) = player.position();
        let rot = solstice_2d::Rad(ctx.time.as_secs_f32());
        let tx = solstice_2d::Transform2D::translation(x, y);
        let tx = tx * solstice_2d::Transform2D::rotation(rot);
        ctx.g.draw_with_color_and_transform(
            solstice_2d::Circle {
                x: 0.,
                y: 0.,
                radius: self.inner.tile_size[0] / 4.,
                segments: 4,
            },
            [0.6, 1., 0.4, 1.0],
            tx,
        );
    }

    fn render_overlay<'a>(
        &'a self,
        player: &'a Player,
        view_distance: i32,
        ctx: &mut StateContext<'_, '_, 'a>,
    ) {
        overlay(ctx, &self.inner, player, view_distance);
    }
}

impl DrawableMap for Map {
    fn render<'a>(
        &'a self,
        _player: &'a crate::player::Player,
        ctx: &mut StateContext<'_, '_, 'a>,
    ) {
        self.draw(ctx);
    }

    fn render_player<'a>(&'a self, player: &'a Player, ctx: &mut StateContext<'_, '_, 'a>) {
        use solstice_2d::Draw;
        let (x, y) = player.position();
        let rot = solstice_2d::Rad(ctx.time.as_secs_f32());
        let tx = solstice_2d::Transform2D::translation(x, y);
        let tx = tx * solstice_2d::Transform2D::rotation(rot);
        ctx.g.draw_with_color_and_transform(
            solstice_2d::Circle {
                x: 0.,
                y: 0.,
                radius: self.tile_size[0] / 4.,
                segments: 4,
            },
            [0.6, 1., 0.4, 1.0],
            tx,
        );
    }

    fn render_overlay<'a>(
        &'a self,
        player: &'a Player,
        view_distance: i32,
        ctx: &mut StateContext<'_, '_, 'a>,
    ) {
        overlay(ctx, self, player, view_distance);
    }
}

pub struct Map {
    pub grid: crate::map::DirectionGrid,
    pub batch: solstice_2d::solstice::quad_batch::QuadBatch<solstice_2d::Vertex2D>,
    pub tile_size: [f32; 2],
    pub seen: crate::map::Grid<bool>,
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
            &ctx.resources.sprites_metadata_raw,
        );
        let mut sp =
            solstice_2d::solstice::quad_batch::QuadBatch::new(ctx.g.ctx_mut(), batch.len())?;
        for quad in batch {
            sp.push(quad);
        }
        Ok(Map {
            grid: map,
            batch: sp,
            tile_size: [tile_width, tile_height],
            seen: crate::map::Grid {
                data: vec![false; width * height].into_boxed_slice(),
                width,
                height,
            },
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

    pub fn draw<'a>(&'a self, ctx: &mut StateContext<'_, '_, 'a>) {
        use solstice_2d::Draw;

        let [gw, gh] = self.grid.grid_size();
        let [tw, th] = self.tile_size;
        let (cw, ch) = ctx.canvas.dimensions();
        let x = cw / (gw as f32 * tw);
        let y = ch / (gh as f32 * th);
        ctx.g.set_camera(solstice_2d::Transform2D::scale(x, y));
        // self.batch.unmap(ctx.g.ctx_mut());
        ctx.g.image(self.batch.geometry(), &ctx.resources.sprites);
    }
}

pub enum State {
    Menu(menu::Menu),
    Main(main::Main),
    MainToMain(rotate_transition::RotateTransition<main::Main, main::Main>),
    BadEnd(bad_end::BadEnd),
    MainToBadEnd(rotate_transition::RotateTransition<main::Main, bad_end::BadEnd>),
}

impl std::default::Default for State {
    fn default() -> Self {
        Self::Menu(menu::Menu::new())
    }
}

impl State {
    pub fn new(_ctx: StateContext) -> Result<Self, solstice_2d::GraphicsError> {
        // Ok(Self::BadEnd(bad_end::BadEnd::new(_ctx)?))
        Ok(Self::Menu(menu::Menu::new()))
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
            State::Main(main) => {
                main.handle_key_event(ctx, state, key_code);
            }
            State::MainToMain(_) => {}
            State::BadEnd(_) => {}
            State::MainToBadEnd(_) => {}
        }
    }
}

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
            let [tw, th] = [tw * scale, th * scale];
            let [pw, ph] = [gw as f32 * tw, gh as f32 * th];
            let camera_should_follow = pw > sw || ph > sh;

            if camera_should_follow {
                let (player_x, player_y) = player.position();
                let [player_x, player_y] = [player_x * scale, player_y * scale];

                let min_x = (sw / 2. - tw).min(pw / 2.);
                let min_y = (sh / 2. - th).min(ph / 2.);

                let max_x = (pw - sw / 2. + tw).max(pw / 2.);
                let max_y = (ph - sh / 2. + th).max(ph / 2.);

                let x = player_x.clamp(min_x, max_x);
                let y = player_y.clamp(min_y, max_y);

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
