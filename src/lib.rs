mod map;
mod player;
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
    prev_mouse_position: (f32, f32),
    mouse_position: (f32, f32),
}

struct Static {
    ctx: solstice_2d::solstice::Context,
    gfx: solstice_2d::Graphics,
    resources: resources::LoadedResources,
    canvas: solstice_2d::Canvas,
    input_state: InputState,
    time: std::time::Duration,
}

impl Static {
    pub fn as_ctx(&mut self) -> state::StateContext {
        state::StateContext {
            resources: &self.resources,
            ctx: &mut self.ctx,
            gfx: &mut self.gfx,
            canvas: &self.canvas,
            input_state: &self.input_state,
            time: self.time,
        }
    }
}

pub struct Game {
    shared: Static,
    game_state: state::State,
    cron: cron::Cron<()>,
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

        Ok(Self {
            shared: Static {
                ctx,
                gfx,
                resources,
                canvas,
                input_state: Default::default(),
                time,
            },
            cron,
            game_state: state::State::new(),
        })
    }

    pub fn update(&mut self, time: std::time::Duration) {
        let dt = time - self.shared.time;
        for callback in self.cron.update(dt) {
            (callback)(&mut ())
        }
        self.shared.time = time;

        self.game_state.update(dt, self.shared.as_ctx());

        for shader in self.shared.resources.shaders.iter_mut() {
            shader.send_uniform("elapsed", self.shared.time.as_secs_f32());
        }

        self.game_state.render(self.shared.as_ctx());
    }

    pub fn handle_key_event(&mut self, state: ElementState, key_code: VirtualKeyCode) {
        let pressed = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        match key_code {
            VirtualKeyCode::W => self.shared.input_state.w = pressed,
            VirtualKeyCode::A => self.shared.input_state.a = pressed,
            VirtualKeyCode::S => self.shared.input_state.s = pressed,
            VirtualKeyCode::D => self.shared.input_state.d = pressed,
            _ => {}
        };

        self.game_state
            .handle_key_event(self.shared.as_ctx(), state, key_code);
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Button(_, _) => {}
            MouseEvent::Moved(x, y) => {
                let mut is = &mut self.shared.input_state;
                if is.mouse_position == is.prev_mouse_position && is.mouse_position == (0., 0.) {
                    is.prev_mouse_position = (x, y);
                    is.mouse_position = (x, y);
                } else {
                    is.prev_mouse_position = is.mouse_position;
                    is.mouse_position = (x, y);
                }
            }
        }

        self.game_state
            .handle_mouse_event(self.shared.as_ctx(), event);
    }

    pub fn handle_resize(&mut self, width: f32, height: f32) {
        self.shared.ctx.set_viewport(0, 0, width as _, height as _);
        self.shared.gfx.set_width_height(width, height);
    }
}

mod cron {
    struct Every<T> {
        t: std::time::Duration,
        running: std::time::Duration,
        callback: Box<dyn FnMut(&mut T)>,
    }

    struct After<T> {
        triggered: bool,
        t: std::time::Duration,
        running: std::time::Duration,
        callback: Box<dyn FnMut(&mut T)>,
    }

    pub struct Cron<T> {
        t: std::time::Duration,
        every_callbacks: Vec<Every<T>>,
        after_callbacks: Vec<After<T>>,
    }

    impl<T> std::default::Default for Cron<T> {
        fn default() -> Self {
            Self {
                t: Default::default(),
                every_callbacks: vec![],
                after_callbacks: vec![],
            }
        }
    }

    impl<T> Cron<T> {
        #[allow(unused)]
        pub fn every<F>(&mut self, t: std::time::Duration, callback: F)
        where
            F: FnMut(&mut T) + 'static,
        {
            self.every_callbacks.push(Every {
                t,
                running: Default::default(),
                callback: Box::new(callback),
            });
        }

        #[allow(unused)]
        pub fn after<F>(&mut self, t: std::time::Duration, callback: F)
        where
            F: FnMut(&mut T) + 'static,
        {
            self.after_callbacks.push(After {
                triggered: false,
                t,
                running: Default::default(),
                callback: Box::new(callback),
            });
        }

        pub fn update(
            &mut self,
            dt: std::time::Duration,
        ) -> impl Iterator<Item = &mut (dyn FnMut(&mut T) + 'static)> + '_ {
            self.t += dt;

            self.every_callbacks
                .iter_mut()
                .filter_map(move |every| {
                    every.running += dt;
                    if every.running >= every.t {
                        every.running -= every.t;
                        Some(&mut *every.callback)
                    } else {
                        None
                    }
                })
                .chain(self.after_callbacks.iter_mut().filter_map(move |after| {
                    if after.triggered {
                        None
                    } else {
                        after.running += dt;
                        if after.running >= after.t {
                            after.triggered = true;
                            Some(&mut *after.callback)
                        } else {
                            None
                        }
                    }
                }))
        }
    }
}
