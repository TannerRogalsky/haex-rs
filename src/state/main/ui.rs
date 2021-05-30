use crate::winit::event::ElementState;
use solstice_2d::{Draw, Stroke};

pub struct Timer {
    time: std::time::Duration,
    elapsed: std::time::Duration,
}

impl Timer {
    fn new(time: std::time::Duration) -> Self {
        Self {
            time,
            elapsed: Default::default(),
        }
    }

    fn ratio(&self) -> f32 {
        self.elapsed.as_secs_f32() / self.time.as_secs_f32()
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.elapsed += dt;
    }
}

#[derive(Default)]
pub struct Open {
    selected: usize,
}

pub enum UIState {
    Closed,
    Opening(Timer),
    Open(Open),
    Closing(Timer),
}

impl UIState {
    const TRANSITION_TIME: std::time::Duration = std::time::Duration::from_millis(200);

    pub fn is_open(&self) -> bool {
        match self {
            UIState::Closed => false,
            _ => true,
        }
    }

    pub fn handle_key_event(
        &mut self,
        state: crate::ElementState,
        key_code: crate::VirtualKeyCode,
        prog_state: crate::programs::StateMut,
    ) -> Option<crate::cron::ID> {
        if let UIState::Open(open) = self {
            match state {
                ElementState::Pressed => match key_code {
                    crate::VirtualKeyCode::W => {
                        open.selected = open.selected.saturating_sub(1);
                    }
                    crate::VirtualKeyCode::S => {
                        open.selected += 1;
                        open.selected = open.selected.min(2);
                    }
                    crate::VirtualKeyCode::D => {
                        if open.selected == 0 {
                            let r = crate::programs::NopSlide::new(prog_state);
                            return Some(r.callback);
                        } else if open.selected == 1 {
                            crate::programs::NoClip::new(prog_state);
                        }
                    }
                    _ => {}
                },
                ElementState::Released => {}
            }
        }

        None
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        match self {
            UIState::Closed => {}
            UIState::Opening(timer) => {
                timer.update(dt);
                if timer.elapsed >= timer.time {
                    *self = UIState::Open(Default::default());
                }
            }
            UIState::Open(_) => {}
            UIState::Closing(timer) => {
                timer.update(dt);
                if timer.elapsed >= timer.time {
                    *self = UIState::Closed;
                }
            }
        }
    }

    pub fn set_open(&mut self, open: bool) {
        match self {
            UIState::Closed => {
                if open {
                    *self = UIState::Opening(Timer::new(Self::TRANSITION_TIME));
                }
            }
            UIState::Opening(timer) => {
                if !open {
                    *self = UIState::Closing(Timer {
                        time: Self::TRANSITION_TIME,
                        elapsed: Self::TRANSITION_TIME - timer.elapsed,
                    });
                }
            }
            UIState::Open(_) => {
                if !open {
                    *self = UIState::Closing(Timer::new(Self::TRANSITION_TIME))
                }
            }
            UIState::Closing(timer) => {
                if open {
                    *self = UIState::Opening(Timer {
                        time: Self::TRANSITION_TIME,
                        elapsed: Self::TRANSITION_TIME - timer.elapsed,
                    });
                }
            }
        }
    }

    pub fn render(
        &self,
        g: &mut solstice_2d::GraphicsLock,
        resources: &crate::resources::LoadedResources,
        player: &crate::player::Player,
    ) {
        use solstice_2d::Rectangle;
        const BG: solstice_2d::Color = solstice_2d::Color::new(0.2, 0.2, 0.2, 1.);
        const WHITE: solstice_2d::Color = solstice_2d::Color::new(1., 1., 1., 1.);

        const SCALE: f32 = 8.;
        const OPEN_RECT: Rectangle = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.,
            height: 100.,
        };
        const CLOSED_RECT: Rectangle = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 50.,
            height: SCALE * 1.5,
        };
        fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
            return v0 + t * (v1 - v0);
        }
        fn rect_lerp(v0: &Rectangle, v1: &Rectangle, t: f32) -> Rectangle {
            let x = lerp(v0.x, v1.x, t);
            let y = lerp(v0.y, v1.y, t);
            let width = lerp(v0.width, v1.width, t);
            let height = lerp(v0.height, v1.height, t);
            Rectangle {
                x,
                y,
                width,
                height,
            }
        }

        match self {
            UIState::Closed => {
                g.draw_with_color(CLOSED_RECT, BG);
                g.stroke_with_color(CLOSED_RECT, WHITE);
                let count = player.programs.nop_slide;
                g.print(
                    format!("PROGS {}", count),
                    resources.pixel_font,
                    SCALE,
                    CLOSED_RECT,
                );
            }
            UIState::Opening(timer) => {
                let ratio = timer.ratio();
                let rect = rect_lerp(&CLOSED_RECT, &OPEN_RECT, ratio);
                g.draw_with_color(rect, BG);
                g.stroke_with_color(rect, WHITE);
            }
            UIState::Open(state) => {
                g.draw_with_color(OPEN_RECT, BG);
                g.stroke_with_color(OPEN_RECT, WHITE);

                fn text_bounds(index: usize) -> Rectangle {
                    let index = index as f32;
                    let mut rect = OPEN_RECT;
                    rect.x += 12.;
                    rect.y += 6. + SCALE * index;
                    rect.width -= 6. * 2.;
                    rect.height -= 6. + SCALE * index;
                    rect
                }

                let font_id = resources.pixel_font;
                let programs = &player.programs;
                let text = [
                    format!("nop_slide: {}", programs.nop_slide),
                    format!("noclip: {}", programs.clip_count),
                ];
                let count = text.len();
                for (index, text) in std::array::IntoIter::new(text).enumerate() {
                    g.print(text, font_id, SCALE, text_bounds(index));
                }
                g.print("EOF", font_id, SCALE, text_bounds(count));
                g.set_color([1., 1., 0., 1.]);
                g.print(">", font_id, SCALE, {
                    let mut b = text_bounds(state.selected);
                    b.x -= 6.;
                    b
                });
                g.set_color([1., 1., 1., 1.]);
            }
            UIState::Closing(timer) => {
                let ratio = timer.ratio();
                let rect = rect_lerp(&OPEN_RECT, &CLOSED_RECT, ratio);
                g.draw_with_color(rect, BG);
                g.stroke_with_color(rect, WHITE);
            }
        }
    }
}
