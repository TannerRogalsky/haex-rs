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

        let tiles = crate::map::create_batch(64., 64., &grid, &ctx.resources.sprites_metadata);
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

        let [w, h] = [width as f32 * 64., height as f32 * 64.];
        let lh = 16. * 2.;
        let [die_x, die_y] = [w * 0.5, h * 0.7 + lh * 2.];
        fn hacker_text(_t: f32) -> String {
            "THE MAN".to_owned()
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

    pub fn render<'a>(&'a mut self, mut ctx: StateContext<'_, '_, 'a>) {
        let viewport = ctx.g.ctx_mut().viewport().clone();
        let (w, h) = ctx.aesthetic_canvas.dimensions();
        let mut camera = super::Camera::new(w, h);
        camera.for_map_with_scale(&self.map, &self.player, Self::SCALE);

        const BLACK: Color = Color::new(0., 0., 0., 1.);

        let mut quads = crate::Quads::new(&ctx.resources.sprites_metadata);
        quads.add(
            solstice_2d::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 256.,
                height: 256.,
            },
            "boss_contrast.png",
        );

        ctx.g.clear(BLACK);

        ctx.g.set_canvas(Some(ctx.canvas.clone()));
        ctx.g.clear(BLACK);

        use super::DrawableMap;
        match self.state {
            EndState::Start => {
                self.map.render(&self.player, &mut ctx);
                self.map.render_player(&self.player, &mut ctx);
                self.map.render_overlay(&self.player, 100, &mut ctx);
            }
            EndState::FadeToSequence(_) => {
                self.map.render(&self.player, &mut ctx);
                for (show, coord) in self.boss_show.iter() {
                    if *show {
                        let [tw, th] = self.map.tile_size;
                        let (px, py) = self.map.coord_to_mid_pixel(coord);
                        ctx.g.draw(solstice_2d::Rectangle {
                            x: px - tw / 2.,
                            y: py - th / 2.,
                            width: tw,
                            height: th,
                        });
                    }
                }
                self.map.render_player(&self.player, &mut ctx);
            }
            EndState::Speech => {
                self.map.render(&self.player, &mut ctx);
                self.map.render_player(&self.player, &mut ctx);
                ctx.g.set_camera(
                    solstice_2d::Transform2D::scale(3., 3.)
                        * solstice_2d::Transform2D::translation(45., 25.),
                );
                self.shodan_text.draw(&mut ctx);
            }
        }

        let g = &mut ctx.g;
        g.set_canvas(Some(ctx.aesthetic_canvas.clone()));
        g.clear(BLACK);

        g.set_shader(Some(ctx.resources.shaders.menu.clone()));
        g.image(
            solstice_2d::Geometry::from(quads.clone()),
            &ctx.resources.sprites,
        );
        g.set_shader(None);

        g.set_camera(camera.transform);
        let plane = solstice_2d::Plane::new(1., 1., 1, 1);
        g.image(plane, ctx.canvas);

        g.set_camera(solstice_2d::Transform2D::default());
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

        pub fn draw<'a>(&'a self, ctx: &mut crate::state::StateContext<'_, '_, 'a>) {
            let mut t = self.elapsed.as_secs_f32();
            for command in self.commands.iter() {
                let text = match &command.ty {
                    TextCommandType::Text(text) => {
                        std::borrow::Cow::Borrowed(&text[0..self.length_to_show(text, t)])
                    }
                    TextCommandType::Fn(f) => {
                        let text = f(t);
                        std::borrow::Cow::Owned(text[0..self.length_to_show(&text, t)].to_owned())
                    }
                };
                let [width, height] = [16. * 64., 16. * 64.];
                t = (t - text.len() as f32 / self.chars_per_sec).max(0.);
                ctx.g.print(
                    text,
                    ctx.resources.debug_font,
                    16.,
                    solstice_2d::Rectangle {
                        x: command.x / 3.,
                        y: command.y / 3.,
                        width: width / 3.,
                        height: height / 3.,
                    },
                );
            }
        }

        fn length_to_show(&self, text: &str, t: f32) -> usize {
            let len = text.len();
            let shown = (t * len as f32).floor() as usize;
            shown.min(len)
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
            ];
            let mut text = TextShower::new(1., commands);
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
