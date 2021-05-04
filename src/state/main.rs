use super::{Map, StateContext};
use solstice_2d::{Color, Draw};

pub struct Main {
    pub map: Map,
    pub player: crate::player::Player,
}

impl Main {
    pub fn update(&mut self, dt: std::time::Duration, ctx: StateContext) {
        use crate::map;
        let direction = if ctx.input_state.w {
            Some(map::Direction::N)
        } else if ctx.input_state.s {
            Some(map::Direction::S)
        } else if ctx.input_state.a {
            Some(map::Direction::W)
        } else if ctx.input_state.d {
            Some(map::Direction::E)
        } else {
            None
        };

        if let Some(direction) = direction {
            let start = self.map.pixel_to_coord(self.player.position());
            if let Some(end) = self.map.map.valid_move(start, direction) {
                let (x, y) = self.map.coord_to_mid_pixel(end);
                let time = std::time::Duration::from_secs_f32(0.2);
                self.player.try_move(x, y, time);
            }
        }

        self.player.update(dt);
    }

    pub fn render(&mut self, ctx: StateContext) {
        let viewport = ctx.gfx.viewport().clone();
        let map = self.map.batch.unmap(ctx.ctx);
        const BLACK: Color = Color::new(0., 0., 0., 1.);

        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(BLACK);

        g.set_canvas(Some(ctx.canvas.clone()));
        g.clear(BLACK);

        g.image(map, &ctx.resources.sprites);

        {
            let (w, h) = self.map.tile_size;
            self.map.map.draw_graph(w, h, &mut g);
        }

        {
            let (x, y) = self.player.position();
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

        {
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

        // {
        //     let fovy = std::f32::consts::FRAC_PI_2;
        //     let aspect = viewport.width() as f32 / viewport.height() as f32;
        //     g.set_projection_mode(Some(solstice_2d::Projection::Perspective(Some(
        //         solstice_2d::Perspective {
        //             aspect,
        //             fovy,
        //             near: 0.1,
        //             far: 1000.0,
        //         },
        //     ))));
        //
        //     let d = 1.;
        //     let dist = d / 2. / fovy.tan();
        //
        //     let geometry = solstice_2d::Box::new(d, d, d, 1, 1, 1);
        //     let tx = solstice_2d::Transform3D::translation(0., 0., dist - d);
        //     // let pitch = solstice_2d::Rad(self.time.as_secs_f32());
        //     // let zero = solstice_2d::Rad(0.);
        //     // let tx = tx * solstice_2d::Transform3D::rotation(zero, pitch, zero);
        //
        //     g.image_with_transform(geometry, self.canvas.clone(), tx);
        //     g.set_projection_mode(None);
        // }
    }
}
