use crate::programs::{State, StateMut};
use solstice_2d::Draw;

struct Timer {
    t: std::time::Duration,
    elapsed: std::time::Duration,
}

impl Timer {
    fn new(t: std::time::Duration) -> Self {
        Self {
            t,
            elapsed: Default::default(),
        }
    }

    fn update(&mut self, dt: std::time::Duration) -> bool {
        self.elapsed += dt;
        if self.elapsed >= self.t {
            self.elapsed -= self.t;
            true
        } else {
            false
        }
    }
}

struct BasicEnemy {
    move_timer: Timer,
    prev_position: [f32; 2],
}

enum EnemyType {
    Basic(BasicEnemy),
}

pub struct Enemy {
    position: [f32; 2],
    ty: EnemyType,
    t: std::time::Duration,
}

impl Enemy {
    pub fn new_basic(x: f32, y: f32) -> Self {
        Self {
            position: [x, y],
            ty: EnemyType::Basic(BasicEnemy {
                move_timer: Timer::new(std::time::Duration::from_secs_f32(1.)),
                prev_position: [x, y],
            }),
            t: Default::default(),
        }
    }

    pub fn update(&mut self, dt: std::time::Duration, ctx: &StateMut) {
        self.t += dt;
        let mut rng: rand::rngs::SmallRng =
            rand::SeedableRng::seed_from_u64(self.t.as_millis() as _);
        let mut directions = crate::map::Direction::cardinals();
        rand::seq::SliceRandom::shuffle(&mut directions[..], &mut rng);
        match &mut self.ty {
            EnemyType::Basic(inner) => {
                if inner.move_timer.update(dt) {
                    let [x, y] = self.position;
                    let coord = ctx.map.pixel_to_coord((x, y));
                    let next = std::array::IntoIter::new(directions)
                        .find_map(|dir| ctx.map.grid.valid_move(coord, dir));
                    if let Some(next) = next {
                        inner.prev_position = self.position;
                        let (x, y) = ctx.map.coord_to_mid_pixel(next);
                        self.position = [x, y];
                    }
                }
            }
        }
    }

    pub fn render(&self, ctx: &mut State) {
        match &self.ty {
            EnemyType::Basic(inner) => {
                let uvs = ctx.ctx.resources.sprites_metadata.enemy1_body.uvs;
                let [tw, th] = ctx.map.tile_size;
                let [tw, th] = [tw * 0.8, th * 0.8];
                let [x1, y1] = self.position;
                let [x2, y2] = inner.prev_position;
                let angle = (y2 - y1).atan2(x2 - x1) + std::f32::consts::FRAC_PI_2;
                ctx.ctx.g.image_with_transform(
                    crate::UVRect {
                        positions: solstice_2d::Rectangle {
                            x: -tw / 2.,
                            y: -th / 2.,
                            width: tw,
                            height: th,
                        },
                        uvs,
                    },
                    &ctx.ctx.resources.sprites,
                    solstice_2d::Transform2D::translation(x1, y1)
                        * solstice_2d::Transform2D::rotation(solstice_2d::Rad(angle)),
                );
            }
        }
    }
}
