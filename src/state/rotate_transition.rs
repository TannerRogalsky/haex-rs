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

    use crate::state::player_tx;
    fn get_radius(map: &crate::state::Map) -> f32 {
        let [tw, _] = map.tile_size;
        let [width, height] = map.pixel_dimensions();
        let scale = 1. / width.max(height);
        scale * tw / 4.
    }

    let from_tx = player_tx(states[0].player, states[0].map);
    let to_tx = player_tx(states[1].player, states[1].map);
    let from_rad = get_radius(states[0].map);
    let to_rad = get_radius(states[1].map);
    let radius = crate::lerp(from_rad, to_rad, ratio);

    const ROT: f32 = std::f32::consts::FRAC_PI_2;

    let move_back = solstice_2d::Transform3D::translation(
        0.,
        0.,
        (ratio * std::f32::consts::PI).sin().powf(2.) * -0.5,
    );

    use solstice_2d::Rad;
    for (index, RenderState { map, player, .. }) in std::array::IntoIter::new(states).enumerate() {
        ctx.g.set_canvas(Some(ctx.canvas.clone()));
        ctx.g.clear(BLACK);

        use super::DrawableMap;
        map.render(player, &mut ctx);
        map.render_overlay(player, 2, &mut ctx);
        ctx.g.set_camera(solstice_2d::Transform2D::default());

        let g = &mut ctx.g;
        g.set_canvas(Some(ctx.aesthetic_canvas.clone()));

        let rot = -ratio * ROT + ROT * index as f32;
        let tx = solstice_2d::Transform3D::translation(0., 0., -1. / 2.);
        let tx = tx * solstice_2d::Transform3D::rotation(Rad(0.), Rad(rot), Rad(0.));
        let tx = tx * solstice_2d::Transform3D::translation(0., 0., 1. / 2.);

        g.set_camera(camera * move_back);
        let plane = solstice_2d::Plane::new(1., 1., 1, 1);
        g.image_with_transform(plane, ctx.canvas, tx);
    }

    // let midpoint = from_tx.lerp_slerp(&to_tx, 0.5);
    let midpoint = solstice_2d::Transform3D::rotation(Rad(0.), Rad(ROT / 2.), Rad(0.));
    let midpoint = solstice_2d::Point3D::from(midpoint.transform_point(0., 0., 1.2));
    let midpoint = midpoint.normalize();
    let point_a = solstice_2d::Point3D::from(from_tx.transform_point(0., 0., 0.));
    let point_b = solstice_2d::Point3D::from(to_tx.transform_point(0., 0., 0.));
    let point = bezier::de_casteljau3(ratio, point_a, midpoint, point_b);
    let _rotation = {
        let [a, b] = bezier::derivative3(point_a, midpoint, point_b);
        let velocity = bezier::de_casteljau2(ratio, a, b).normalize();
        solstice_2d::Transform3D::look_at(velocity.x, velocity.y, velocity.z)
    };
    let tx = solstice_2d::Transform3D::translation(point.x, point.y, point.z);
    Player::render(
        radius,
        [1., 0., 0., 1.],
        tx,
        &mut ctx,
        camera.inverse_transform_point(0., 0., 0.),
    );

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

mod bezier {
    use solstice_2d::Point3D;

    pub fn derivative3(a: Point3D, b: Point3D, c: Point3D) -> [Point3D; 2] {
        let a = Point3D {
            x: b.x - a.x,
            y: b.y - a.y,
            z: b.z - a.z,
        };
        let b = Point3D {
            x: c.x - b.x,
            y: c.y - b.y,
            z: c.z - b.z,
        };
        [a.multiply_scalar(2.), b.multiply_scalar(2.)]
    }

    pub fn de_casteljau3(t: f32, a: Point3D, b: Point3D, c: Point3D) -> Point3D {
        let one_minus = 1. - t;
        let a = a.multiply_scalar(one_minus * one_minus);
        let b = b.multiply_scalar(2. * one_minus * t);
        let c = c.multiply_scalar(t * t);
        Point3D {
            x: a.x + b.x + c.x,
            y: a.y + b.y + c.y,
            z: a.z + b.z + c.z,
        }
    }

    pub fn de_casteljau2(t: f32, a: Point3D, b: Point3D) -> Point3D {
        let a = a.multiply_scalar(1. - t);
        let b = b.multiply_scalar(t);
        Point3D {
            x: a.x + b.x,
            y: a.y + b.y,
            z: a.z + b.z,
        }
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
