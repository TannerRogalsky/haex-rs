use crate::resources::{ImageData, ImageDataRepr, Resources, SpriteSheet};
use wasm_bindgen::prelude::*;

fn into_js_value<E: std::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&format!("{}", err))
}

#[wasm_bindgen]
pub enum KeyEvent {
    W,
    A,
    S,
    D,
    Space,
}

impl Into<winit::event::VirtualKeyCode> for KeyEvent {
    fn into(self) -> winit::event::VirtualKeyCode {
        use winit::event::VirtualKeyCode;
        match self {
            KeyEvent::W => VirtualKeyCode::W,
            KeyEvent::A => VirtualKeyCode::A,
            KeyEvent::S => VirtualKeyCode::S,
            KeyEvent::D => VirtualKeyCode::D,
            KeyEvent::Space => VirtualKeyCode::Space,
        }
    }
}

#[wasm_bindgen]
pub struct ResourcesWrapper {
    debug_font_data: Option<Vec<u8>>,
    sprites_data: Option<ImageData>,
    noise_data: Option<ImageData>,
    sprites_metadata: Option<SpriteSheet>,
    aesthetic_shader_src: Option<String>,
    menu_shader_src: Option<String>,
}

#[wasm_bindgen]
impl ResourcesWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        ResourcesWrapper {
            debug_font_data: None,
            sprites_data: None,
            noise_data: None,
            sprites_metadata: None,
            aesthetic_shader_src: None,
            menu_shader_src: None,
        }
    }

    pub fn set_debug_font_data(&mut self, data: Vec<u8>) {
        self.debug_font_data = Some(data);
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

        let resources = Resources {
            debug_font_data: resources
                .debug_font_data
                .ok_or(JsValue::from_str("missing font data"))?,
            sprites_data: resources
                .sprites_data
                .ok_or(JsValue::from_str("missing font data"))?,
            noise_data: resources
                .noise_data
                .ok_or(JsValue::from_str("missing font data"))?,
            sprites_metadata: resources
                .sprites_metadata
                .ok_or(JsValue::from_str("missing font data"))?,
            aesthetic_shader_src: resources
                .aesthetic_shader_src
                .ok_or(JsValue::from_str("missing font data"))?,
            menu_shader_src: resources
                .menu_shader_src
                .ok_or(JsValue::from_str("missing font data"))?,
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
