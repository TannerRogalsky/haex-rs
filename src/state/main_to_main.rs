use super::{main::Main, State, StateContext};

pub struct MainToMain {
    pub from: Main,
    pub to: Main,
    pub time: std::time::Duration,
    pub elapsed: std::time::Duration,
}

impl MainToMain {
    pub fn update(mut self, dt: std::time::Duration, _ctx: StateContext) -> State {
        self.elapsed += dt;
        if self.elapsed >= self.time {
            State::Main(self.to)
        } else {
            State::MainToMain(self)
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        let mut g = ctx.gfx.lock(ctx.ctx);

        g.clear([0., 0., 0., 1.]);
    }
}
