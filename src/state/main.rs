use super::{Map, State, StateContext};
use crate::ProgressionType;
use solstice_2d::{Color, Draw};

pub struct Main {
    pub map: Map,
    pub player: crate::player::Player,
    active_program: Option<crate::cron::ID>,
    progression: crate::MapProgression,
}

impl Main {
    pub fn new(
        ctx: &mut StateContext,
        settings: crate::MapProgression,
    ) -> Result<Self, solstice_2d::GraphicsError> {
        Self::with_seed(ctx, 0, settings)
    }

    pub fn with_seed(
        ctx: &mut StateContext,
        seed: u64,
        settings: crate::MapProgression,
    ) -> Result<Self, solstice_2d::GraphicsError> {
        let crate::map::MapGenSettings { width, height, .. } = settings.settings;
        let map = Map::with_seed(width, height, seed, ctx)?;

        let player = {
            let start = map.map.path()[0];
            let (x, y) = map.coord_to_mid_pixel(start);
            crate::player::Player::new(x, y)
        };

        Ok(Self {
            map,
            player,
            active_program: None,
            progression: settings,
        })
    }

    pub fn update(mut self, dt: std::time::Duration, mut ctx: StateContext) -> State {
        if let Some(active_program) = self.active_program {
            if !ctx.cron.contains(active_program) {
                self.active_program = None;
            }
        }

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

        {
            // player is at exit
            let grid_pos = self.map.pixel_to_coord(self.player.position());
            if let Some(target) = self.map.map.path().last().copied() {
                if grid_pos == target {
                    let seed = ctx.time.as_millis() as u64;
                    if let Some(progression) = &self.progression.exit {
                        match progression {
                            ProgressionType::Standard(settings) => {
                                if let Ok(to) =
                                    Self::with_seed(&mut ctx, seed, (**settings).clone())
                                {
                                    return State::MainToMain(super::main_to_main::MainToMain {
                                        from: self,
                                        to,
                                        time: std::time::Duration::from_secs_f32(3.),
                                        elapsed: Default::default(),
                                    });
                                }
                            }
                            ProgressionType::BadEnding => {
                                return match super::bad_end::BadEnd::new(ctx) {
                                    Ok(state) => State::BadEnd(state),
                                    Err(err) => {
                                        log::error!("Error transitioning to BadEnd: {}", err);
                                        State::Menu(super::menu::Menu::new())
                                    }
                                };
                            }
                        }
                    }
                }
            }
        }

        if cfg!(debug_assertions) {
            if ctx.input_state.ctrl && self.active_program.is_none() {
                let state = crate::programs::StateMut {
                    ctx: &mut ctx,
                    player: &mut self.player,
                    map: &mut self.map,
                };
                let r = crate::programs::NopSlide::new(state);
                self.active_program = Some(r.callback);
            }
        }

        State::Main(self)
    }

    pub fn render(&mut self, ctx: StateContext) {
        let viewport = ctx.gfx.viewport().clone();
        let map = self.map.batch.unmap(ctx.ctx);
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

        let [gw, gh] = self.map.map.grid_size();
        let [tw, th] = self.map.tile_size;
        let [pw, ph] = [gw as f32 * tw, gh as f32 * th];
        let camera_should_follow = pw > 256. || ph > 256.;

        let camera = if camera_should_follow {
            let (player_x, player_y) = self.player.position();
            let x = player_x - 256. / 2.;
            let y = player_y - 256. / 2.;

            let max_x = (pw - 256.).max(0.);
            let max_y = (ph - 256.).max(0.);

            let x = x.clamp(0., max_x);
            let y = y.clamp(0., max_y);
            solstice_2d::Transform2D::translation(-x, -y)
        } else {
            let x = 256. / 2. - pw / 2.;
            let y = 256. / 2. - ph / 2.;
            solstice_2d::Transform2D::translation(x, y)
        };
        g.set_camera(camera);
        g.image(map, &ctx.resources.sprites);

        {
            let [w, h] = self.map.tile_size;
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
                    radius: self.map.tile_size[0] / 4.,
                    segments: 4,
                },
                [0.6, 1., 0.4, 1.0],
                tx,
            );
        }

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
                ctx.canvas,
            );
        }
    }
}
