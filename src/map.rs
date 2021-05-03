use enumflags2::*;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Direction {
    N = 1 << 0,
    E = 1 << 1,
    S = 1 << 2,
    W = 1 << 3,
    NEC = 1 << 4,
    SEC = 1 << 5,
    SWC = 1 << 6,
    NWC = 1 << 7,
}

impl Direction {
    pub fn into_dir(self) -> (i8, i8) {
        match self {
            Direction::N => (0, -1),
            Direction::E => (1, 0),
            Direction::S => (0, 1),
            Direction::W => (-1, 0),
            Direction::NEC => (1, -1),
            Direction::SEC => (1, 1),
            Direction::SWC => (-1, 1),
            Direction::NWC => (-1, -1),
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::N => Direction::S,
            Direction::E => Direction::W,
            Direction::S => Direction::N,
            Direction::W => Direction::E,
            Direction::NEC => Direction::SWC,
            Direction::SEC => Direction::NWC,
            Direction::SWC => Direction::NEC,
            Direction::NWC => Direction::SEC,
        }
    }
}

pub fn create_batch(
    tile_width: f32,
    tile_height: f32,
    map: &Map,
    tiles: &std::collections::HashMap<String, solstice_2d::solstice::quad_batch::Quad<(f32, f32)>>,
) -> Vec<solstice_2d::solstice::quad_batch::Quad<solstice_2d::Vertex2D>> {
    use solstice_2d::solstice::{quad_batch::Quad, viewport::Viewport};

    let mut quads = Vec::with_capacity(map.width * map.height);

    for x in 0..map.width {
        for y in 0..map.height {
            let index = x + y * map.width;
            let cell = map.grid[index];

            let name = if cell.is_empty() {
                "tiles/tile_342.png"
            } else if cell == BitFlags::<Direction>::from(Direction::N) {
                "tiles/tile_286.png"
            } else if cell == BitFlags::<Direction>::from(Direction::E) {
                "tiles/tile_313.png"
            } else if cell == BitFlags::<Direction>::from(Direction::S) {
                "tiles/tile_312.png"
            } else if cell == BitFlags::<Direction>::from(Direction::W) {
                "tiles/tile_285.png"
            } else if cell == Direction::N | Direction::E {
                "tiles/tile_307.png"
            } else if cell == Direction::N | Direction::W {
                "tiles/tile_308.png"
            } else if cell == Direction::N | Direction::S {
                "tiles/tile_309.png"
            } else if cell == Direction::S | Direction::E {
                "tiles/tile_280.png"
            } else if cell == Direction::S | Direction::W {
                "tiles/tile_281.png"
            } else if cell == Direction::E | Direction::W {
                "tiles/tile_282.png"
            } else if cell == Direction::S | Direction::E | Direction::W {
                "tiles/tile_283.png"
            } else if cell == Direction::N | Direction::E | Direction::W {
                "tiles/tile_284.png"
            } else if cell == Direction::N | Direction::S | Direction::E {
                "tiles/tile_310.png"
            } else if cell == Direction::N | Direction::S | Direction::W {
                "tiles/tile_311.png"
            } else if cell == Direction::N | Direction::S | Direction::E | Direction::W {
                "tiles/tile_341.png"
            } else if cell == Direction::NEC | Direction::N | Direction::E {
                "tiles/tile_314.png"
            } else if cell == Direction::NWC | Direction::N | Direction::W {
                "tiles/tile_315.png"
            } else if cell == Direction::SEC | Direction::S | Direction::E {
                "tiles/tile_287.png"
            } else if cell == Direction::SWC | Direction::S | Direction::W {
                "tiles/tile_288.png"
            } else if cell == Direction::NEC | Direction::N | Direction::E | Direction::W {
                "tiles/tile_419.png"
            } else if cell == Direction::NWC | Direction::N | Direction::E | Direction::W {
                "tiles/tile_420.png"
            } else if cell == Direction::SEC | Direction::S | Direction::E | Direction::W {
                "tiles/tile_392.png"
            } else if cell == Direction::SWC | Direction::S | Direction::E | Direction::W {
                "tiles/tile_393.png"
            } else if cell == Direction::NEC | Direction::N | Direction::S | Direction::E {
                "tiles/tile_417.png"
            } else if cell == Direction::SEC | Direction::N | Direction::S | Direction::E {
                "tiles/tile_390.png"
            } else if cell == Direction::NWC | Direction::N | Direction::S | Direction::W {
                "tiles/tile_418.png"
            } else if cell == Direction::SWC | Direction::N | Direction::S | Direction::W {
                "tiles/tile_391.png"
            } else if cell
                == Direction::NEC | Direction::NWC | Direction::N | Direction::E | Direction::W
            {
                "tiles/tile_366.png"
            } else if cell
                == Direction::SEC | Direction::SWC | Direction::S | Direction::E | Direction::W
            {
                "tiles/tile_365.png"
            } else if cell
                == Direction::NEC | Direction::SEC | Direction::N | Direction::S | Direction::E
            {
                "tiles/tile_338.png"
            } else if cell
                == Direction::NWC | Direction::SWC | Direction::N | Direction::S | Direction::W
            {
                "tiles/tile_339.png"
            } else if cell
                == Direction::NEC
                    | Direction::NWC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_336.png"
            } else if cell
                == Direction::SEC
                    | Direction::SWC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_337.png"
            } else if cell
                == Direction::NWC
                    | Direction::SWC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_363.png"
            } else if cell
                == Direction::NEC
                    | Direction::SEC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_364.png"
            } else if cell
                == Direction::SWC
                    | Direction::NWC
                    | Direction::NEC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_334.png"
            } else if cell
                == Direction::SEC
                    | Direction::NWC
                    | Direction::NEC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_335.png"
            } else if cell
                == Direction::SEC
                    | Direction::SWC
                    | Direction::NEC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_362.png"
            } else if cell
                == Direction::SEC
                    | Direction::SWC
                    | Direction::NWC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_361.png"
            } else if cell
                == Direction::SEC
                    | Direction::SWC
                    | Direction::NWC
                    | Direction::NEC
                    | Direction::N
                    | Direction::S
                    | Direction::E
                    | Direction::W
            {
                "tiles/tile_340.png'"
            } else {
                panic!()
            };

            let tile = tiles.get(name).unwrap().clone();
            let quad = Quad::from(Viewport::new(
                x as f32 * tile_width,
                y as f32 * tile_height,
                tile_width,
                tile_height,
            ))
            .zip(tile)
            .map(|((x, y), (s, t))| solstice_2d::Vertex2D {
                position: [x, y],
                color: [1., 1., 1., 1.],
                uv: [s, t],
            });
            quads.push(quad);
        }
    }

    quads
}

