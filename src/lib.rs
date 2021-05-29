pub mod audio;
mod cron;
mod enemy;
mod map;
mod player;
mod programs;
pub mod resources;
mod state;
#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use glutin as winit;
#[cfg(target_arch = "wasm32")]
pub use winit;

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

impl InputState {
    pub fn key_states(&self) -> impl Iterator<Item = bool> {
        std::array::IntoIter::new([self.ctrl, self.space, self.w, self.a, self.s, self.d])
    }
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

pub struct AudioSinks {
    pub agent_smith_laugh: crate::audio::Sink,
    pub last_level_drone: crate::audio::Sink,
    pub level_finish: crate::audio::Sink,
    pub music: crate::audio::Sink,
    pub quote: crate::audio::Sink,
}

struct Static {
    ctx: solstice_2d::solstice::Context,
    gfx: solstice_2d::Graphics,
    resources: resources::LoadedResources,
    aesthetic_canvas: solstice_2d::Canvas,
    canvas: solstice_2d::Canvas,
    input_state: InputState,
    audio_ctx: audio::AudioContext,
    maps: MapProgression,
    time: std::time::Duration,
    sinks: Option<AudioSinks>,
}

impl Static {
    pub fn as_ctx<'a>(
        &'a mut self,
        cron: &'a mut cron::Cron<CronContext>,
    ) -> state::StateContext<'a, '_, '_> {
        state::StateContext {
            resources: &self.resources,
            g: self.gfx.lock(&mut self.ctx),
            aesthetic_canvas: &self.aesthetic_canvas,
            canvas: &self.canvas,
            input_state: &self.input_state,
            audio_ctx: &mut self.audio_ctx,
            cron,
            maps: &self.maps,
            time: self.time,
            audio_sinks: &mut self.sinks,
        }
    }
}

pub struct CronContext {
    shared: Static,
    game_state: Option<state::State>,
}

