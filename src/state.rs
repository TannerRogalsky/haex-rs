pub enum State {
    Menu(Menu),
    Main,
    // Over,
}

impl State {
    pub fn new() -> Self {
        Self::Menu(Menu)
    }

    pub fn handle_mouse_event(&mut self, event: crate::MouseEvent) {
        match self {
            State::Menu(inner) => {
                if let Some(new_state) = inner.handle_mouse_event(event) {
                    *self = new_state;
                }
            }
            State::Main { .. } => {}
        }
    }
}

pub struct Menu;

impl Menu {
    pub fn handle_mouse_event(&mut self, _event: crate::MouseEvent) -> Option<State> {
        Some(State::Main)
    }
}
