mod bad_end;
mod black;
mod main;
mod menu;
mod rotate_transition;
mod shatter_transition;

use crate::player::Player;
use camera::Camera;
use std::option::Option::None;

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
    fn render<'a>(&'a self, player: &Player, ctx: &mut StateContext<'_, '_, 'a>);
    fn render_player(&self, player: &Player, ctx: &mut StateContext<'_, '_, '_>) {
        self.render_player_with_transform(player, solstice_2d::Transform3D::default(), ctx);
    }
    fn render_player_with_transform(
        &self,
        player: &Player,
        tx: solstice_2d::Transform3D,
        ctx: &mut StateContext<'_, '_, '_>,
    );
    fn render_overlay<'a>(
        &'a self,
        player: &Player,
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

        let longest_path = {
            let dead_ends = find_dead_ends(&graph).collect::<Vec<_>>();
            let couples = dead_ends[..dead_ends.len() - 1]
                .iter()
                .copied()
                .enumerate()
                .flat_map(|(index, start)| {
                    dead_ends[(index + 1)..]
                        .iter()
                        .copied()
                        .map(move |end| (start, end))
                });
            let mut longest_path = vec![];
            for (from, to) in couples {
                let path = petgraph::algo::astar(&graph, from, |node| node == to, |_| 1, |_| 0);
                if let Some((_cost, path)) = path {
                    if path.len() > longest_path.len() {
                        longest_path = path;
                    }
                }
            }
            longest_path
        };

        Self {
            inner: map,
            graph: Graph {
                inner: graph,
                longest_path,
            },
        }
    }

    pub fn get_enemy_spawns<'a, R: rand::Rng>(
        &'a self,
        max: usize,
        player: &Player,
        rng: &'a mut R,
    ) -> enemy_spawn::SpawnIterator<impl FnMut() -> Option<(usize, usize)> + 'a> {
        let player_exclusion_size = 2;
        let player_exclusion = {
            let (x, y) = self.inner.pixel_to_coord(player.position());
            let x1 = x.saturating_sub(player_exclusion_size);
            let y1 = y.saturating_sub(player_exclusion_size);
            let x2 = (x + player_exclusion_size).max(self.inner.grid.width - 1);
            let y2 = (y + player_exclusion_size).max(self.inner.grid.height - 1);
            [x1, y1, x2, y2]
        };
        let valid_count = self.inner.grid.width * self.inner.grid.height;

        let is_excluded = move |(x, y): (usize, usize)| -> bool {
            let [x1, y1, x2, y2] = player_exclusion;
            x1 <= x && x < x2 && y1 <= y && y < y2
        };

        let mut iter = enemy_spawn::bag_random(valid_count, rng);
        enemy_spawn::SpawnIterator::new(max, move || {
            while let Some(next) = iter.next() {
                let coord = self.inner.grid.index_to_coord(next);
                if !is_excluded(coord) {
                    return Some(coord);
                }
            }
            None
        })
    }
}

mod enemy_spawn {
    pub fn bag_random<R: rand::Rng>(count: usize, rng: &mut R) -> impl Iterator<Item = usize> {
        let prime = (count..).find(|n| is_prime(*n)).unwrap();

        let skip = {
            let mut skip = 0;
            while skip % prime == 0 {
                let a = rng.gen_range(0..prime) + 1;
                let b = rng.gen_range(0..prime) + 1;
                let c = rng.gen_range(0..prime) + 1;
                skip = a * (count * count) + b * count + c;
            }
            skip
        };

        let mut next = 0;
        let mut returned = 0;
        std::iter::from_fn(move || {
            if returned >= count {
                None
            } else {
                loop {
                    next += skip;
                    next %= prime;

                    if next < count {
                        break;
                    }
                }
                returned += 1;
                Some(next)
            }
        })
    }

    fn is_prime(n: usize) -> bool {
        if n <= 1 {
            return false;
        }
        if n <= 3 {
            return true;
        }

        // This is checked so that we can skip
        // middle five numbers in below loop
        if n % 2 == 0 || n % 3 == 0 {
            return false;
        }

        let mut i = 5;
        while i * i < n {
            if n % i == 0 || n % (i + 2) == 0 {
                return false;
            }
            i += 6;
        }

        true
    }

    pub struct SpawnIterator<F> {
        max: usize,
        inner: std::iter::Take<std::iter::FromFn<F>>,
    }

    impl<T, F> SpawnIterator<F>
    where
        F: FnMut() -> Option<T>,
    {
        pub fn new(max: usize, inner: F) -> Self {
            Self {
                max,
                inner: std::iter::from_fn(inner).take(max),
            }
        }
    }

    impl<T, F> std::iter::FusedIterator for SpawnIterator<F> where F: FnMut() -> Option<T> {}

    impl<T, F> std::iter::Iterator for SpawnIterator<F>
    where
        F: FnMut() -> Option<T>,
    {
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, Some(self.max))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn bag_random_test() {
            let mut rng: rand::rngs::SmallRng = rand::SeedableRng::seed_from_u64(0);
            let count = 50;
            let v = bag_random(count, &mut rng).collect::<std::collections::BTreeSet<_>>();
            assert_eq!(v.len(), count);
            assert_eq!(v.iter().next(), Some(&0));
            assert_eq!(v.iter().last(), Some(&(count - 1)));
        }
    }
}

