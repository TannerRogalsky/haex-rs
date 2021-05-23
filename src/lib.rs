pub mod audio;
mod cron;
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
    aesthetic_canvas: solstice_2d::Canvas,
    canvas: solstice_2d::Canvas,
    input_state: InputState,
    audio_ctx: audio::AudioContext,
    maps: MapProgression,
    time: std::time::Duration,
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

        let aesthetic_canvas = solstice_2d::Canvas::with_settings(
            &mut ctx,
            solstice_2d::solstice::canvas::Settings {
                width: 256,
                height: 256,
                with_depth: true,
                ..Default::default()
            },
        )?;
        let canvas = solstice_2d::Canvas::with_settings(
            &mut ctx,
            solstice_2d::solstice::canvas::Settings {
                width: 1080,
                height: 1080,
                with_depth: false,
                ..Default::default()
            },
        )?;

        let cron = cron::Cron::default();

        let maps = MapProgression {
            settings: map::MapGenSettings {
                width: 5,
                height: 5,
                programs: map::ProgramGenSettings { nop_slide_count: 0 },
            },
            exit: Some(ProgressionType::Standard(Box::new(MapProgression {
                settings: map::MapGenSettings {
                    width: 10,
                    height: 10,
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
        // let maps = MapProgression {
        //     settings: map::MapGenSettings {
        //         width: 5,
        //         height: 5,
        //         programs: map::ProgramGenSettings { nop_slide_count: 0 },
        //     },
        //     exit: Some(ProgressionType::BadEnding),
        // };

        let audio_ctx = audio::AudioContext::new();

        Ok(Self {
            cron_ctx: CronContext {
                shared: Static {
                    ctx,
                    gfx,
                    resources,
                    aesthetic_canvas,
                    canvas,
                    input_state: Default::default(),
                    audio_ctx,
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

use quads::Quads;
mod quads {
    use solstice_2d::{
        solstice::{quad_batch::Quad, viewport::Viewport},
        Vertex2D,
    };

    #[derive(Clone, PartialEq)]
    pub struct Quads<'a> {
        metadata: &'a std::collections::HashMap<String, Quad<(f32, f32)>>,
        vertices: Vec<Vertex2D>,
        count: usize,
    }

    impl<'a> Quads<'a> {
        pub fn new(metadata: &'a std::collections::HashMap<String, Quad<(f32, f32)>>) -> Self {
            Self {
                metadata,
                vertices: vec![],
                count: 0,
            }
        }

        pub fn add(&mut self, position: solstice_2d::Rectangle, name: &str) {
            if let Some(uvs) = self.metadata.get(name) {
                let quad = uvs
                    .zip(Quad::from(Viewport::new(
                        position.x,
                        position.y,
                        position.width,
                        position.height,
                    )))
                    .map(|((s, t), (x, y))| Vertex2D {
                        position: [x, y],
                        uv: [s, t],
                        ..Default::default()
                    });
                self.vertices.extend_from_slice(&quad.vertices);
                self.count += 1;
            }
        }

        pub fn clear(&mut self) {
            self.count = 0;
            self.vertices.clear();
        }
    }

    impl From<Quads<'_>> for solstice_2d::Geometry<'_, Vertex2D> {
        fn from(quads: Quads<'_>) -> Self {
            let indices = (0..quads.count)
                .flat_map(|i| {
                    let offset = i as u32 * 4;
                    std::array::IntoIter::new([0, 1, 2, 2, 1, 3]).map(move |i| i + offset)
                })
                .collect::<Vec<_>>();
            solstice_2d::Geometry::new(quads.vertices, Some(indices))
        }
    }
}
