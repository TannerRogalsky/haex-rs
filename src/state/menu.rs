use super::{Map, State, StateContext};
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
        ctx: StateContext,
        _state: crate::ElementState,
        _key_code: crate::VirtualKeyCode,
    ) -> Option<State> {
        let mut rng = {
            use rand::SeedableRng;
            rand::rngs::SmallRng::seed_from_u64(2)
        };

        let map = {
            let (width, height) = (10, 10);
            let tile_width = 256. / width as f32;
            let tile_height = 256. / height as f32;
            let map = crate::map::Map::new(width, height, &mut rng);
            let batch = crate::map::create_batch(
                tile_width,
                tile_height,
                &map,
                &ctx.resources.sprites_metadata,
            );
            let mut sp = solstice::quad_batch::QuadBatch::new(ctx.ctx, batch.len()).ok()?;
            for quad in batch {
                sp.push(quad);
            }
            Map {
                map,
                batch: sp,
                tile_size: (tile_width, tile_height),
            }
        };

        let player = {
            let start = map.map.path()[0];
            let (x, y) = map.coord_to_mid_pixel(start);
            crate::player::Player::new(x, y)
        };

        Some(State::Main(super::main::Main { map, player }))
    }

    pub fn handle_mouse_event(&mut self, _event: crate::MouseEvent) -> Option<State> {
        // Some(State::Main)
        None
    }
}
