use super::{bad_end::BadEnd, main::Main, State, StateContext};
use crate::{player::Player, state::Map};
use solstice_2d::{Color, Draw};

fn render<'a>(
    mut ctx: StateContext<'a, '_, 'a>,
    ratio: f32,
    states: [(&'a mut Map, &'a Player, f32); 2],
) {
    let viewport = ctx.g.gfx().viewport().clone();

    let (w, h) = ctx.aesthetic_canvas.dimensions();
    let mut from_camera = super::Camera::new(w, h);
    from_camera.for_map_with_scale(&states[0].0, &states[0].1, states[0].2);
    let mut to_camera = super::Camera::new(w, h);
    to_camera.for_map_with_scale(&states[1].0, &states[1].1, states[1].2);

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
        let g = &mut ctx.g;
        g.clear(BLACK);

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
        g.set_canvas(None);
    }

    for (index, (map, player, _)) in std::array::IntoIter::new(states).enumerate() {
        ctx.g.set_canvas(Some(ctx.canvas.clone()));
        ctx.g.clear(BLACK);

        use super::DrawableMap;
        map.render(player, &mut ctx);
        map.render_player(player, &mut ctx);
        map.render_overlay(player, 2, &mut ctx);
        ctx.g.set_camera(solstice_2d::Transform2D::default());

        let g = &mut ctx.g;
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
        let g = &mut ctx.g;
        g.set_camera(solstice_2d::Transform3D::default());
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
        let from = (&mut self.from.map.inner, &self.from.player, 1.);
        let to = (&mut self.to.map.inner, &self.to.player, 1.);
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
        let from = (&mut self.from.map.inner, &self.from.player, 1.);
        let to = (&mut self.to.map, &self.to.player, BadEnd::SCALE);
        render(ctx, ratio, [from, to]);
    }
}
