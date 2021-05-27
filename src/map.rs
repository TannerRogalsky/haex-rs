use enumflags2::*;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Direction {
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

    pub const fn cardinals() -> [Direction; 4] {
        [Direction::N, Direction::E, Direction::S, Direction::W]
    }
}

fn d1_to_d2(index: usize, width: usize) -> Coord {
    (index % width, index / width)
}

#[derive(Debug)]
pub struct Grid<T> {
    pub data: Box<[T]>,
    pub width: usize,
    pub height: usize,
}

impl<T> Grid<T> {
    pub fn grid_size(&self) -> [usize; 2] {
        [self.width, self.height]
    }

    pub fn contains(&self, coord: Coord) -> bool {
        let (x, y) = coord;
        x < self.width && y < self.height
    }

    pub fn coord_to_index(&self, coord: Coord) -> usize {
        coord.0 + coord.1 * self.width
    }

    pub fn checked_coord_to_index(&self, coord: Coord) -> Option<usize> {
        if coord.0 < self.width && coord.1 < self.height {
            Some(self.coord_to_index(coord))
        } else {
            None
        }
    }

    pub fn index_to_coord(&self, index: usize) -> Coord {
        d1_to_d2(index, self.width)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&T, Coord)> + '_ {
        let width = self.width;
        self.data
            .iter()
            .enumerate()
            .map(move |(index, v)| (v, d1_to_d2(index, width)))
    }
}

pub type DirectionGrid = Grid<BitFlags<Direction>>;

impl DirectionGrid {
    pub fn new<R: rand::Rng>(width: usize, height: usize, rng: &mut R) -> Self {
        let weights = Weights {
            random: 1.,
            newest: 1.,
            ..Default::default()
        };
        let grid = growing_tree(width, height, weights, rng);
        Self {
            data: grid.into_boxed_slice(),
            width,
            height,
        }
    }

    pub fn make_open(&mut self, from: Coord, direction: Direction) {
        let cell = self.checked_coord_to_index(from);
        let neighbor = neighbor_coord(from, direction)
            .ok()
            .and_then(|coord| self.checked_coord_to_index(coord));
        if let Some((cell, neighbor)) = cell.zip(neighbor) {
            self.data[cell] |= direction;
            self.data[neighbor] |= direction.opposite();
        }
    }

