use crate::programs::{State, StateMut};
use solstice_2d::Draw;

#[derive(Debug)]
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

    fn ratio(&self) -> f32 {
        self.elapsed.as_secs_f32() / self.t.as_secs_f32()
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

#[derive(Debug)]
enum BasicEnemyState {
    Stationary(Timer),
    Moving([f32; 2], Timer),
}

#[derive(Debug)]
struct BasicEnemy {
    state: BasicEnemyState,
    prev_position: [f32; 2],
}

impl BasicEnemy {
    pub const WAIT_TIME: std::time::Duration = std::time::Duration::from_secs(1);
}

#[derive(Debug)]
enum EnemyType {
    Basic(BasicEnemy),
}

#[derive(Debug)]
pub struct Enemy {
    pub position: [f32; 2],
    ty: EnemyType,
    t: std::time::Duration,
}

impl Enemy {
    pub fn new_basic(x: f32, y: f32) -> Self {
        Self {
            position: [x, y],
            ty: EnemyType::Basic(BasicEnemy {
                state: BasicEnemyState::Stationary(Timer::new(BasicEnemy::WAIT_TIME)),
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
            EnemyType::Basic(inner) => match &mut inner.state {
                BasicEnemyState::Stationary(timer) => {
                    if timer.update(dt) {
                        let [x, y] = self.position;
                        let coord = ctx.map.pixel_to_coord((x, y));
                        let next = std::array::IntoIter::new(directions)
                            .find_map(|dir| ctx.map.grid.valid_move(coord, dir));
                        if let Some(next) = next {
                            inner.prev_position = self.position;
                            let (x, y) = ctx.map.coord_to_mid_pixel(next);
                            inner.state =
                                BasicEnemyState::Moving([x, y], Timer::new(BasicEnemy::WAIT_TIME));
                        }
                    }
                }
                BasicEnemyState::Moving(target, timer) => {
                    let ratio = timer.ratio();
                    for (a, b) in self.position.iter_mut().zip(target.iter()) {
                        *a = crate::lerp(*a, *b, ratio);
                    }
                    if timer.update(dt) {
                        inner.state = BasicEnemyState::Stationary(Timer::new(BasicEnemy::WAIT_TIME))
                    }
                }
            },
        }
    }

    pub fn collides_with(&self, player: &crate::player::Player, map: &crate::state::Map) -> bool {
        let [tw, th] = map.tile_size;
        let (px, py) = player.position();
        let [sx, sy] = self.position;
        (px - sx).abs() < tw && (py - sy).abs() < th
    }

    pub fn render(&self, ctx: &mut State) {
        match &self.ty {
            EnemyType::Basic(inner) => {
                let [tw, th] = ctx.map.tile_size;
                let [tw, th] = [tw * 0.8, th * 0.8];
                let [x1, y1] = self.position;
                let [x2, y2] = inner.prev_position;
                let angle = (y2 - y1).atan2(x2 - x1) + std::f32::consts::FRAC_PI_2;
                let transform = solstice_2d::Transform2D::translation(x1, y1)
                    * solstice_2d::Transform2D::rotation(solstice_2d::Rad(angle));
                ctx.ctx.g.image_with_transform(
                    ctx.ctx
                        .resources
                        .sprites_metadata
                        .enemy1_body
                        .with_size(tw, th)
                        .center_on(0., 0.),
                    &ctx.ctx.resources.sprites,
                    transform,
                );
                ctx.ctx.g.image_with_transform(
                    ctx.ctx
                        .resources
                        .sprites_metadata
                        .enemy1_color
                        .with_size(tw, th)
                        .center_on(0., 0.),
                    &ctx.ctx.resources.sprites,
                    transform,
                );
            }
        }
    }
}
