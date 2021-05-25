use crate::{
    map::Direction,
    player::*,
    state::{Map, State, StateContext},
    CronContext,
};

pub struct StateMut<'a, 'b, 'c, 'd> {
    pub ctx: &'a mut StateContext<'b, 'c, 'd>,
    pub player: &'a mut Player,
    pub map: &'a mut Map,
}

pub struct NopSlide {
    pub callback: crate::cron::ID,
}

impl NopSlide {
    pub fn new(state: StateMut) -> Self {
        let origin = state.map.pixel_to_coord(state.player.position());
        let mut index = 0;

        let step = std::time::Duration::from_secs_f32(0.1);
        let id = state.ctx.cron.every(step, move |ctx: &mut CronContext| {
            let tiles = &ctx.shared.resources.sprites_metadata_raw;
            match &mut ctx.game_state {
                Some(State::Main(main)) => {
                    let mut changed = false;
                    let dirs = std::array::IntoIter::new(Direction::cardinals());
                    for direction in dirs {
                        use crate::map::*;
                        let cell = neighbor_coord_mult(origin, direction, index);
                        if let Ok(cell) = cell {
                            let [tile_width, tile_height] = main.map.inner.tile_size;
                            main.map.inner.grid.make_open(cell, direction);
                            let batch =
                                create_batch(tile_width, tile_height, &main.map.inner.grid, tiles);
                            main.map.inner.batch.clear();
                            for quad in batch {
                                main.map.inner.batch.push(quad);
                            }

                            if main.map.inner.grid.contains(cell) {
                                changed = true
                            }
                        }
                    }
                    index += 1;

                    if changed {
                        crate::cron::ControlFlow::Continue
                    } else {
                        crate::cron::ControlFlow::Stop
                    }
                }
                _ => crate::cron::ControlFlow::Stop,
            }
        });

        Self { callback: id }
    }
}
