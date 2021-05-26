use super::{State, StateContext};
use crate::map::{Direction, DirectionGrid, Grid};
use enumflags2::BitFlags;
use solstice_2d::solstice::quad_batch::QuadBatch;
use solstice_2d::{Color, Draw};

enum EndState {
    Start,
    FadeToSequence(u8),
    Speech,
}

pub struct BadEnd {
    pub map: super::Map,
    pub player: crate::player::Player,
    state: EndState,
    boss_show: Grid<bool>,
    shodan_text: text::TextShower,
}

impl BadEnd {
    pub const SCALE: f32 = 0.25;

    pub fn new(mut ctx: StateContext) -> Result<Self, solstice_2d::GraphicsError> {
        let width = 16;
        let height = 16;
        let grid = map_gen(width, height);

        let tiles = crate::map::create_batch(64., 64., &grid, &ctx.resources.sprites_metadata_raw);
        let mut batch = QuadBatch::new(ctx.g.ctx_mut(), width * height)?;
        for tile in tiles {
            batch.push(tile);
        }

        let [x, y] = [
            (width as f32 / 2. - 0.5) * 64.,
            ((height - 1) as f32 + 0.5) * 64.,
        ];
        let player = crate::player::Player::new(x, y);
        let map = super::Map {
            grid,
            batch,
            tile_size: [64., 64.],
            seen: Grid {
                data: vec![false; width * height].into_boxed_slice(),
                width,
                height,
            },
        };

        let boss_show = Grid {
            data: vec![false; width * height].into_boxed_slice(),
            width,
            height,
        };

        let [w, h] = [width as f32 * 64., height as f32 * 64. * 1.25];
        let lh = 16. * 4.;
        let [die_x, die_y] = [w * 0.5, h * 0.7 + lh * 2.];
        fn hacker_text(t: f32) -> String {
            text::lerp_string("THE MAN,", "THE HACKER,", t).to_string()
        }
        let commands = vec![
            text::TextCommand::new(w * 0.1, h * 0.1, "AND THE LORD GOD COMMANDED"),
            text::TextCommand::new(w * 0.3, h * 0.1 + lh, hacker_text as fn(_) -> _),
            text::TextCommand::new(w * 0.0, h * 0.25, "'YOU ARE FREE TO EAT"),
            text::TextCommand::new(w * 0.1, h * 0.25 + lh * 1., "FROM ANY TREE IN THE GARDEN;"),
            text::TextCommand::new(w * 0.15, h * 0.25 + lh * 3.5, "BUT YOU MUST NOT EAT FROM"),
            text::TextCommand::new(w * 0.1, h * 0.5, "THE TREE OF THE KNOWLEDGE OF"),
            text::TextCommand::new(w * 0.2, h * 0.5 + lh, "GOOD AND EVIL,"),
            text::TextCommand::new(w * 0.1, h * 0.5 + lh * 2., "FOR WHEN YOU EAT FROM IT"),
            text::TextCommand::new(w * 0.1, h * 0.7, "     YOU WILL"),
            text::TextCommand::new(w * 0.1, h * 0.7 + lh * 1., "           CERTAINLY"),
            text::TextCommand::new(w * 0.1, h * 0.7 + lh * 2., "                "),
            text::TextCommand::new(die_x, die_y, "DIE.'"),
        ];
        let shodan_text = text::TextShower::new(12., commands);

        {
            let music = ctx.sinks().music.clone();
            let drone = ctx.sinks().last_level_drone.clone();
            ctx.audio_ctx.stop(&music);
            ctx.audio_ctx.play(&drone);
        }

        Ok(Self {
            map,
            player,
            state: EndState::Start,
            boss_show,
            shodan_text,
        })
    }

    pub fn update(mut self, dt: std::time::Duration, mut ctx: StateContext) -> State {
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
            if let Some(end) = self.map.grid.valid_move(start, direction) {
                let (x, y) = self.map.coord_to_mid_pixel(end);
                let time = std::time::Duration::from_secs_f32(0.2);
                self.player.try_move(x, y, time);
            }
        }

        self.player.update(dt);

        {
            let (px, py) = self.map.pixel_to_coord(self.player.position());
            for x in (px.saturating_sub(2))..=(px + 2) {
                for y in (py.saturating_sub(2))..=(py + 2) {
                    let d = (px as i32 - x as i32).abs() + (py as i32 - y as i32).abs();
                    if d <= 2 {
                        if let Some(index) = self.map.seen.checked_coord_to_index((x, y)) {
                            self.map.seen.data[index] = true;
                        }
                    }
                }
            }
        }
        self.map.batch.unmap(ctx.g.ctx_mut());