    pub fn valid_move(&self, start: Coord, direction: Direction) -> Option<Coord> {
        let v = self.data.get(self.coord_to_index(start))?;
        if v.contains(direction) {
            let end = neighbor_coord(start, direction).ok()?;
            if self.checked_coord_to_index(end).is_some() {
                Some(end)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn as_graph(&self) -> Graph {
        let mut graph = Graph::with_capacity(self.data.len(), self.data.len() * 2);

        let origin = self.index_to_coord(0);
        graph.add_node(origin);
        self.add_node(&mut graph, origin);

        graph
    }

    fn add_node(&self, graph: &mut Graph, origin: Coord) {
        let v = &self.data[self.coord_to_index(origin)];
        for direction in Direction::cardinals().iter().copied() {
            if v.contains(direction) {
                let (dx, dy) = direction.into_dir();
                let (nx, ny) = (origin.0 as i32 + dx as i32, origin.1 as i32 + dy as i32);
                let next = (nx as usize, ny as usize);
                if !graph.contains_node(next) {
                    graph.add_node(next);
                    graph.add_edge(origin, next, ());
                    self.add_node(graph, next);
                }
            }
        }
    }
}

pub type Graph = petgraph::graphmap::UnGraphMap<Coord, ()>;
pub type Coord = (usize, usize);

#[derive(Debug, Copy, Clone)]
pub struct MapGenSettings {
    pub width: usize,
    pub height: usize,
    pub programs: ProgramGenSettings,
    pub aesthetic: crate::AestheticShader,
}

#[derive(Debug, Copy, Clone)]
pub struct ProgramGenSettings {
    pub nop_slide_count: usize,
}

pub fn apply_not_corner_bit(grid: &mut DirectionGrid) {
    for index in 0..grid.data.len() {
        let (x, y) = grid.index_to_coord(index);
        if let Ok((tx, ty)) = neighbor_coord((x, y), Direction::SEC) {
            let t1 = grid.coord_to_index((tx, y));
            let t2 = grid.coord_to_index((x, ty));
            if grid.data[index].contains(Direction::E)
                && grid.data[index].contains(Direction::S)
                && grid.data[t1].contains(Direction::S)
                && grid.data[t2].contains(Direction::E)
            {
                grid.data[index].insert(Direction::SEC);
            }
        }
        if let Ok((tx, ty)) = neighbor_coord((x, y), Direction::SWC) {
            let t1 = grid.coord_to_index((tx, y));
            let t2 = grid.coord_to_index((x, ty));
            if grid.data[index].contains(Direction::W)
                && grid.data[index].contains(Direction::S)
                && grid.data[t1].contains(Direction::S)
                && grid.data[t2].contains(Direction::W)
            {
                grid.data[index].insert(Direction::SWC);
            }
        }
        if let Ok((tx, ty)) = neighbor_coord((x, y), Direction::NWC) {
            let t1 = grid.coord_to_index((tx, y));
            let t2 = grid.coord_to_index((x, ty));
            if grid.data[index].contains(Direction::W)
                && grid.data[index].contains(Direction::N)
                && grid.data[t1].contains(Direction::N)
                && grid.data[t2].contains(Direction::W)
            {
                grid.data[index].insert(Direction::NWC);
            }
        }
        if let Ok((tx, ty)) = neighbor_coord((x, y), Direction::NEC) {
            let t1 = grid.coord_to_index((tx, y));
            let t2 = grid.coord_to_index((x, ty));
            if grid.data[index].contains(Direction::E)
                && grid.data[index].contains(Direction::N)
                && grid.data[t1].contains(Direction::N)
                && grid.data[t2].contains(Direction::E)
            {
                grid.data[index].insert(Direction::NEC);
            }
        }
    }
}

pub fn create_batch(
    tile_width: f32,
    tile_height: f32,
    grid: &DirectionGrid,
    tiles: &std::collections::HashMap<String, crate::UVRect>,
) -> Vec<solstice_2d::solstice::quad_batch::Quad<solstice_2d::Vertex2D>> {
    use solstice_2d::solstice::{quad_batch::Quad, viewport::Viewport};

    let mut quads = Vec::with_capacity(grid.width * grid.height);

    for x in 0..grid.width {
        for y in 0..grid.height {
            let index = grid.coord_to_index((x, y));
            let cell = grid.data[index];

            let name = if cell.is_empty() {
                "tiles/tile_342.png"
            } else if cell == Direction::N {
                "tiles/tile_286.png"
            } else if cell == Direction::E {
                "tiles/tile_313.png"
            } else if cell == Direction::S {
                "tiles/tile_312.png"
            } else if cell == Direction::W {
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
                "tiles/tile_340.png"
            } else {
                panic!("couldn't match cell to tile. {:?}", cell);
            };

            let tile = match tiles.get(name).cloned() {
                None => panic!("couldn't find {}", name),
                Some(tile) => tile,
            };
            let quad = Quad::from(Viewport::new(
                x as f32 * tile_width,
                y as f32 * tile_height,
                tile_width,
                tile_height,
            ))
            .zip(tile.uvs.into())
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

pub fn neighbor_coord(
    from: Coord,
    direction: Direction,
) -> Result<Coord, std::num::TryFromIntError> {
    neighbor_coord_mult(from, direction, 1)
}

pub fn neighbor_coord_mult(
    from: Coord,
    direction: Direction,
    mult: i32,
) -> Result<Coord, std::num::TryFromIntError> {
    use std::convert::TryInto;
    let (dx, dy) = direction.into_dir();
    let (x, y) = (from.0 as i32, from.1 as i32);
    let (nx, ny) = (x + dx as i32 * mult, y + dy as i32 * mult);
    let nx = nx.try_into()?;
    let ny = ny.try_into()?;
    Ok((nx, ny))
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
    let mut cardinal_directions = Direction::cardinals();

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

    #[test]
    fn graph_test() {
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

        let grid = Grid {
            data: grid.into_boxed_slice(),
            width: 3,
            height: 3,
        };
        let graph = grid.as_graph();

        for x in 0..3 {
            for y in 0..3 {
                let coord = (x, y);
                let index = grid.coord_to_index(coord);

                for neighbor in graph.neighbors(coord) {
                    let dx = neighbor.0 as i32 - coord.0 as i32;
                    let dy = neighbor.1 as i32 - coord.1 as i32;

                    let directions = grid.data[index];
                    let valid = directions.iter().any(|direction| {
                        let (nx, ny) = direction.into_dir();
                        nx as i32 == dx && ny as i32 == dy
                    });
                    assert!(valid, "({:?} -> {:?})", coord, neighbor);
                }
            }
        }
    }

    #[test]
    fn neighbor_test() {
        let origin = (5, 4);
        assert_eq!(neighbor_coord(origin, Direction::N), Ok((5, 3)));

        assert_eq!(neighbor_coord_mult(origin, Direction::N, 2), Ok((5, 2)));
        assert_eq!(neighbor_coord_mult(origin, Direction::N, 3), Ok((5, 1)));
        assert_eq!(neighbor_coord_mult(origin, Direction::N, 4), Ok((5, 0)));
        assert!(neighbor_coord_mult(origin, Direction::N, 5).is_err());
    }

    #[test]
    fn make_open_test() {
        let (width, height) = (2, 2);
        let data = vec![BitFlags::empty(); width * height];
        let mut grid = Grid {
            data: data.into_boxed_slice(),
            width,
            height,
        };

        let from = (0, 0);
        let to = (0, 1);
        grid.make_open(from, Direction::S);
        assert_eq!(grid.data[grid.coord_to_index(from)], Direction::S);
        assert_eq!(grid.data[grid.coord_to_index(to)], Direction::N);

        grid.make_open((1, 0), Direction::E);
        assert_eq!(grid.data[grid.coord_to_index((1, 0))], BitFlags::empty());
        assert_ne!(grid.data[grid.coord_to_index((0, 1))], BitFlags::empty());
    }
}
