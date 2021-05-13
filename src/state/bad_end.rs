use super::{State, StateContext};
use crate::map::{Direction, Grid};
use enumflags2::BitFlags;
use solstice_2d::solstice::quad_batch::QuadBatch;
use solstice_2d::{Color, Draw, Vertex2D};

pub struct BadEnd {
    _grid: Grid,
    batch: QuadBatch<Vertex2D>,
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

        Ok(Self { _grid: grid, batch })
    }

    pub fn update(self, _dt: std::time::Duration, _ctx: StateContext) -> State {
        State::BadEnd(self)
    }

    pub fn render(&mut self, ctx: StateContext) {
        let geometry = self.batch.unmap(ctx.ctx);

        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(Color::new(0., 0., 0., 1.));
        g.image(geometry, &ctx.resources.sprites);
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
