struct Moving {
    origin: (f32, f32),
    target: (f32, f32),
    time: std::time::Duration,
    elapsed: std::time::Duration,
}

struct Stationary {
    position: (f32, f32),
}

enum State {
    Stationary(Stationary),
    Moving(Moving),
}

#[derive(Default)]
pub struct Programs {
    pub nop_slide: usize,
}

pub struct Player {
    state: State,
    pub programs: Programs,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            state: State::Stationary(Stationary { position: (x, y) }),
            programs: Default::default(),
        }
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        match &mut self.state {
            State::Moving(state) => {
                state.elapsed += dt;
                if state.elapsed >= state.time {
                    self.state = State::Stationary(Stationary {
                        position: state.target,
                    })
                }
            }
            _ => {}
        }
    }

    pub fn render<C, T>(
        radius: f32,
        color: C,
        tx: T,
        ctx: &mut crate::state::StateContext,
        camera: [f32; 3],
    ) where
        C: Into<solstice_2d::Color>,
        T: Into<solstice_2d::Transform3D>,
    {
        use solstice_2d::{Draw, Rad};
        let rot = solstice_2d::Transform3D::rotation(Rad(0.), Rad(0.), Rad(ctx.time.as_secs_f32()));
        let geometry = solstice_2d::Polyhedron::octahedron(radius, 0);
        let transform = tx.into() * rot;
        let mut shader = ctx.resources.shaders.player.clone();
        shader.send_uniform(
            "lightPos",
            solstice_2d::solstice::shader::RawUniformValue::Vec3(camera.into()),
        );

        let g = &mut ctx.g;
        g.set_shader(Some(shader));
        g.draw_with_color_and_transform(geometry, color, transform);
        g.set_shader(None);
    }

    pub fn position(&self) -> (f32, f32) {
        match &self.state {
            State::Stationary(state) => state.position,
            State::Moving(state) => {
                fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
                    return v0 + t * (v1 - v0);
                }
                let ratio = state.elapsed.as_secs_f32() / state.time.as_secs_f32();
                let x = lerp(state.origin.0, state.target.0, ratio);
                let y = lerp(state.origin.1, state.target.1, ratio);
                (x, y)
            }
        }
    }

    pub fn is_moving(&self) -> bool {
        match &self.state {
            State::Stationary(_) => false,
            State::Moving(_) => true,
        }
    }

    pub fn try_move(&mut self, x: f32, y: f32, time: std::time::Duration) -> bool {
        match &mut self.state {
            State::Stationary(state) => {
                self.state = State::Moving(Moving {
                    origin: state.position,
                    target: (x, y),
                    time,
                    elapsed: Default::default(),
                });
                true
            }
            _ => false,
        }
    }
}