        match &mut self.state {
            EndState::Start => {
                let all_seen = self.map.seen.data.iter().all(|v| *v);
                if all_seen {
                    self.state = EndState::FadeToSequence(0);
                }
            }
            EndState::FadeToSequence(frame) => {
                let index = feistel(*frame);
                self.boss_show.data[index as usize] = true;
                if *frame == 255 {
                    self.state = EndState::Speech;
                } else {
                    *frame = frame.saturating_add(1);
                }
            }
            EndState::Speech => {
                self.shodan_text.update(dt);
            }
        }

        if cfg!(debug_assertions) {
            if ctx.input_state.ctrl {
                for v in self.map.seen.data.iter_mut() {
                    *v = true;
                }
            }
        }

        State::BadEnd(self)
    }

    pub fn render<'a>(&'a mut self, mut ctx: StateContext<'a, '_, 'a>) {
        let viewport = ctx.g.ctx_mut().viewport().clone();
        let (w, h) = ctx.aesthetic_canvas.dimensions();
        let mut camera = super::Camera::new(w, h);
        camera.for_map_with_scale(&self.map, &self.player, Self::SCALE);

        const BLACK: Color = Color::new(0., 0., 0., 1.);

        ctx.g.clear(BLACK);

        ctx.g.set_canvas(Some(ctx.canvas.clone()));
        ctx.g.clear(BLACK);

        let full_screen = {
            let (width, height) = ctx.canvas.dimensions();
            solstice_2d::Rectangle {
                x: 0.0,
                y: 0.0,
                width,
                height,
            }
        };

        use super::DrawableMap;
        match self.state {
            EndState::Start => {
                self.map.render(&self.player, &mut ctx);
                self.map.render_player(&self.player, &mut ctx);
                ctx.g
                    .set_shader(Some(ctx.resources.shaders.vignette.clone()));
                ctx.g.draw(full_screen);
                ctx.g.set_shader(None);
                self.map.render_overlay(&self.player, 100, &mut ctx);
            }
            EndState::FadeToSequence(_) => {
                self.map.render(&self.player, &mut ctx);

                let boss = ctx.resources.sprites_metadata.boss_body;
                let boss_accent = ctx.resources.sprites_metadata.boss_color;

                for (show, coord) in self.boss_show.iter() {
                    if *show {
                        let [tw, th] = self.map.tile_size;
                        let [gw, gh] = self.map.grid.grid_size();
                        let (px, py) = self.map.coord_to_mid_pixel(coord);

                        let positions = solstice_2d::Rectangle {
                            x: px - tw / 2.,
                            y: py - th / 2.,
                            width: tw,
                            height: th,
                        };

                        let sub = |mut uvs: solstice_2d::Rectangle| {
                            let u = uvs.width / gw as f32;
                            let v = uvs.height / gh as f32;
                            uvs.x += coord.0 as f32 * u;
                            uvs.y += coord.1 as f32 * v;
                            uvs.width = u;
                            uvs.height = v;
                            uvs
                        };

                        ctx.g.draw_with_color(positions, [0., 0., 0., 1.]);
                        ctx.g.image(
                            crate::UVRect {
                                positions,
                                uvs: sub(boss.uvs),
                            },
                            &ctx.resources.sprites,
                        );
                        ctx.g.image(
                            crate::UVRect {
                                positions,
                                uvs: sub(boss_accent.uvs),
                            },
                            &ctx.resources.sprites,
                        );
                    }
                }
                self.map.render_player(&self.player, &mut ctx);
                ctx.g
                    .set_shader(Some(ctx.resources.shaders.vignette.clone()));
                ctx.g.draw(full_screen);
                ctx.g.set_shader(None);
            }
            EndState::Speech => {
                ctx.g.image(
                    crate::UVRect {
                        positions: full_screen,
                        uvs: ctx.resources.sprites_metadata.boss_body.uvs,
                    },
                    &ctx.resources.sprites,
                );
                ctx.g.image(
                    crate::UVRect {
                        positions: full_screen,
                        uvs: ctx.resources.sprites_metadata.boss_color.uvs,
                    },
                    &ctx.resources.sprites,
                );
                self.map.render_player(&self.player, &mut ctx);

                ctx.g
                    .set_shader(Some(ctx.resources.shaders.vignette.clone()));
                ctx.g.draw(full_screen);
                ctx.g.set_shader(None);

                let offset = solstice_2d::Transform2D::translation(25., 0.);
                let inner = solstice_2d::Transform2D::scale(0.9, 0.9);
                let outer = solstice_2d::Transform2D::scale(0.92, 0.92);
                ctx.g.set_camera(outer * offset);
                ctx.g.set_color([0., 0., 0., 1.]);
                self.shodan_text.draw(&mut ctx);
                ctx.g.set_camera(inner * offset);
                ctx.g.set_color(Color::from_bytes(255, 75, 50, 255));
                self.shodan_text.draw(&mut ctx);
                ctx.g.set_color([1., 1., 1., 1.]);
                ctx.g.set_camera(solstice_2d::Transform3D::default());
            }
        }

        let g = &mut ctx.g;
        g.set_canvas(Some(ctx.aesthetic_canvas.clone()));
        g.clear(BLACK);

        let full_screen = {
            let (width, height) = ctx.aesthetic_canvas.dimensions();
            solstice_2d::Rectangle {
                x: 0.0,
                y: 0.0,
                width,
                height,
            }
        };

        g.set_shader(Some(ctx.resources.shaders.menu.clone()));
        g.image(
            crate::UVRect {
                positions: full_screen,
                uvs: ctx.resources.sprites_metadata.boss_contrast.uvs,
            },
            &ctx.resources.sprites,
        );
        g.set_shader(None);

        g.set_camera(camera.transform);
        let plane = solstice_2d::Plane::new(1., 1., 1, 1);
        g.image(plane, ctx.canvas);

        g.set_camera(solstice_2d::Transform2D::default());
        g.set_canvas(None);
        g.set_shader(Some(
            crate::AestheticShader::default().as_shader(ctx.resources),
        ));

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
                ctx.aesthetic_canvas,
            );
        }
    }
}