fn overlay(ctx: &mut StateContext<'_, '_, '_>, map: &Map, player: &Player, view_distance: i32) {
    use solstice_2d::Draw;

    let [tw, th] = map.tile_size;
    let [half_w, half_h] = [tw / 2., th / 2.];

    let uv_bounds = ctx.resources.sprites_metadata.boss_contrast;
    let (u1, v1) = (uv_bounds.uvs.x, uv_bounds.uvs.y);
    let (uw, uh) = (uv_bounds.uvs.width, uv_bounds.uvs.height);

    let u = uw / map.grid.width as f32;
    let v = uh / map.grid.height as f32;

    let (px, py) = map.pixel_to_coord(player.position());

    let vertices = map.seen.iter().filter_map(|(seen, (x, y))| {
        use solstice_2d::solstice::{quad_batch::Quad, viewport::Viewport};
        if *seen {
            let d = (px as i32 - x as i32).abs() + (py as i32 - y as i32).abs();
            if d <= view_distance {
                None
            } else {
                let (px, py) = map.coord_to_mid_pixel((x, y));
                let vertices = Quad::from(Viewport::new(px - half_w, py - half_h, tw, th))
                    .map(|(x, y)| solstice_2d::Vertex2D {
                        position: [x, y],
                        color: [0.1, 0.1, 0.1, 0.7],
                        ..Default::default()
                    });
                Some(std::array::IntoIter::new(vertices.vertices))
            }
        } else {
            None
        }
    }).flatten().collect::<Vec<_>>();
    let indices = (0..(vertices.len() / 4))
        .flat_map(|i| {
            let offset = i as u32 * 4;
            std::array::IntoIter::new(solstice_2d::solstice::quad_batch::INDICES)
                .map(move |i| i as u32 + offset)
        })
        .collect::<Vec<_>>();
    let geometry = solstice_2d::Geometry::new(vertices, Some(indices));
    ctx.g.set_shader(None);
    ctx.g.draw(geometry);

    let vertices =
        map.seen
            .iter()
            .filter_map(|(seen, (x, y))| {
                use solstice_2d::solstice::{quad_batch::Quad, viewport::Viewport};
                if *seen {
                    None
                } else {
                    let (px, py) = map.coord_to_mid_pixel((x, y));
                    let positions = Quad::from(Viewport::new(px - half_w, py - half_h, tw, th));
                    let uvs = Quad::from(Viewport::new(u1 + u * x as f32, v1 + v * y as f32, u, v));
                    let quad_uvs = Quad::from(Viewport::new(0., 0., 1., 1.));
                    let vertices =
                        positions
                            .zip(uvs)
                            .zip(quad_uvs)
                            .map(|(((x, y), (u, v)), (u1, v1))| solstice_2d::Vertex2D {
                                position: [x, y],
                                color: [u1, v1, 1., 1.],
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
    let mut shader = ctx.resources.shaders.map_obscuring.clone();
    shader.send_uniform(
        "grid_dimensions",
        solstice_2d::solstice::shader::RawUniformValue::Vec2(
            [map.grid.width as f32, map.grid.height as f32].into(),
        ),
    );
    ctx.g.set_shader(Some(shader));
    ctx.g.image(geometry, &ctx.resources.sprites);
    ctx.g.set_shader(None);
}

impl DrawableMap for NavigableMap {
    fn render<'a>(&'a self, _player: &Player, ctx: &mut StateContext<'_, '_, 'a>) {
        self.inner.draw(ctx);

        if cfg!(debug_assertions) {
            let [w, h] = self.inner.tile_size;
            self.graph.draw(w, h, &mut ctx.g);
        }

        if let Some(end) = self.graph.longest_path.last().copied() {
            use solstice_2d::Draw;
            let (x, y) = self.inner.coord_to_mid_pixel(end);
            let [tw, th] = self.inner.tile_size;
            let [width, height] = [tw / 2., th / 2.];

            let pos = solstice_2d::Transform2D::translation(x, y);
            let rot = solstice_2d::Transform2D::rotation(solstice_2d::Rad(ctx.time.as_secs_f32()));

            ctx.g.image(
                ctx.resources
                    .sprites_metadata
                    .exit_body
                    .with_size(width, height)
                    .center_on(x, y),
                &ctx.resources.sprites,
            );
            ctx.g.image_with_color_and_transform(
                ctx.resources
                    .sprites_metadata
                    .exit_alpha
                    .with_size(width, height)
                    .center_on(0., 0.),
                &ctx.resources.sprites,
                [0.2, 1., 0.4, 1.],
                pos * rot,
            );
        }
    }

    fn render_player_with_transform(
        &self,
        player: &Player,
        tx: solstice_2d::Transform3D,
        ctx: &mut StateContext<'_, '_, '_>,
    ) {
        self.inner.render_player_with_transform(player, tx, ctx);
    }

    fn render_overlay<'a>(
        &'a self,
        player: &Player,
        view_distance: i32,
        ctx: &mut StateContext<'_, '_, 'a>,
    ) {
        overlay(ctx, &self.inner, player, view_distance);
    }
}

pub fn player_tx(player: &Player, map: &crate::state::Map) -> solstice_2d::Transform3D {
    let (px, py) = player.position();
    let [tw, _] = map.tile_size;
    let [width, height] = map.pixel_dimensions();
    let scale = 1. / width.max(height);
    let radius = scale * tw / 4.;
    let x = (px - width / 2.) * scale;
    let y = (py - height / 2.) * scale;

    solstice_2d::Transform3D::translation(x, y, radius)
}

impl DrawableMap for Map {
    fn render<'a>(&'a self, _player: &Player, ctx: &mut StateContext<'_, '_, 'a>) {
        self.draw(ctx);
    }

    fn render_player_with_transform(
        &self,
        player: &Player,
        tx: solstice_2d::Transform3D,
        ctx: &mut StateContext,
    ) {
        let [tw, _] = self.tile_size;
        let [width, height] = self.pixel_dimensions();
        let scale = 1. / width.max(height);
        let radius = scale * tw / 4.;

        let mut camera = Camera::new(256., 256.);
        camera.for_map(self, player);

        Player::render(
            radius,
            [1., 0., 0., 1.],
            tx * player_tx(player, self),
            ctx,
            camera.transform.inverse_transform_point(0., 0., 0.),
        );
    }

    fn render_overlay<'a>(
        &'a self,
        player: &Player,
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

    pub fn pixel_dimensions(&self) -> [f32; 2] {
        let [tw, th] = self.tile_size;
        [tw * self.grid.width as f32, th * self.grid.height as f32]
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

        let shader = ctx.resources.shaders.grayscale.clone();
        ctx.g.set_camera(solstice_2d::Transform2D::scale(x, y));
        ctx.g.set_shader(Some(shader));
        ctx.g.image(self.batch.geometry(), &ctx.resources.sprites);
        ctx.g.set_shader(None);
    }
}

pub enum State {
    Menu(menu::Menu),
    Main(main::Main),
    MainToMain(rotate_transition::RotateTransition<main::Main, main::Main>),
    MainToBlack(shatter_transition::ShatterTransition<main::Main, black::Black>),
    BadEnd(bad_end::BadEnd),
    MainToBadEnd(rotate_transition::RotateTransition<main::Main, bad_end::BadEnd>),
    Black(black::Black),
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

    pub fn update(mut self, dt: std::time::Duration, ctx: StateContext) -> Self {
        let ty = std::mem::discriminant(&self);
        let next = match self {
            State::Main(main) => main.update(dt, ctx),
            State::MainToMain(inner) => inner.update(dt, ctx),
            State::BadEnd(inner) => inner.update(dt, ctx),
            State::MainToBadEnd(inner) => inner.update(dt, ctx),
            State::Menu(_) => self,
            State::MainToBlack(inner) => inner.update(dt),
            State::Black(ref mut inner) => {
                inner.update(dt);
                self
            }
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
            State::MainToBlack(inner) => inner.render(ctx),
            State::Black(_) => {}
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
            State::MainToBlack(_) => {}
            State::Black(_) => {}
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
            State::BadEnd(inner) => {
                if let Some(new_state) = inner.handle_key_event(ctx, state, key_code) {
                    *self = new_state;
                }
            }
            State::MainToBadEnd(_) => {}
            State::MainToBlack(_) => {}
            State::Black(inner) => {
                if let Some(new_state) = inner.handle_key_event(state, key_code) {
                    *self = new_state;
                }
            }
        }
    }
}

mod camera {
    use super::{Map, Player};

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

        pub fn should_follow(&self, map: &Map, scale: f32) -> bool {
            let [sw, sh] = self.screen_dimension;
            let [gw, gh] = map.grid.grid_size();
            let [tw, th] = map.tile_size;
            let [tw, th] = [tw * scale, th * scale];
            let [pw, ph] = [gw as f32 * tw, gh as f32 * th];
            pw >= sw || ph >= sh
        }

        pub fn for_map_with_scale(&mut self, map: &Map, player: &Player, scale: f32) {
            let camera_should_follow = self.should_follow(map, scale);
            self.for_map_with_scale_and_follow(map, player, scale, camera_should_follow)
        }

        pub fn for_map_with_scale_and_follow(
            &mut self,
            map: &Map,
            player: &Player,
            scale: f32,
            camera_should_follow: bool,
        ) {
            let [sw, sh] = self.screen_dimension;
            let [gw, gh] = map.grid.grid_size();
            let [tw, th] = map.tile_size;
            let [tw, th] = [tw * scale, th * scale];
            let [pw, ph] = [gw as f32 * tw, gh as f32 * th];

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

        pub fn for_map(&mut self, map: &Map, player: &Player) {
            self.for_map_with_scale(map, player, 1.);
        }
    }
}
