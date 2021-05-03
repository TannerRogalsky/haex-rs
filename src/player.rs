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

pub struct Player {
    state: State,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            state: State::Stationary(Stationary { position: (x, y) }),
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
