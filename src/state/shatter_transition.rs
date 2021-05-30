use super::{black::Black, main::Main, StateContext};

pub struct ShatterTransition<FROM, TO> {
    pub from: FROM,
    pub to: TO,
    pub elapsed: std::time::Duration,
    pub time: std::time::Duration,
}

impl<FROM, TO> ShatterTransition<FROM, TO> {
    pub fn new(from: FROM, to: TO, time: std::time::Duration) -> Self {
        Self {
            from,
            to,
            elapsed: Default::default(),
            time,
        }
    }

    pub fn update_time(&mut self, dt: std::time::Duration) -> bool {
        self.elapsed += dt;
        self.elapsed >= self.time
    }
}

impl ShatterTransition<Main, Black> {
    pub fn update(mut self, dt: std::time::Duration) -> super::State {
        self.from.player.update(dt);
        if self.update_time(dt) {
            super::State::Black(self.to)
        } else {
            super::State::MainToBlack(self)
        }
    }

    pub fn render<'a>(&'a mut self, mut ctx: StateContext<'_, '_, 'a>) {
        use solstice_2d::Draw;

        let ratio = self.elapsed.as_secs_f32() / self.time.as_secs_f32();
        let mut aesthetic = self.from.progression.settings.aesthetic;
        aesthetic.screen_transition_ratio = ratio.min(1.);

        self.from.render_into_canvas(&mut ctx);
        ctx.g.set_canvas(None);
        ctx.g.set_shader(Some(aesthetic.as_shader(ctx.resources)));

        {
            let viewport = ctx.g.ctx_mut().viewport();
            let d = viewport.width().min(viewport.height()) as f32;
            let x = viewport.width() as f32 / 2. - d / 2.;
            ctx.g.image(
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
