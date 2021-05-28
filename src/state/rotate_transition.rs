use super::{bad_end::BadEnd, main::Main, State, StateContext};
use crate::{player::Player, state::Map};
use solstice_2d::{Color, Draw};

struct RenderState<'a> {
    map: &'a mut Map,
    player: &'a Player,
    aesthetic: crate::AestheticShader,
    camera: super::Camera,
}

fn render<'a>(mut ctx: StateContext<'a, '_, 'a>, ratio: f32, states: [RenderState<'a>; 2]) {
    let viewport = ctx.g.gfx().viewport().clone();

    let aesthetic = states[0].aesthetic.lerp(&states[1].aesthetic, ratio);
    let camera = states[0]
        .camera
        .transform
        .lerp_slerp(&states[1].camera.transform, ratio);

    const BLACK: Color = Color::new(0., 0., 0., 1.);

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

    for (index, RenderState { map, player, .. }) in std::array::IntoIter::new(states).enumerate() {
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
        g.set_shader(Some(aesthetic.as_shader(ctx.resources)));
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
        let (w, h) = ctx.aesthetic_canvas.dimensions();
        let mut from_camera = super::Camera::new(w, h);
        from_camera.for_map_with_scale(&self.from.map.inner, &self.from.player, 1.);
        let from = RenderState {
            map: &mut self.from.map.inner,
            player: &self.from.player,
            aesthetic: self.from.progression.settings.aesthetic,
            camera: from_camera,
        };
        let mut to_camera = super::Camera::new(w, h);
        to_camera.for_map_with_scale(&self.to.map.inner, &self.to.player, 1.);
        let to = RenderState {
            map: &mut self.to.map.inner,
            player: &self.to.player,
            aesthetic: self.to.progression.settings.aesthetic,
            camera: to_camera,
        };
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
        let (w, h) = ctx.aesthetic_canvas.dimensions();
        let mut from_camera = super::Camera::new(w, h);
        from_camera.for_map_with_scale(&self.from.map.inner, &self.from.player, 1.);
        let from = RenderState {
            map: &mut self.from.map.inner,
            player: &self.from.player,
            aesthetic: self.from.progression.settings.aesthetic,
            camera: from_camera,
        };
        let mut to_camera = super::Camera::new(w, h);
        to_camera.for_map_with_scale_and_follow(
            &self.to.map,
            &self.to.player,
            BadEnd::SCALE,
            false,
        );
        let to = RenderState {
            map: &mut self.to.map,
            player: &self.to.player,
            aesthetic: BadEnd::AESTHETIC,
            camera: to_camera,
        };
        render(ctx, ratio, [from, to]);
    }
}
