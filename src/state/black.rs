pub struct Black {
    wait_time: std::time::Duration,
    t: std::time::Duration,
}

impl Black {
    pub fn new(wait_time: std::time::Duration) -> Self {
        Self {
            wait_time,
            t: Default::default(),
        }
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.t += dt;
    }

    pub fn handle_key_event(
        &self,
        state: crate::ElementState,
        _key_code: crate::VirtualKeyCode,
    ) -> Option<super::State> {
        if state == crate::ElementState::Released {
            if self.t >= self.wait_time {
                Some(super::State::default())
            } else {
                None
            }
        } else {
            None
        }
    }
}
