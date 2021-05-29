use haex::*;
use image::DynamicImage;

fn main() -> eyre::Result<()> {
    let (width, height) = (1280, 720);
    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("HAEX")
        .with_inner_size(glutin::dpi::PhysicalSize::new(width, height));
    let (glow_ctx, window) = window::init_ctx(wb, &event_loop);
    let ctx = solstice_2d::solstice::Context::new(glow_ctx);

    let resources_folder = std::path::PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("docs");
    let fonts_folder = resources_folder.join("fonts");
    let images_folder = resources_folder.join("images");
    let shaders_folder = resources_folder.join("shaders");
    let sounds_folder = resources_folder.join("sounds");

    let new_audio = |path: &str| -> Result<audio::StreamingAudioSource, std::io::Error> {
        Ok(audio::StreamingAudioSource::from_data(std::fs::read(
            sounds_folder.join(path),
        )?))
    };

    let resources = resources::Resources {
        debug_font_data: std::fs::read(fonts_folder.join("Inconsolata-Regular.ttf"))?,
        pixel_font_data: std::fs::read(fonts_folder.join("04b03.ttf"))?,
        sprites_data: image(images_folder.join("sprites.png"))?,
        noise_data: image(images_folder.join("noise.png"))?,
        sprites_metadata: serde_json::from_reader(std::fs::File::open(
            images_folder.join("sprites.json"),
        )?)?,
        aesthetic_shader_src: std::fs::read_to_string(shaders_folder.join("aesthetic.glsl"))?,
        menu_shader_src: std::fs::read_to_string(shaders_folder.join("menu.glsl"))?,
        vignette_shader_src: std::fs::read_to_string(shaders_folder.join("vignette.glsl"))?,
        map_obscuring_shader_src: std::fs::read_to_string(
            shaders_folder.join("map_obscuring.glsl"),
        )?,
        grayscale_shader_src: std::fs::read_to_string(shaders_folder.join("grayscale.glsl"))?,
        player_shader_src: std::fs::read_to_string(shaders_folder.join("player.glsl"))?,
        audio: resources::Audio {
            agent_smith_laugh: new_audio("agent_smith_laugh.ogg")?,
            last_level_drone: new_audio("last_level_drone.ogg")?,
            level_finish: new_audio("level_finish.ogg")?,
            music: new_audio("music.ogg")?,
            quote: new_audio("quote.ogg")?,
        },
    };

    let now = {
        let epoch = std::time::Instant::now();
        move || epoch.elapsed()
    };

    let mut game = Game::new(ctx, now(), width as _, height as _, resources)?;

    event_loop.run(move |event, _, cf| {
        use glutin::{event::*, event_loop::ControlFlow};
        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    game.handle_resize(size.width as _, size.height as _);
                }
                WindowEvent::CloseRequested => {
                    *cf = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(key_code),
                            ..
                        },
                    ..
                } => game.handle_key_event(state, key_code),
                WindowEvent::MouseInput { state, button, .. } => {
                    game.handle_mouse_event(MouseEvent::Button(state, button));
                }
                WindowEvent::CursorMoved { position, .. } => {
                    game.handle_mouse_event(MouseEvent::Moved(position.x as _, position.y as _));
                }
                _ => {}
            },
            Event::DeviceEvent { .. } => {}
            Event::UserEvent(_) => {}
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                game.update(now());
                window.swap_buffers().expect("omfg");
            }
            Event::RedrawEventsCleared => {}
            Event::LoopDestroyed => {}
        }
    });
}

fn image<P: AsRef<std::path::Path>>(path: P) -> eyre::Result<resources::ImageData> {
    match image::io::Reader::open(path)?.decode()? {
        DynamicImage::ImageRgb8(image) => Ok(resources::ImageData {
            width: image.width(),
            height: image.height(),
            format: solstice_2d::solstice::PixelFormat::RGB8,
            data: resources::ImageDataRepr::Bytes(image.into_raw()),
        }),
        DynamicImage::ImageRgba8(image) => Ok(resources::ImageData {
            width: image.width(),
            height: image.height(),
            format: solstice_2d::solstice::PixelFormat::RGBA8,
            data: resources::ImageDataRepr::Bytes(image.into_raw()),
        }),
        _ => Err(eyre::Report::msg("Unsupported image format.")),
    }
}

mod window {
    mod native {
        use glutin as winit;
        use solstice_2d::solstice::glow::Context;
        use winit::{
            event_loop::EventLoop,
            window::{Window, WindowBuilder},
        };

        type WindowContext = winit::ContextWrapper<winit::PossiblyCurrent, winit::window::Window>;

        pub struct NativeWindow {
            inner: WindowContext,
        }

        impl NativeWindow {
            pub fn new(inner: WindowContext) -> Self {
                Self { inner }
            }

            pub fn swap_buffers(&self) -> eyre::Result<()> {
                self.inner.swap_buffers().map_err(eyre::Report::new)
            }
        }

        impl std::ops::Deref for NativeWindow {
            type Target = Window;

            fn deref(&self) -> &Self::Target {
                &self.inner.window()
            }
        }

        pub fn init_ctx(wb: WindowBuilder, el: &EventLoop<()>) -> (Context, NativeWindow) {
            let windowed_context = winit::ContextBuilder::new()
                .with_multisampling(16)
                .with_vsync(true)
                .build_windowed(wb, &el)
                .unwrap();
            let windowed_context = unsafe { windowed_context.make_current().unwrap() };
            let gfx = unsafe {
                Context::from_loader_function(|s| windowed_context.get_proc_address(s) as *const _)
            };
            (gfx, NativeWindow::new(windowed_context))
        }
    }

    pub use {
        glutin as winit,
        native::{init_ctx, NativeWindow as Window},
    };
}
