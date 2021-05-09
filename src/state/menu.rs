use super::{State, StateContext};
use crate::winit::event::{ElementState, MouseButton};
use crate::MouseEvent;
use solstice_2d::{solstice, Color, Draw};

pub struct Menu;

impl Menu {
    pub fn render(&mut self, ctx: StateContext) {
        let uvs = ctx
            .resources
            .sprites_metadata
            .get("boss_contrast.png")
            .unwrap();
        let quad = uvs
            .zip(solstice::quad_batch::Quad::from(
                solstice::viewport::Viewport::new(0., 0., 256., 256.),
            ))
            .map(|((s, t), (x, y))| solstice_2d::Vertex2D {
                position: [x, y],
                uv: [s, t],
                ..Default::default()
            });

        let mut vertices = Vec::with_capacity(4);
        vertices.extend_from_slice(&quad.vertices);
        let indices = Some(vec![0, 1, 2, 2, 1, 3]);
        let quad = solstice_2d::Geometry::new(vertices, indices);

        let viewport = ctx.gfx.viewport().clone();
        const BLACK: Color = Color::new(0., 0., 0., 1.);

        let mut g = ctx.gfx.lock(ctx.ctx);
        g.clear(BLACK);

        g.set_canvas(Some(ctx.canvas.clone()));
        g.clear(BLACK);
        g.set_shader(Some(ctx.resources.shaders.menu.clone()));
        g.image(quad, &ctx.resources.sprites);

        g.set_canvas(None);
        g.set_shader(Some({
            let mut shader = ctx.resources.shaders.aesthetic.clone();
            shader.send_uniform("blockThreshold", 0.073f32);
            shader.send_uniform("lineThreshold", 0.23f32);
            shader.send_uniform("randomShiftScale", 0.002f32);
            shader.send_uniform("radialScale", 0.1f32);
            shader.send_uniform("radialBreathingScale", 0.01f32);
            let unit = 1;
            shader.bind_texture_at_location(&ctx.resources.noise, (unit as usize).into());
            shader.send_uniform("tex1", unit);
            shader
        }));

        {
            let d = viewport.width().min(viewport.height()) as f32;
            let x = viewport.width() as f32 / 2. - d / 2.;
            g.image(
                solstice_2d::Rectangle {
                    x,
                    y: 0.0,
                    width: d,
                    height: d,
                },
                ctx.canvas,
            );
        }
    }

    pub fn handle_key_event(
        &mut self,
        mut ctx: StateContext,
        _state: crate::ElementState,
        _key_code: crate::VirtualKeyCode,
    ) -> Option<State> {
        let settings = ctx.maps.clone();
        let main = super::main::Main::new(&mut ctx, settings).ok()?;
        Some(State::Main(main))
    }

    pub fn handle_mouse_event(
        &mut self,
        ctx: StateContext,
        event: crate::MouseEvent,
    ) -> Option<State> {
        match event {
            MouseEvent::Button(state, button) => {
                if state == ElementState::Pressed {
                    match button {
                        MouseButton::Left => {
                            let _r = ctx.audio_ctx.play_new(ctx.resources.music.clone());
                        }
                        _ => {}
                    }
                }
            }
            MouseEvent::Moved(_, _) => {}
        }
        // Some(State::Main)
        None
    }
}
