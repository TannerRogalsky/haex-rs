mod cron;
mod map;
mod player;
mod programs;
pub mod resources;
mod state;
#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
use glutin as winit;
use winit::event::{ElementState, MouseButton, VirtualKeyCode};

pub enum MouseEvent {
    Button(ElementState, MouseButton),
    Moved(f32, f32),
}

#[derive(Default)]
pub struct InputState {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
    space: bool,
    ctrl: bool,
    prev_mouse_position: (f32, f32),
    mouse_position: (f32, f32),
}

#[derive(Clone)]
pub enum ProgressionType {
    Standard(Box<MapProgression>),
    BadEnding,
}

#[derive(Clone)]
pub struct MapProgression {
    settings: map::MapGenSettings,
    exit: Option<ProgressionType>,
}

struct Static {
    ctx: solstice_2d::solstice::Context,
    gfx: solstice_2d::Graphics,
    resources: resources::LoadedResources,
    canvas: solstice_2d::Canvas,
    input_state: InputState,
    maps: MapProgression,
    time: std::time::Duration,
}

impl Static {
    pub fn as_ctx<'a>(
        &'a mut self,
        cron: &'a mut cron::Cron<CronContext>,
    ) -> state::StateContext<'a> {
        state::StateContext {
            resources: &self.resources,
            ctx: &mut self.ctx,
            gfx: &mut self.gfx,
            canvas: &self.canvas,
            input_state: &self.input_state,
            cron,
            maps: &self.maps,
            time: self.time,
        }
    }
}

pub struct CronContext {
    shared: Static,
    game_state: Option<state::State>,
}

impl CronContext {
    pub fn game_state_mut(&mut self) -> &mut state::State {
        self.game_state.get_or_insert_with(state::State::new)
    }
}

pub struct Game {
    cron_ctx: CronContext,
    cron: cron::Cron<CronContext>,
}

impl Game {
    pub fn new(
        mut ctx: solstice_2d::solstice::Context,
        time: std::time::Duration,
        width: f32,
        height: f32,
        resources: resources::Resources,
    ) -> eyre::Result<Self> {
        let mut gfx = solstice_2d::Graphics::new(&mut ctx, width, height)?;
        let resources = resources.try_into_loaded(&mut ctx, &mut gfx)?;

        let canvas = solstice_2d::Canvas::new(&mut ctx, 256., 256.)?;

        let cron = cron::Cron::default();

        let maps = MapProgression {
            settings: map::MapGenSettings {
                width: 5,
                height: 5,
                programs: map::ProgramGenSettings { nop_slide_count: 0 },
            },
            exit: Some(ProgressionType::Standard(Box::new(MapProgression {
                settings: map::MapGenSettings {
                    width: 7,
                    height: 7,
                    programs: map::ProgramGenSettings { nop_slide_count: 0 },
                },
                exit: Some(ProgressionType::Standard(Box::new(MapProgression {
                    settings: map::MapGenSettings {
                        width: 10,
                        height: 10,
                        programs: map::ProgramGenSettings { nop_slide_count: 0 },
                    },
                    exit: Some(ProgressionType::BadEnding),
                }))),
            }))),
        };

        Ok(Self {
            cron_ctx: CronContext {
                shared: Static {
                    ctx,
                    gfx,
                    resources,
                    canvas,
                    input_state: Default::default(),
                    maps,
                    time,
                },
                game_state: Some(state::State::new()),
            },
            cron,
        })
    }

    pub fn update(&mut self, time: std::time::Duration) {
        let dt = time - self.cron_ctx.shared.time;
        self.cron_ctx.shared.time = time;

        self.cron.update(dt, &mut self.cron_ctx);

        self.cron_ctx.game_state = self
            .cron_ctx
            .game_state
            .take()
            .map(|state| state.update(dt, self.cron_ctx.shared.as_ctx(&mut self.cron)));

        let ctx = &mut self.cron_ctx;
        for shader in ctx.shared.resources.shaders.iter_mut() {
            shader.send_uniform("elapsed", ctx.shared.time.as_secs_f32());
        }

        ctx.game_state
            .get_or_insert_with(state::State::new)
            .render(ctx.shared.as_ctx(&mut self.cron));
    }

    pub fn handle_key_event(&mut self, state: ElementState, key_code: VirtualKeyCode) {
        let ctx = &mut self.cron_ctx;

        let pressed = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        match key_code {
            VirtualKeyCode::W => ctx.shared.input_state.w = pressed,
            VirtualKeyCode::A => ctx.shared.input_state.a = pressed,
            VirtualKeyCode::S => ctx.shared.input_state.s = pressed,
            VirtualKeyCode::D => ctx.shared.input_state.d = pressed,
            VirtualKeyCode::Space => ctx.shared.input_state.space = pressed,
            VirtualKeyCode::LControl => ctx.shared.input_state.ctrl = pressed,
            _ => {}
        };

        ctx.game_state
            .get_or_insert_with(state::State::new)
            .handle_key_event(ctx.shared.as_ctx(&mut self.cron), state, key_code);
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Button(_, _) => {}
            MouseEvent::Moved(x, y) => {
                let mut is = &mut self.cron_ctx.shared.input_state;
                if is.mouse_position == is.prev_mouse_position && is.mouse_position == (0., 0.) {
                    is.prev_mouse_position = (x, y);
                    is.mouse_position = (x, y);
                } else {
                    is.prev_mouse_position = is.mouse_position;
                    is.mouse_position = (x, y);
                }
            }
        }

        self.cron_ctx
            .game_state
            .get_or_insert_with(state::State::new)
            .handle_mouse_event(self.cron_ctx.shared.as_ctx(&mut self.cron), event);
    }

    pub fn handle_resize(&mut self, width: f32, height: f32) {
        self.cron_ctx
            .shared
            .ctx
            .set_viewport(0, 0, width as _, height as _);
        self.cron_ctx.shared.gfx.set_width_height(width, height);
    }
}
