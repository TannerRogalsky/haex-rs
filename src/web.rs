use crate::resources::{Audio, ImageData, ImageDataRepr, Resources, SpriteSheet};
use wasm_bindgen::prelude::*;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn into_js_value<E: std::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&format!("{}", err))
}

#[wasm_bindgen(start)]
pub fn js_main() {
    #[cfg(debug_assertions)]
    let level = log::Level::Debug;
    #[cfg(not(debug_assertions))]
    let level = log::Level::Error;
    wasm_logger::init(wasm_logger::Config::new(level));
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub enum KeyEvent {
    W,
    A,
    S,
    D,
    Space,
    Ctrl,
}

impl From<KeyEvent> for winit::event::VirtualKeyCode {
    fn from(key: KeyEvent) -> Self {
        use winit::event::VirtualKeyCode;
        match key {
            KeyEvent::W => VirtualKeyCode::W,
            KeyEvent::A => VirtualKeyCode::A,
            KeyEvent::S => VirtualKeyCode::S,
            KeyEvent::D => VirtualKeyCode::D,
            KeyEvent::Space => VirtualKeyCode::Space,
            KeyEvent::Ctrl => VirtualKeyCode::LControl,
        }
    }
}

#[wasm_bindgen]
pub struct ResourcesWrapper {
    debug_font_data: Option<Vec<u8>>,
    pixel_font_data: Option<Vec<u8>>,
    sprites_data: Option<ImageData>,
    noise_data: Option<ImageData>,
    sprites_metadata: Option<SpriteSheet>,
    aesthetic_shader_src: Option<String>,
    menu_shader_src: Option<String>,
    vignette_shader_src: Option<String>,
    map_obscuring_shader_src: Option<String>,
    grayscale_shader_src: Option<String>,
    player_shader_src: Option<String>,
    music: Option<web_sys::HtmlMediaElement>,
    agent_smith_laugh: Option<web_sys::HtmlMediaElement>,
    last_level_drone: Option<web_sys::HtmlMediaElement>,
    level_finish: Option<web_sys::HtmlMediaElement>,
    quote: Option<web_sys::HtmlMediaElement>,
}

#[wasm_bindgen]
impl ResourcesWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        ResourcesWrapper {
            debug_font_data: None,
            pixel_font_data: None,
            sprites_data: None,
            noise_data: None,
            sprites_metadata: None,
            aesthetic_shader_src: None,
            menu_shader_src: None,
            vignette_shader_src: None,
            map_obscuring_shader_src: None,
            grayscale_shader_src: None,
            player_shader_src: None,
            music: None,
            agent_smith_laugh: None,
            last_level_drone: None,
            level_finish: None,
            quote: None,
        }
    }

    pub fn set_debug_font_data(&mut self, data: Vec<u8>) {
        self.debug_font_data = Some(data);
    }

    pub fn set_pixel_font_data(&mut self, data: Vec<u8>) {
        self.pixel_font_data = Some(data);
    }

    pub fn set_sprites(&mut self, image: web_sys::HtmlImageElement) {
        Self::set_image(&mut self.sprites_data, image)
    }

    pub fn set_sprites_metadata(&mut self, data: Vec<u8>) -> Result<(), JsValue> {
        let sprites_metadata = serde_json::from_slice(&data).map_err(into_js_value)?;
        self.sprites_metadata = Some(sprites_metadata);
        Ok(())
    }

    pub fn set_noise(&mut self, image: web_sys::HtmlImageElement) {
        Self::set_image(&mut self.noise_data, image)
    }

    pub fn set_aesthetic_shader(&mut self, src: String) {
        self.aesthetic_shader_src = Some(src);
    }

    pub fn set_menu_shader(&mut self, src: String) {
        self.menu_shader_src = Some(src);
    }

    pub fn set_vignette_shader(&mut self, src: String) {
        self.vignette_shader_src = Some(src);
    }

    pub fn set_map_obscuring_shader(&mut self, src: String) {
        self.map_obscuring_shader_src = Some(src);
    }

    pub fn set_grayscale_shader(&mut self, src: String) {
        self.grayscale_shader_src = Some(src);
    }

    pub fn set_player_shader(&mut self, src: String) {
        self.player_shader_src = Some(src);
    }

    pub fn set_agent_smith_laugh(&mut self, source: web_sys::HtmlMediaElement) {
        self.agent_smith_laugh = Some(source);
    }
    pub fn set_last_level_drone(&mut self, source: web_sys::HtmlMediaElement) {
        self.last_level_drone = Some(source);
    }
    pub fn set_level_finish(&mut self, source: web_sys::HtmlMediaElement) {
        self.level_finish = Some(source);
    }
    pub fn set_quote(&mut self, source: web_sys::HtmlMediaElement) {
        self.quote = Some(source);
    }
    pub fn set_music(&mut self, source: web_sys::HtmlMediaElement) {
        self.music = Some(source);
    }

    fn set_image(field: &mut Option<ImageData>, image: web_sys::HtmlImageElement) {
        let width = image.width();
        let height = image.height();
        *field = Some(ImageData {
            data: ImageDataRepr::ImageElement(image),
            width,
            height,
            format: solstice_2d::solstice::PixelFormat::RGBA8,
        });
    }
}