fn map_gen(width: usize, height: usize) -> DirectionGrid {
    let all = BitFlags::from(Direction::N) | Direction::E | Direction::S | Direction::W;
    let mut data = vec![all; width * height];
    for x in 0..width {
        data[x].remove(Direction::N);
        data[x + (height - 1) * width].remove(Direction::S);
    }
    for y in 0..height {
        data[y * width].remove(Direction::W);
        data[(width - 1) + y * width].remove(Direction::E);
    }

    let mut grid = DirectionGrid {
        data: data.into_boxed_slice(),
        width,
        height,
    };

    crate::map::apply_not_corner_bit(&mut grid);

    grid
}

fn feistel(input: u8) -> u8 {
    let mut l = input & 0xf;
    let mut r = input >> 4;
    for _i in 0..8 {
        let nl = r;
        let f = (((r * 5).wrapping_add(r >> 3).wrapping_add(3 * 63)) ^ r) & 0xf;
        r = l ^ f;
        l = nl;
    }
    return ((r << 4) | l) & 0xff;
}

mod text {
    use rand::Rng;

    pub enum TextCommandType {
        Text(String),
        Fn(fn(f32) -> String),
    }

    impl From<String> for TextCommandType {
        fn from(inner: String) -> Self {
            Self::Text(inner)
        }
    }

    impl From<&str> for TextCommandType {
        fn from(inner: &str) -> Self {
            Self::Text(inner.to_owned())
        }
    }

    impl From<fn(f32) -> String> for TextCommandType {
        fn from(inner: fn(f32) -> String) -> Self {
            Self::Fn(inner)
        }
    }

    pub struct TextCommand {
        x: f32,
        y: f32,
        ty: TextCommandType,
    }

    impl TextCommand {
        pub fn new<T: Into<TextCommandType>>(x: f32, y: f32, ty: T) -> Self {
            Self {
                x,
                y,
                ty: ty.into(),
            }
        }
    }