#[derive(Debug)]
pub struct Map {
    grid: Box<[BitFlags<Direction>]>,
    width: usize,
    height: usize,
}

impl Map {
    pub fn new<R: rand::Rng>(width: usize, height: usize, rng: &mut R) -> Self {
        let weights = Weights {
            random: 1.,
            newest: 1.,
            ..Default::default()
        };
        let grid = growing_tree(width, height, weights, rng);
        Self {
            grid: grid.into_boxed_slice(),
            width,
            height,
        }
    }
}

#[derive(Copy, Clone)]
enum Selector {
    Random,
    Newest,
    Middle,
    Oldest,
}

impl Selector {
    pub fn select<R: rand::Rng>(self, r: &mut R, ceil: usize) -> usize {
        match self {
            Selector::Random => r.gen_range(0..=ceil),
            Selector::Newest => ceil,
            Selector::Middle => ceil / 2,
            Selector::Oldest => 0,
        }
    }
}

#[derive(Default)]
pub struct Weights {
    random: f32,
    newest: f32,
    middle: f32,
    oldest: f32,
}

fn growing_tree<R>(
    width: usize,
    height: usize,
    weights: Weights,
    rng: &mut R,
) -> Vec<BitFlags<Direction>>
where
    R: rand::Rng,
{
    let choices = [
        Selector::Random,
        Selector::Newest,
        Selector::Middle,
        Selector::Oldest,
    ];
    let dist = rand_distr::WeightedIndex::new(&[
        weights.random,
        weights.newest,
        weights.middle,
        weights.oldest,
    ])
    .unwrap();

    let mut cells: Vec<(usize, usize)> = Vec::with_capacity((width * height) / 2);
    cells.push((rng.gen_range(0..width), rng.gen_range(0..height)));

    let mut grid = vec![BitFlags::empty(); width * height];
    let mut cardinal_directions = [Direction::N, Direction::S, Direction::E, Direction::W];

    while !cells.is_empty() {
        let selector = choices[rand::distributions::Distribution::sample(&dist, rng)];
        let index = selector.select(rng, cells.len() - 1);
        let (x, y) = cells[index];

        let mut remove_cell = true;

        rand::seq::SliceRandom::shuffle(&mut cardinal_directions[..], rng);
        for direction in cardinal_directions.iter() {
            let (dx, dy) = direction.into_dir();
            let (nx, ny) = (x as i32 + dx as i32, y as i32 + dy as i32);
            if nx >= 0 && ny >= 0 {
                let (nx, ny) = (nx as usize, ny as usize);
                if nx < width && ny < height {
                    let index = x + y * width;
                    let next = nx + ny * width;
                    if grid[next] == BitFlags::empty() {
                        grid[index] |= BitFlags::from(*direction);
                        grid[next] |= direction.opposite();
                        cells.push((nx, ny));

                        remove_cell = false;
                        break;
                    }
                }
            }
        }

        if remove_cell {
            cells.remove(index);
        }
    }

    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growing_tree_test() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
        let grid = growing_tree(
            3,
            3,
            Weights {
                random: 1.0,
                newest: 1.0,
                middle: 0.0,
                oldest: 0.0,
            },
            &mut rng,
        );

        assert_eq!(grid.len(), 3 * 3);
        for cell in grid {
            assert_ne!(cell, BitFlags::empty());
        }
    }
}
