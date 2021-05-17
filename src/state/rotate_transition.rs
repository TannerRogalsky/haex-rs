use super::{bad_end::BadEnd, main::Main, State, StateContext};
use crate::{player::Player, state::Map};
use solstice_2d::{Color, Draw};

fn render(ctx: StateContext, ratio: f32, states: [(&mut Map, &Player); 2]) {
    let viewport = ctx.gfx.viewport().clone();

    let (w, h) = ctx.aesthetic_canvas.dimensions();
    let mut from_camera = super::Camera::new(w, h);
    from_camera.for_map(&states[0].0, &states[0].1);
    let mut to_camera = super::Camera::new(w, h);
    to_camera.for_map(&states[1].0, &states[1].1);

    let camera = from_camera
        .transform
        .lerp_slerp(&to_camera.transform, ratio);

    const BLACK: Color = Color::new(0., 0., 0., 1.);

    let aesthetic_shader = {
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
    };

    {
        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(BLACK);

        g.set_canvas(Some(ctx.aesthetic_canvas.clone()));
        g.clear(BLACK);
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
        g.set_shader(Some(ctx.resources.shaders.menu.clone()));
        g.image(
            solstice_2d::Geometry::from(quads.clone()),
            &ctx.resources.sprites,
        );
        g.set_canvas(None);
    }

    for (index, (map, player)) in std::array::IntoIter::new(states).enumerate() {
        let geometry = map.batch.unmap(ctx.ctx);
        let mut g = ctx.gfx.lock(ctx.ctx);

        {
            g.set_canvas(Some(ctx.canvas.clone()));
            g.clear(BLACK);

            let [gw, gh] = map.grid.grid_size();
            let [tw, th] = map.tile_size;
            let (cw, ch) = ctx.canvas.dimensions();
            let x = cw / (gw as f32 * tw);
            let y = ch / (gh as f32 * th);
            g.set_camera(solstice_2d::Transform2D::scale(x, y));
            g.image(geometry, &ctx.resources.sprites);

            {
                let (x, y) = player.position();
                let rot = solstice_2d::Rad(ctx.time.as_secs_f32());
                let tx = solstice_2d::Transform2D::translation(x, y);
                let tx = tx * solstice_2d::Transform2D::rotation(rot);
                g.draw_with_color_and_transform(
                    solstice_2d::Circle {
                        x: 0.,
                        y: 0.,
                        radius: map.tile_size[0] / 4.,
                        segments: 4,
                    },
                    [0.6, 1., 0.4, 1.0],
                    tx,
                );
            }
            g.set_camera(solstice_2d::Transform2D::default());
        }

        g.set_canvas(Some(ctx.aesthetic_canvas.clone()));

        use solstice_2d::Rad;
        const ROT: f32 = std::f32::consts::FRAC_PI_2;
        let rot = -ratio * ROT + ROT * index as f32;
        let tx = solstice_2d::Transform3D::translation(0., 0., -1. / 2.);
        let tx = tx * solstice_2d::Transform3D::rotation(Rad(0.), Rad(rot), Rad(0.));
        let tx = tx * solstice_2d::Transform3D::translation(0., 0., 1. / 2.);

        g.set_camera(camera);
        let plane = solstice_2d::Plane::new(1., 1., 1, 1);
        g.image_with_transform(plane, ctx.canvas, tx);
    }

    {
        let mut g = ctx.gfx.lock(ctx.ctx);
        g.set_canvas(None);
        g.set_shader(Some(aesthetic_shader.clone()));
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

pub struct RotateTransition<FROM, TO> {
    pub from: FROM,
    pub to: TO,
    pub elapsed: std::time::Duration,
    pub time: std::time::Duration,
}

impl RotateTransition<Main, Main> {
    pub fn update(mut self, dt: std::time::Duration, _ctx: StateContext) -> State {
        self.elapsed += dt;
        if self.elapsed >= self.time {
            self.elapsed -= self.time;
            State::Main(self.to)
        } else {
            State::MainToMain(self)
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        let ratio = self.elapsed.as_secs_f32() / self.time.as_secs_f32();
        let from = (&mut self.from.map.inner, &self.from.player);
        let to = (&mut self.to.map.inner, &self.to.player);
        render(ctx, ratio, [from, to]);
    }
}

impl RotateTransition<Main, BadEnd> {
    pub fn update(mut self, dt: std::time::Duration, _ctx: StateContext) -> State {
        self.elapsed += dt;
        if self.elapsed >= self.time {
            State::BadEnd(self.to)
        } else {
            State::MainToBadEnd(self)
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        let ratio = self.elapsed.as_secs_f32() / self.time.as_secs_f32();
        let from = (&mut self.from.map.inner, &self.from.player);
        let to = (&mut self.to.map, &self.to.player);
        render(ctx, ratio, [from, to]);
    }
}
