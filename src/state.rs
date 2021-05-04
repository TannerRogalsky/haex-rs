mod main;
mod menu;

pub struct StateContext<'a> {
    pub resources: &'a crate::resources::LoadedResources,
    pub ctx: &'a mut solstice_2d::solstice::Context,
    pub gfx: &'a mut solstice_2d::Graphics,
    pub canvas: &'a solstice_2d::Canvas,
    pub input_state: &'a crate::InputState,
    pub time: std::time::Duration,
}

pub struct Map {
    pub map: crate::map::Map,
    pub batch: solstice_2d::solstice::quad_batch::QuadBatch<solstice_2d::Vertex2D>,
    pub tile_size: (f32, f32),
}

impl Map {
    pub fn coord_to_mid_pixel(&self, coord: crate::map::Coord) -> (f32, f32) {
        self.scale((coord.0 as f32 + 0.5, coord.1 as f32 + 0.5))
    }

    fn scale(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (x * self.tile_size.0, y * self.tile_size.1)
    }

    pub fn pixel_to_coord(&self, (x, y): (f32, f32)) -> crate::map::Coord {
        let x = (x / self.tile_size.0).floor() as usize;
        let y = (y / self.tile_size.1).floor() as usize;
        (x, y)
    }
}

pub enum State {
    Menu(menu::Menu),
    Main(main::Main),
    // Over,
}

impl State {
    pub fn new() -> Self {
        Self::Menu(menu::Menu)
    }

    pub fn update(&mut self, dt: std::time::Duration, ctx: StateContext) {
        match self {
            State::Menu(_) => {}
            State::Main(main) => main.update(dt, ctx),
        }
    }

    pub fn render(&mut self, ctx: StateContext) {
        match self {
            State::Menu(menu) => menu.render(ctx),
            State::Main(main) => main.render(ctx),
        }
    }

    pub fn handle_mouse_event(&mut self, _ctx: StateContext, event: crate::MouseEvent) {
        match self {
            State::Menu(inner) => {
                if let Some(new_state) = inner.handle_mouse_event(event) {
                    *self = new_state;
                }
            }
            State::Main(_) => {}
        }
    }

    pub fn handle_key_event(
        &mut self,
        ctx: StateContext,
        state: crate::ElementState,
        key_code: crate::VirtualKeyCode,
    ) {
        match self {
            State::Menu(menu) => {
                if let Some(new_state) = menu.handle_key_event(ctx, state, key_code) {
                    *self = new_state;
                }
            }
            State::Main(_) => {}
        }
    }
}
