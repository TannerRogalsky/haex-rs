use super::{main::Main, State, StateContext};
use solstice_2d::{Color, Draw};

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
            self.elapsed -= self.time;
            State::Main(self.to)
        } else {
            State::MainToMain(self)
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        let viewport = ctx.gfx.viewport().clone();
        let fovy = std::f32::consts::FRAC_PI_2;
        let aspect = viewport.width() as f32 / viewport.height() as f32;
        let perspective = solstice_2d::Perspective {
            aspect,
            fovy,
            near: 0.1,
            far: 1000.0,
        };
        let projection = Some(solstice_2d::Projection::Perspective(Some(perspective)));
        const BLACK: Color = Color::new(0., 0., 0., 1.);

        {
            let mut g = ctx.gfx.lock(ctx.ctx);
            g.clear(BLACK);

            g.set_canvas(Some(ctx.canvas.clone()));
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

            {
                g.set_shader(Some({
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
                }));
                let d = viewport.width().min(viewport.height()) as f32;
                let x = viewport.width() as f32 / 2. - d / 2.;
                g.image(
                    solstice_2d::Rectangle {
                        x,
                        y: 0.0,
                        width: d,
                        height: d,
                    },
                    ctx.canvas,
                );
            }
        }

        let [gw, _gh] = self.from.map.inner.grid.grid_size();
        let [tw, _th] = self.from.map.inner.tile_size;
        let d1 = 256. / (gw as f32 * tw);
        let dist = d1 / 2. / fovy.tan();

        let [gw, _gh] = self.to.map.inner.grid.grid_size();
        let [tw, _th] = self.to.map.inner.tile_size;
        let d2 = 256. / (gw as f32 * tw);
        // let dist = d2 / 2. / fovy.tan();

        fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
            return v0 + t * (v1 - v0);
        }
        let ratio = self.elapsed.as_secs_f32() / self.time.as_secs_f32();
        let d = lerp(d1, d2, ratio);

        let states = [&mut self.from, &mut self.to];
        for (index, main) in std::array::IntoIter::new(states).enumerate() {
            let map = main.map.inner.batch.unmap(ctx.ctx);
            let mut g = ctx.gfx.lock(ctx.ctx);

            g.set_canvas(Some(ctx.canvas.clone()));
            g.clear(BLACK);

            {
                let [gw, gh] = main.map.inner.grid.grid_size();
                let [tw, th] = main.map.inner.tile_size;
                let x = 256. / (gw as f32 * tw);
                let y = 256. / (gh as f32 * th);
                let camera = solstice_2d::Transform2D::scale(x, y);
                g.set_camera(camera);
                g.image(map, &ctx.resources.sprites);

                // {
                //     let [w, h] = main.map.inner.tile_size;
                //     main.map.draw_graph(w, h, &mut g);
                // }

                {
                    let (x, y) = main.player.position();
                    let rot = solstice_2d::Rad(ctx.time.as_secs_f32());
                    let tx = solstice_2d::Transform2D::translation(x, y);
                    let tx = tx * solstice_2d::Transform2D::rotation(rot);
                    g.draw_with_color_and_transform(
                        solstice_2d::Circle {
                            x: 0.,
                            y: 0.,
                            radius: 5.,
                            segments: 4,
                        },
                        [0.6, 1., 0.4, 1.0],
                        tx,
                    );
                }

                g.set_camera(solstice_2d::Transform2D::default());
            }

            g.set_canvas(None);

            let plane = solstice_2d::Plane::new(1., 1., 1, 1);
            use solstice_2d::Rad;
            const ROT: f32 = std::f32::consts::FRAC_PI_2;
            let rot = -ratio * ROT + ROT * index as f32;
            let tx = solstice_2d::Transform3D::translation(0., 0., dist - 0.5 / (1. / d) - 1. / 2.);
            let tx = tx * solstice_2d::Transform3D::scale(1., -1., 1.);
            let tx = tx * solstice_2d::Transform3D::rotation(Rad(0.), Rad(rot), Rad(0.));
            let tx = tx * solstice_2d::Transform3D::translation(0., 0., 1. / 2.);

            g.set_projection_mode(projection);
            g.image_with_transform(plane, ctx.canvas, tx);
        }
    }
}