#[wasm_bindgen]
pub struct Wrapper {
    canvas: web_sys::HtmlCanvasElement,
    inner: crate::Game,
}

#[wasm_bindgen]
impl Wrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: web_sys::HtmlCanvasElement,
        time_ms: f64,
        resources: ResourcesWrapper,
    ) -> Result<Wrapper, JsValue> {
        let webgl_context = {
            use wasm_bindgen::JsCast;
            canvas
                .get_context("webgl")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::WebGlRenderingContext>()
                .unwrap()
        };
        let ctx = solstice_2d::solstice::glow::Context::from_webgl1_context(webgl_context);
        let ctx = solstice_2d::solstice::Context::new(ctx);

        let width = canvas.width();
        let height = canvas.height();

        fn new_audio(
            src: Option<web_sys::HtmlMediaElement>,
        ) -> Result<crate::audio::StreamingAudioSource, JsValue> {
            Ok(crate::audio::StreamingAudioSource::from_element(
                src.ok_or(JsValue::from_str("missing music data"))?,
            ))
        }

        let resources = Resources {
            debug_font_data: resources
                .debug_font_data
                .ok_or(JsValue::from_str("missing debug font data"))?,
            pixel_font_data: resources
                .pixel_font_data
                .ok_or(JsValue::from_str("missing pixel font data"))?,
            sprites_data: resources
                .sprites_data
                .ok_or(JsValue::from_str("missing sprites data"))?,
            noise_data: resources
                .noise_data
                .ok_or(JsValue::from_str("missing noise data"))?,
            sprites_metadata: resources
                .sprites_metadata
                .ok_or(JsValue::from_str("missing sprites metadata"))?,
            aesthetic_shader_src: resources
                .aesthetic_shader_src
                .ok_or(JsValue::from_str("missing aesthetic shader source"))?,
            menu_shader_src: resources
                .menu_shader_src
                .ok_or(JsValue::from_str("missing menu shader source"))?,
            vignette_shader_src: resources
                .vignette_shader_src
                .ok_or(JsValue::from_str("missing vignette shader source"))?,
            map_obscuring_shader_src: resources
                .map_obscuring_shader_src
                .ok_or(JsValue::from_str("missing map obscuring shader source"))?,
            grayscale_shader_src: resources
                .grayscale_shader_src
                .ok_or(JsValue::from_str("missing map obscuring shader source"))?,
            player_shader_src: resources
                .player_shader_src
                .ok_or(JsValue::from_str("missing player shader source"))?,
            audio: Audio {
                agent_smith_laugh: new_audio(resources.agent_smith_laugh)?,
                last_level_drone: new_audio(resources.last_level_drone)?,
                level_finish: new_audio(resources.level_finish)?,
                music: new_audio(resources.music)?,
                quote: new_audio(resources.quote)?,
            },
        };

        let time = duration_from_f64(time_ms);
        let inner = crate::Game::new(ctx, time, width as _, height as _, resources)
            .map_err(into_js_value)?;
        Ok(Self { canvas, inner })
    }

    pub fn step(&mut self, time_ms: f64) {
        self.inner.update(duration_from_f64(time_ms));
    }

    pub fn handle_resize(&mut self) {
        let width = self.canvas.width();
        let height = self.canvas.height();
        self.inner.handle_resize(width as _, height as _);
    }

    pub fn handle_key_down(&mut self, key_code: KeyEvent) {
        let state = winit::event::ElementState::Pressed;
        self.inner.handle_key_event(state, key_code.into())
    }

    pub fn handle_key_up(&mut self, key_code: KeyEvent) {
        let state = winit::event::ElementState::Released;
        self.inner.handle_key_event(state, key_code.into())
    }

    pub fn handle_mouse_down(&mut self, is_left_button: bool) {
        let state = winit::event::ElementState::Pressed;
        let button = match is_left_button {
            true => winit::event::MouseButton::Left,
            false => winit::event::MouseButton::Right,
        };
        self.inner
            .handle_mouse_event(crate::MouseEvent::Button(state, button))
    }

    pub fn handle_mouse_up(&mut self, is_left_button: bool) {
        let state = winit::event::ElementState::Released;
        let button = match is_left_button {
            true => winit::event::MouseButton::Left,
            false => winit::event::MouseButton::Right,
        };
        self.inner
            .handle_mouse_event(crate::MouseEvent::Button(state, button))
    }

    pub fn handle_mouse_move(&mut self, x: f32, y: f32) {
        self.inner
            .handle_mouse_event(crate::MouseEvent::Moved(x, y))
    }
}

fn duration_from_f64(millis: f64) -> std::time::Duration {
    std::time::Duration::from_millis(millis.trunc() as u64)
        + std::time::Duration::from_nanos((millis.fract() * 1.0e6) as u64)
}