    #[derive(PartialEq, Debug)]
    struct TextSection<'a> {
        text: std::borrow::Cow<'a, str>,
        x: f32,
        y: f32,
    }

    pub struct TextShower {
        chars_per_sec: f32,
        pub time: std::time::Duration,
        elapsed: std::time::Duration,
        commands: Vec<TextCommand>,
    }

    impl TextShower {
        pub fn new(chars_per_sec: f32, commands: Vec<TextCommand>) -> Self {
            let time = commands
                .iter()
                .fold(std::time::Duration::default(), |time, command| {
                    let len = match &command.ty {
                        TextCommandType::Text(text) => text.len(),
                        TextCommandType::Fn(f) => f(1.).len(),
                    };
                    time + std::time::Duration::from_secs_f32(len as f32 / chars_per_sec)
                });
            Self {
                chars_per_sec,
                time,
                elapsed: Default::default(),
                commands,
            }
        }

        pub fn update(&mut self, dt: std::time::Duration) {
            self.elapsed += dt;
        }

        fn to_show(&self) -> impl Iterator<Item = TextSection> + '_ {
            let mut t = self.elapsed.as_secs_f32();
            self.commands.iter().filter_map(move |command| {
                let text = match &command.ty {
                    TextCommandType::Text(text) => {
                        std::borrow::Cow::Borrowed(&text[0..self.length_to_show(text, t)])
                    }
                    TextCommandType::Fn(f) => {
                        let text = f(t);
                        std::borrow::Cow::Owned(text[0..self.length_to_show(&text, t)].to_owned())
                    }
                };
                t = (t - text.len() as f32 / self.chars_per_sec).max(0.);
                if text.len() > 0 {
                    Some(TextSection {
                        text,
                        x: command.x,
                        y: command.y,
                    })
                } else {
                    None
                }
            })
        }

        pub fn draw<'a>(&'a self, ctx: &mut crate::state::StateContext<'_, '_, 'a>) {
            let [width, height] = [16. * 64., 16. * 64.];
            for section in self.to_show() {
                ctx.g.print(
                    section.text,
                    ctx.resources.pixel_font,
                    16. * 4.,
                    solstice_2d::Rectangle {
                        x: section.x,
                        y: section.y,
                        width,
                        height,
                    },
                );
            }
        }

        fn length_to_show(&self, text: &str, t: f32) -> usize {
            let shown = (t * self.chars_per_sec).floor() as usize;
            shown.min(text.len())
        }
    }

    pub fn lerp_string<'a>(from: &'a str, to: &'a str, ratio: f32) -> std::borrow::Cow<'a, str> {
        if ratio <= 0. {
            return from.into();
        }
        if ratio >= 1. {
            return to.into();
        }

        let uppercase_ascii = 65u8..=90;
        let mut r: rand::rngs::SmallRng = rand::SeedableRng::seed_from_u64((ratio * 80.) as u64);

        if ratio < (1. / 3.) {
            let to_change = (from.len() as f32 * ratio * 3.).floor() as usize;
            let mut s = String::new();
            for _i in 0..to_change {
                s.push(r.gen_range(uppercase_ascii.clone()) as char);
            }
            s.push_str(&from[to_change..]);
            s.into()
        } else if ratio < (2. / 3.) {
            let length = from.len()
                + ((to.len() as f32 - from.len() as f32) * (ratio * 3.).fract()).floor() as usize;
            (0..length)
                .map(|_| r.gen_range(uppercase_ascii.clone()) as char)
                .collect::<String>()
                .into()
        } else {
            let to_change = (to.len() as f32 * (ratio * 3.).fract()).floor() as usize;
            let mut s = String::new();
            s.push_str(&to[0..to_change]);
            for _ in to_change..to.len() {
                s.push(r.gen_range(uppercase_ascii.clone()) as char);
            }
            s.into()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn basic_test() {
            let commands = vec![
                TextCommand {
                    x: 0.0,
                    y: 0.0,
                    ty: TextCommandType::Text("This is a test.".to_string()),
                },
                TextCommand {
                    x: 10.0,
                    y: 230.0,
                    ty: TextCommandType::Fn(|t| {
                        let v = "a moving target";
                        let l = ((v.len() as f32 * t).floor() as usize).clamp(0, v.len());
                        v[0..l].to_string()
                    }),
                },
                TextCommand {
                    x: 1.,
                    y: 2.,
                    ty: TextCommandType::Fn(|t| format!("t: {:.2}", t)),
                },
            ];
            let one_sec = std::time::Duration::from_secs(1);
            let mut text = TextShower::new(1., commands);
            assert_eq!(
                text.length_to_show("This is a test", one_sec.as_secs_f32()),
                1
            );
            assert_eq!(text.to_show().count(), 0);

            text.update(one_sec);
            assert_eq!(
                text.to_show().next(),
                Some(TextSection {
                    text: "T".into(),
                    x: 0.0,
                    y: 0.0
                })
            );
            text.update(one_sec * 14);
            {
                let mut iter = text.to_show();
                assert_eq!(
                    iter.next(),
                    Some(TextSection {
                        text: "This is a test.".into(),
                        x: 0.0,
                        y: 0.0
                    })
                );
                assert_eq!(iter.next(), None);
            }

            text.update(one_sec);
            {
                let mut iter = text.to_show();
                assert!(iter.next().is_some());
                assert_eq!(
                    iter.next(),
                    Some(TextSection {
                        text: "a".into(),
                        x: 10.,
                        y: 230.,
                    })
                );
                assert!(iter.next().is_none());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_gen_test() {
        let map = map_gen(3, 3);
        assert_eq!(map.data[4], BitFlags::all());
    }

    #[test]
    fn feistel_test() {
        // fn feistel(input: u16) -> u16 {
        //     let mut l = input & 0xff;
        //     let mut r = input >> 8;
        //     for _i in 0..8 {
        //         let nl = r;
        //         let f = (((r * 11) + (r >> 5) + 7 * 127) ^ r) & 0xff;
        //         r = l ^ f;
        //         l = nl;
        //     }
        //     return ((r << 8) | l) & 0xffff;
        // }

        let v = (0..=255)
            .map(|i| feistel(i))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(v.len(), 256);
        let mut v = v.into_iter().peekable();
        while let Some(i) = v.next() {
            if let Some(next) = v.peek().copied() {
                assert_eq!(i + 1, next);
            }
        }
    }
}
