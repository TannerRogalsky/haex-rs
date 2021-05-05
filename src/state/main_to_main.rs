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
            State::Main(self.to)
        } else {
            State::MainToMain(self)
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        {
            let main = &mut self.from;
            let map = main.map.batch.unmap(ctx.ctx);
            const BLACK: Color = Color::new(0., 0., 0., 1.);

            let mut g = ctx.gfx.lock(ctx.ctx);
            g.clear(BLACK);

            g.set_canvas(Some(ctx.canvas.clone()));
            g.clear(BLACK);

            g.image(map, &ctx.resources.sprites);

            {
                let (w, h) = main.map.tile_size;
                main.map.map.draw_graph(w, h, &mut g);
            }

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
            g.set_canvas(None);
        }

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

        let d = 1.;
        let dist = d / 2. / fovy.tan();

        let plane = solstice_2d::Plane::new(1., 1., 1, 1);
        use solstice_2d::Rad;
        let ratio = self.elapsed.as_secs_f32() / self.time.as_secs_f32();
        const ROT: f32 = std::f32::consts::FRAC_PI_2;
        let tx = solstice_2d::Transform3D::translation(0., 0., dist - d / 2.);
        let tx = tx * solstice_2d::Transform3D::rotation(Rad(0.), Rad(ratio * ROT), Rad(0.));
        {
            let mut g = ctx.gfx.lock(ctx.ctx);
            g.set_projection_mode(projection);
            g.image_with_transform(plane, ctx.canvas, tx);
        }

        // {
        //     let main = &mut self.to;
        //     let viewport = ctx.gfx.viewport().clone();
        //     let map = main.map.batch.unmap(ctx.ctx);
        //     const BLACK: Color = Color::new(0., 0., 0., 1.);
        //
        //     let mut g = ctx.gfx.lock(ctx.ctx);
        //     g.clear(BLACK);
        //
        //     g.set_canvas(Some(ctx.canvas.clone()));
        //     g.clear(BLACK);
        //
        //     g.image(map, &ctx.resources.sprites);
        //
        //     {
        //         let (w, h) = main.map.tile_size;
        //         main.map.map.draw_graph(w, h, &mut g);
        //     }
        //
        //     {
        //         let (x, y) = main.player.position();
        //         let rot = solstice_2d::Rad(ctx.time.as_secs_f32());
        //         let tx = solstice_2d::Transform2D::translation(x, y);
        //         let tx = tx * solstice_2d::Transform2D::rotation(rot);
        //         g.draw_with_color_and_transform(
        //             solstice_2d::Circle {
        //                 x: 0.,
        //                 y: 0.,
        //                 radius: 5.,
        //                 segments: 4,
        //             },
        //             [0.6, 1., 0.4, 1.0],
        //             tx,
        //         );
        //     }
        //
        //     g.set_canvas(None);
        //     g.set_shader(Some({
        //         let mut shader = ctx.resources.shaders.aesthetic.clone();
        //         shader.send_uniform("blockThreshold", 0.073f32);
        //         shader.send_uniform("lineThreshold", 0.23f32);
        //         shader.send_uniform("randomShiftScale", 0.002f32);
        //         shader.send_uniform("radialScale", 0.1f32);
        //         shader.send_uniform("radialBreathingScale", 0.01f32);
        //         let unit = 1;
        //         shader.bind_texture_at_location(&ctx.resources.noise, (unit as usize).into());
        //         shader.send_uniform("tex1", unit);
        //         shader
        //     }));
        //
        //     {
        //         let d = viewport.width().min(viewport.height()) as f32;
        //         let x = viewport.width() as f32 / 2. - d / 2.;
        //         g.image(
        //             solstice_2d::Rectangle {
        //                 x,
        //                 y: 0.0,
        //                 width: d,
        //                 height: d,
        //             },
        //             ctx.canvas,
        //         );
        //     }
        // }
        //
        // {
        //     let mut g = ctx.gfx.lock(ctx.ctx);
        //     let tx = tx * solstice_2d::Transform3D::rotation(Rad(0.), Rad(-ROT), Rad(0.));
        //     g.image_with_transform(plane, ctx.canvas, tx);
        // }
    }
}