impl CronContext {
    pub fn game_state_mut(&mut self) -> &mut state::State {
        self.game_state.get_or_insert_with(state::State::default)
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

        let filter = solstice_2d::solstice::texture::Filter::new(
            solstice_2d::solstice::texture::FilterMode::Nearest,
            solstice_2d::solstice::texture::FilterMode::Nearest,
            solstice_2d::solstice::texture::FilterMode::None,
            0.0,
        );

        let aesthetic_canvas = solstice_2d::Canvas::with_settings(
            &mut ctx,
            solstice_2d::solstice::canvas::Settings {
                width: 256,
                height: 256,
                with_depth: true,
                filter,
                ..Default::default()
            },
        )?;
        let canvas = solstice_2d::Canvas::with_settings(
            &mut ctx,
            solstice_2d::solstice::canvas::Settings {
                width: 1080,
                height: 1080,
                with_depth: false,
                filter,
                ..Default::default()
            },
        )?;

        let maps = MapProgression {
            settings: map::MapGenSettings {
                width: 4,
                height: 4,
                programs: map::ProgramGenSettings { nop_slide_count: 0 },
                aesthetic: crate::AestheticShader {
                    random_shift_scale: 0.001,
                    radial_scale: 1.0,
                    ..Default::default()
                },
            },
            exit: Some(ProgressionType::Standard(Box::new(MapProgression {
                settings: map::MapGenSettings {
                    width: 8,
                    height: 8,
                    programs: map::ProgramGenSettings { nop_slide_count: 0 },
                    aesthetic: AestheticShader {
                        block_threshold: 0.093,
                        line_threshold: 0.33,
                        random_shift_scale: 0.001,
                        radial_scale: 1.0,
                        ..Default::default()
                    },
                },
                exit: Some(ProgressionType::Standard(Box::new(MapProgression {
                    settings: map::MapGenSettings {
                        width: 12,
                        height: 12,
                        programs: map::ProgramGenSettings { nop_slide_count: 0 },
                        aesthetic: AestheticShader {
                            block_threshold: 0.11,
                            line_threshold: 0.39,
                            random_shift_scale: 0.001,
                            radial_scale: 1.0,
                            ..Default::default()
                        },
                    },
                    exit: Some(ProgressionType::BadEnding),
                }))),
            }))),
        };

        // let maps = MapProgression {
        //     settings: map::MapGenSettings {
        //         width: 4,
        //         height: 4,
        //         programs: map::ProgramGenSettings { nop_slide_count: 0 },
        //         aesthetic: crate::AestheticShader::default(),
        //     },
        //     exit: Some(ProgressionType::BadEnding),
        // };

        let audio_ctx = audio::AudioContext::new();

        let mut shared = Static {
            ctx,
            gfx,
            resources,
            aesthetic_canvas,
            canvas,
            input_state: Default::default(),
            audio_ctx,
            maps,
            time,
            sinks: None,
        };
        let mut cron = cron::Cron::default();
        let game_state = Some(state::State::new(shared.as_ctx(&mut cron))?);

        Ok(Self {
            cron_ctx: CronContext { shared, game_state },
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
            .get_or_insert_with(state::State::default)
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
            .get_or_insert_with(state::State::default)
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
            .get_or_insert_with(state::State::default)
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

pub fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    return v0 + t * (v1 - v0);
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AestheticShader {
    pub block_threshold: f32,
    pub line_threshold: f32,
    pub random_shift_scale: f32,
    pub radial_scale: f32,
    pub radial_breathing_scale: f32,
    pub screen_transition_ratio: f32,
}

impl AestheticShader {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            block_threshold: lerp(self.block_threshold, other.block_threshold, t),
            line_threshold: lerp(self.line_threshold, other.line_threshold, t),
            random_shift_scale: lerp(self.random_shift_scale, other.random_shift_scale, t),
            radial_scale: lerp(self.radial_scale, other.radial_scale, t),
            radial_breathing_scale: lerp(
                self.radial_breathing_scale,
                other.radial_breathing_scale,
                t,
            ),
            screen_transition_ratio: lerp(
                self.screen_transition_ratio,
                other.screen_transition_ratio,
                t,
            ),
        }
    }
}

impl Default for AestheticShader {
    fn default() -> Self {
        Self {
            block_threshold: 0.07,
            line_threshold: 0.23,
            random_shift_scale: 0.002,
            radial_scale: 0.1,
            radial_breathing_scale: 0.01,
            screen_transition_ratio: 0.0,
        }
    }
}

impl AestheticShader {
    pub fn as_shader(&self, ctx: &resources::LoadedResources) -> solstice_2d::Shader {
        let mut shader = ctx.shaders.aesthetic.clone();
        shader.send_uniform("blockThreshold", self.block_threshold);
        shader.send_uniform("lineThreshold", self.line_threshold);
        shader.send_uniform("randomShiftScale", self.random_shift_scale);
        shader.send_uniform("radialScale", self.radial_scale);
        shader.send_uniform("radialBreathingScale", self.radial_breathing_scale);
        shader.send_uniform("screenTransitionRatio", self.screen_transition_ratio);
        let unit = 1;
        shader.bind_texture_at_location(&ctx.noise, (unit as usize).into());
        shader.send_uniform("tex1", unit);
        shader
    }
}

use quads::UVRect;
mod quads {
    use solstice_2d::Vertex2D;

    #[derive(Copy, Clone, PartialEq)]
    pub struct UVRect {
        pub positions: solstice_2d::Rectangle,
        pub uvs: solstice_2d::Rectangle,
    }

    impl UVRect {
        pub fn center_on(mut self, x: f32, y: f32) -> Self {
            self.positions.x = x - self.positions.width / 2.;
            self.positions.y = y - self.positions.height / 2.;
            self
        }

        pub fn with_size(mut self, width: f32, height: f32) -> Self {
            self.positions.width = width;
            self.positions.height = height;
            self
        }

        pub fn at_zero(mut self) -> Self {
            self.positions.x = 0.;
            self.positions.y = 0.;
            self
        }
    }

    impl From<UVRect> for solstice_2d::Geometry<'_, Vertex2D> {
        fn from(rect: UVRect) -> Self {
            let vertices = solstice_2d::solstice::quad_batch::Quad::<Vertex2D>::from(rect);
            solstice_2d::Geometry::new(vertices.vertices.to_vec(), Some(&[0u32, 1, 3, 1, 2, 3][..]))
        }
    }

    impl From<UVRect> for solstice_2d::solstice::quad_batch::Quad<Vertex2D> {
        fn from(rect: UVRect) -> Self {
            let positions =
                solstice_2d::solstice::quad_batch::Quad::<(f32, f32)>::from(rect.positions);
            let uvs = solstice_2d::solstice::quad_batch::Quad::<(f32, f32)>::from(rect.uvs);
            positions.zip(uvs).map(|((x, y), (s, t))| Vertex2D {
                position: [x, y],
                uv: [s, t],
                ..Default::default()
            })
        }
    }
}
