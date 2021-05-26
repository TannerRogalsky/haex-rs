use serde::{Deserialize, Serialize};
use solstice_2d::solstice::{self, Context};

#[derive(Serialize, Deserialize)]
struct Point<T> {
    x: T,
    y: T,
}

#[derive(Serialize, Deserialize)]
struct Rect {
    #[serde(flatten)]
    point: Point<u32>,
    #[serde(flatten)]
    size: Dimension,
}

impl From<Rect> for solstice_2d::Rectangle {
    fn from(r: Rect) -> Self {
        Self::new(
            r.point.x as f32,
            r.point.y as f32,
            r.size.w as f32,
            r.size.h as f32,
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Dimension {
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize, Deserialize)]
struct SpriteSheetEntry {
    filename: String,
    frame: Rect,
    rotated: bool,
    trimmed: bool,
    #[serde(rename = "spriteSourceSize")]
    sprite_source_size: Rect,
    #[serde(rename = "sourceSize")]
    source_size: Dimension,
    pivot: Point<f32>,
}

impl Rect {
    fn into_quad(self, src_dim: &Dimension) -> crate::UVRect {
        let Dimension { w, h } = src_dim;
        let (w, h) = (*w as f32, *h as f32);
        let uvs = solstice_2d::Rectangle::new(
            self.point.x as f32 / w,
            self.point.y as f32 / h,
            self.size.w as f32 / w,
            self.size.h as f32 / h,
        );
        let position = solstice_2d::Rectangle::new(
            self.point.x as f32,
            self.point.y as f32,
            self.size.w as f32,
            self.size.h as f32,
        );
        crate::UVRect {
            positions: position,
            uvs,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SpriteSheetMetadata {
    pub size: Dimension,
}

#[derive(Serialize, Deserialize)]
pub struct SpriteSheet {
    frames: Vec<SpriteSheetEntry>,
    meta: SpriteSheetMetadata,
}

pub enum ImageDataRepr {
    Bytes(Vec<u8>),
    #[cfg(target_arch = "wasm32")]
    ImageElement(web_sys::HtmlImageElement),
}

pub struct ImageData {
    pub data: ImageDataRepr,
    pub width: u32,
    pub height: u32,
    pub format: solstice::PixelFormat,
}

impl ImageData {
    fn try_into_image(
        self,
        ctx: &mut Context,
        nearest: bool,
    ) -> eyre::Result<solstice::image::Image> {
        use solstice::{
            image::{Image, Settings},
            texture::TextureType,
        };
        let ImageData {
            data,
            width,
            height,
            format,
        } = self;
        let settings = Settings {
            mipmaps: false,
            filter: if nearest {
                solstice::texture::FilterMode::Nearest.into()
            } else {
                solstice::texture::FilterMode::Linear.into()
            },
            wrap: solstice::texture::WrapMode::Repeat.into(),
            ..Default::default()
        };
        let img = match data {
            ImageDataRepr::Bytes(data) => Image::with_data(
                ctx,
                TextureType::Tex2D,
                format,
                width,
                height,
                &data,
                settings,
            )?,
            #[cfg(target_arch = "wasm32")]
            ImageDataRepr::ImageElement(data) => Image::with_html_image(
                ctx,
                TextureType::Tex2D,
                format,
                width,
                height,
                &data,
                settings,
            )?,
        };
        Ok(img)
    }
}

pub struct Audio {
    pub agent_smith_laugh: crate::audio::StreamingAudioSource,
    pub last_level_drone: crate::audio::StreamingAudioSource,
    pub level_finish: crate::audio::StreamingAudioSource,
    pub music: crate::audio::StreamingAudioSource,
    pub quote: crate::audio::StreamingAudioSource,
}

pub struct Resources {
    pub debug_font_data: Vec<u8>,
    pub pixel_font_data: Vec<u8>,
    pub sprites_data: ImageData,
    pub noise_data: ImageData,
    pub sprites_metadata: SpriteSheet,
    pub aesthetic_shader_src: String,
    pub menu_shader_src: String,
    pub vignette_shader_src: String,
    pub map_obscuring_shader_src: String,
    pub audio: Audio,
}

impl Resources {
    pub fn try_into_loaded(
        self,
        ctx: &mut Context,
        gfx: &mut solstice_2d::Graphics,
    ) -> eyre::Result<LoadedResources> {
        use std::convert::TryInto;

        let src_dim = self.sprites_metadata.meta.size.clone();
        let mut raw = self
            .sprites_metadata
            .frames
            .into_iter()
            .map(|entry| {
                let quad = entry.frame.into_quad(&src_dim);
                (entry.filename, quad)
            })
            .collect::<std::collections::HashMap<_, _>>();

        Ok(LoadedResources {
            debug_font: gfx.add_font(self.debug_font_data.try_into()?),
            pixel_font: gfx.add_font(self.pixel_font_data.try_into()?),
            sprites: self.sprites_data.try_into_image(ctx, true)?,
            noise: self.noise_data.try_into_image(ctx, false)?,
            sprites_metadata: Sprites::try_new(&mut raw)
                .ok_or(eyre::Report::msg("Missing sprite definition."))?,
            sprites_metadata_raw: raw,
            shaders: Shaders {
                aesthetic: solstice_2d::Shader::with(&self.aesthetic_shader_src, ctx)?,
                menu: solstice_2d::Shader::with(&self.menu_shader_src, ctx)?,
                vignette: solstice_2d::Shader::with(&self.vignette_shader_src, ctx)?,
                map_obscuring: solstice_2d::Shader::with(&self.map_obscuring_shader_src, ctx)?,
            },
            audio: self.audio,
        })
    }
}

pub struct Shaders {
    pub aesthetic: solstice_2d::Shader,
    pub menu: solstice_2d::Shader,
    pub vignette: solstice_2d::Shader,
    pub map_obscuring: solstice_2d::Shader,
}

impl Shaders {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut solstice_2d::Shader> + '_ {
        std::array::IntoIter::new([
            &mut self.aesthetic,
            &mut self.menu,
            &mut self.vignette,
            &mut self.map_obscuring,
        ])
    }
}

#[derive(PartialEq)]
pub struct Sprites {
    pub boss_alpha: crate::UVRect,
    pub boss_body: crate::UVRect,
    pub boss_color: crate::UVRect,
    pub boss_contrast: crate::UVRect,
    pub enemy1_alpha: crate::UVRect,
    pub enemy1_body: crate::UVRect,
    pub enemy1_color: crate::UVRect,
    pub enemy2_alpha: crate::UVRect,
    pub enemy2_body: crate::UVRect,
    pub enemy2_color: crate::UVRect,
    pub enemy3_alpha: crate::UVRect,
    pub enemy3_body: crate::UVRect,
    pub enemy3_color: crate::UVRect,
    pub exit_alpha: crate::UVRect,
    pub exit_body: crate::UVRect,
    pub exit_color: crate::UVRect,
    pub title_arrow: crate::UVRect,
    pub title_kyle: crate::UVRect,
    pub title_kyle_white: crate::UVRect,
    pub title_ryan: crate::UVRect,
    pub title_ryan_white: crate::UVRect,
    pub title_tanner: crate::UVRect,
    pub title_tanner_white: crate::UVRect,
    pub title_title: crate::UVRect,
}

impl Sprites {
    fn try_new(raw: &mut std::collections::HashMap<String, crate::UVRect>) -> Option<Self> {
        Some(Sprites {
            boss_alpha: raw.remove("boss_alpha.png")?,
            boss_body: raw.remove("boss_body.png")?,
            boss_color: raw.remove("boss_color.png")?,
            boss_contrast: raw.remove("boss_contrast.png")?,
            enemy1_alpha: raw.remove("enemy1_alpha.png")?,
            enemy1_body: raw.remove("enemy1_body.png")?,
            enemy1_color: raw.remove("enemy1_color.png")?,
            enemy2_alpha: raw.remove("enemy2_alpha.png")?,
            enemy2_body: raw.remove("enemy2_body.png")?,
            enemy2_color: raw.remove("enemy2_color.png")?,
            enemy3_alpha: raw.remove("enemy3_alpha.png")?,
            enemy3_body: raw.remove("enemy3_body.png")?,
            enemy3_color: raw.remove("enemy3_color.png")?,
            exit_alpha: raw.remove("exit_alpha.png")?,
            exit_body: raw.remove("exit_body.png")?,
            exit_color: raw.remove("exit_color.png")?,
            title_arrow: raw.remove("title_arrow.png")?,
            title_kyle: raw.remove("title_kyle.png")?,
            title_kyle_white: raw.remove("title_kyle_white.png")?,
            title_ryan: raw.remove("title_ryan.png")?,
            title_ryan_white: raw.remove("title_ryan_white.png")?,
            title_tanner: raw.remove("title_tanner.png")?,
            title_tanner_white: raw.remove("title_tanner_white.png")?,
            title_title: raw.remove("title_title.png")?,
        })
    }
}

pub struct LoadedResources {
    pub debug_font: solstice_2d::FontId,
    pub pixel_font: solstice_2d::FontId,
    pub sprites: solstice::image::Image,
    pub noise: solstice::image::Image,
    pub sprites_metadata: Sprites,
    pub sprites_metadata_raw: std::collections::HashMap<String, crate::UVRect>,
    pub shaders: Shaders,
    pub audio: Audio,
}
