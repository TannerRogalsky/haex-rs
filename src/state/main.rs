mod ui;

use crate::{
    state::{NavigableMap, State, StateContext},
    ProgressionType,
};
use solstice_2d::{Color, Draw};
use ui::UIState;

pub struct Main {
    pub map: NavigableMap,
    pub player: crate::player::Player,
    active_program: Option<crate::cron::ID>,
    progression: crate::MapProgression,
    ui_state: UIState,
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
        let map = super::Map::with_seed(width, height, seed, ctx)?;
        let mut map = NavigableMap::with_map(map);
        map.inner.batch.unmap(ctx.g.ctx_mut());

        let player = {
            let start = map.graph.longest_path[0];
            let (x, y) = map.inner.coord_to_mid_pixel(start);
            crate::player::Player::new(x, y)
        };

        Ok(Self {
            map,
            player,
            active_program: None,
            progression: settings,
            ui_state: UIState::Closed,
        })
    }

    pub fn handle_key_event(
        &mut self,
        mut ctx: StateContext,
        state: crate::ElementState,
        key_code: crate::VirtualKeyCode,
    ) {
        let prog_state = crate::programs::StateMut {
            ctx: &mut ctx,
            player: &mut self.player,
            map: &mut self.map.inner,
        };
        if let Some(prog) = self.ui_state.handle_key_event(state, key_code, prog_state) {
            self.active_program = Some(prog);
        }
    }

    pub fn update(mut self, dt: std::time::Duration, mut ctx: StateContext) -> State {
        self.ui_state.update(dt);

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

        if !self.ui_state.is_open() {
            if let Some(direction) = direction {
                let start = self.map.inner.pixel_to_coord(self.player.position());
                if let Some(end) = self.map.inner.grid.valid_move(start, direction) {
                    let (x, y) = self.map.inner.coord_to_mid_pixel(end);
                    let time = std::time::Duration::from_secs_f32(0.2);
                    self.player.try_move(x, y, time);
                }
            }
        }

        self.player.update(dt);

        {
            let (px, py) = self.map.inner.pixel_to_coord(self.player.position());
            for x in (px.saturating_sub(2))..=(px + 2) {
                for y in (py.saturating_sub(2))..=(py + 2) {
                    let d = (px as i32 - x as i32).abs() + (py as i32 - y as i32).abs();
                    if d <= 2 {
                        if let Some(index) = self.map.inner.seen.checked_coord_to_index((x, y)) {
                            self.map.inner.seen.data[index] = true;
                        }
                    }
                }
            }
        }

        {
            // player is at exit
            let grid_pos = self.map.inner.pixel_to_coord(self.player.position());
            if let Some(target) = self.map.graph.longest_path.last().copied() {
                if grid_pos == target {
                    let seed = ctx.time.as_millis() as u64;
                    if let Some(progression) = &self.progression.exit {
                        let sound = ctx.sinks().level_finish.clone();
                        ctx.audio_ctx.play(&sound);
                        match progression {
                            ProgressionType::Standard(settings) => {
                                if let Ok(to) =
                                    Self::with_seed(&mut ctx, seed, (**settings).clone())
                                {
                                    return State::MainToMain(
                                        super::rotate_transition::RotateTransition {
                                            from: self,
                                            to,
                                            time: std::time::Duration::from_secs_f32(3.),
                                            elapsed: Default::default(),
                                        },
                                    );
                                }
                            }
                            ProgressionType::BadEnding => {
                                return match super::bad_end::BadEnd::new(ctx) {
                                    Ok(to) => State::MainToBadEnd(
                                        super::rotate_transition::RotateTransition {
                                            from: self,
                                            to,
                                            elapsed: Default::default(),
                                            time: std::time::Duration::from_secs_f32(3.),
                                        },
                                    ),
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

        self.ui_state.set_open(ctx.input_state.ctrl);
        // if cfg!(debug_assertions) {
        //     if ctx.input_state.ctrl && self.active_program.is_none() {
        //         let state = crate::programs::StateMut {
        //             ctx: &mut ctx,
        //             player: &mut self.player,
        //             map: &mut self.map.inner,
        //         };
        //         let r = crate::programs::NopSlide::new(state);
        //         self.active_program = Some(r.callback);
        //     }
        // }
        self.map.inner.batch.unmap(ctx.g.ctx_mut());

        State::Main(self)
    }

    pub fn render<'a>(&'a mut self, mut ctx: StateContext<'a, '_, 'a>) {
        let viewport = ctx.g.ctx_mut().viewport().clone();
        let (w, h) = ctx.aesthetic_canvas.dimensions();
        let mut camera = super::Camera::new(w, h);
        camera.for_map(&self.map.inner, &self.player);

        const BLACK: Color = Color::new(0., 0., 0., 1.);

        ctx.g.clear(BLACK);

        {
            ctx.g.set_canvas(Some(ctx.canvas.clone()));
            ctx.g.clear(BLACK);

            use super::DrawableMap;
            self.map.render(&self.player, &mut ctx);
            self.map.render_player(&self.player, &mut ctx);
            self.map.render_overlay(&self.player, 2, &mut ctx);

            ctx.g.set_camera(solstice_2d::Transform2D::default());
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

        self.ui_state.render(g, ctx.resources, &self.player);

        g.set_canvas(None);
        g.set_shader(Some(
            self.progression.settings.aesthetic.as_shader(ctx.resources),
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
