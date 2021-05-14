use super::{State, StateContext};
use crate::map::{Direction, Grid};
use enumflags2::BitFlags;
use solstice_2d::solstice::quad_batch::QuadBatch;
use solstice_2d::{Color, Draw};

pub struct BadEnd {
    map: super::Map,
    player: crate::player::Player,
}

impl BadEnd {
    pub fn new(ctx: StateContext) -> Result<Self, solstice_2d::GraphicsError> {
        let width = 16;
        let height = 16;
        let grid = map_gen(width, height);

        let tiles = crate::map::create_batch(64., 64., &grid, &ctx.resources.sprites_metadata);
        let mut batch = QuadBatch::new(ctx.ctx, width * height)?;
        for tile in tiles {
            batch.push(tile);
        }

        let [x, y] = [width as f32 / 2. * 64., (height - 1) as f32 * 64.];
        let player = crate::player::Player::new(x, y);
        let map = super::Map {
            grid,
            batch,
            tile_size: [64., 64.],
        };

        Ok(Self { map, player })
    }

    pub fn update(mut self, dt: std::time::Duration, ctx: StateContext) -> State {
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
            if let Some(end) = self.map.grid.valid_move(start, direction) {
                let (x, y) = self.map.coord_to_mid_pixel(end);
                let time = std::time::Duration::from_secs_f32(0.2);
                self.player.try_move(x, y, time);
            }
        }

        self.player.update(dt);

        State::BadEnd(self)
    }

    pub fn render(&mut self, ctx: StateContext) {
        let viewport = *ctx.gfx.viewport();
        let geometry = self.map.batch.unmap(ctx.ctx);

        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(Color::new(0., 0., 0., 1.));

        g.set_canvas(Some(ctx.canvas.clone()));
        g.clear(Color::new(0., 0., 0., 1.));

        {
            let scale = 3.;
            let [gw, gh] = self.map.grid.grid_size();
            let [pw, ph] = [gw as f32 * 64., gh as f32 * 64.];

            let (player_x, player_y) = self.player.position();
            let x = player_x - 256. * scale / 2.;
            let y = player_y - 256. * scale / 2.;

            let max_x = (pw - 256. * scale).max(0.);
            let max_y = (ph - 256. * scale).max(0.);

            let x = x.clamp(0., max_x);
            let y = y.clamp(0., max_y);

            let translation = solstice_2d::Transform2D::translation(-x, -y);
            let scale = solstice_2d::Transform2D::scale(1. / scale, 1. / scale);
            g.set_camera(scale * translation);
        }

        g.image(geometry, &ctx.resources.sprites);

        {
            let (x, y) = self.player.position();
            let rot = solstice_2d::Rad(ctx.time.as_secs_f32());
            let tx = solstice_2d::Transform2D::translation(x, y);
            let tx = tx * solstice_2d::Transform2D::rotation(rot);
            g.draw_with_color_and_transform(
                solstice_2d::Circle {
                    x: 0.,
                    y: 0.,
                    radius: 64. / 4.,
                    segments: 4,
                },
                [0.6, 1., 0.4, 1.0],
                tx,
            );
        }

        g.set_camera(solstice_2d::Transform2D::default());
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
    }
}

fn map_gen(width: usize, height: usize) -> Grid {
    let all = BitFlags::from(Direction::N) | Direction::E | Direction::S | Direction::W;
    let mut data = vec![all; width * height];
    for x in 0..width {
        data[x].remove(Direction::N);
        data[x + (height - 1) * width].remove(Direction::S);
    }
    for y in 0..height {
        data[y * width].remove(Direction::W);
        data[(width - 1) + y * width].remove(Direction::E);
    }

    let mut grid = Grid {
        data: data.into_boxed_slice(),
        width,
        height,
    };

    crate::map::apply_not_corner_bit(&mut grid);

    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_gen_test() {
        let map = map_gen(3, 3);
        assert_eq!(map.data[4], BitFlags::all());
    }
}
