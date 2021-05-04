use serde::{Deserialize, Serialize};
use solstice_2d::solstice::{self, quad_batch::Quad, Context};
use std::convert::TryInto;

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
    fn into_quad(self, src_dim: &Dimension) -> Quad<(f32, f32)> {
        let Dimension { w, h } = src_dim;
        let (w, h) = (*w as f32, *h as f32);
        use solstice::viewport::Viewport as Vp;
        Quad::from(Vp::new(
            self.point.x as f32 / w,
            self.point.y as f32 / h,
            self.size.w as f32 / w,
            self.size.h as f32 / h,
        ))
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

pub struct ImageData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: solstice::PixelFormat,
}

impl ImageData {
    fn try_into_image(self, ctx: &mut Context) -> eyre::Result<solstice::image::Image> {
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
        Ok(Image::with_data(
            ctx,
            TextureType::Tex2D,
            format,
            width,
            height,
            &data,
            Settings {
                mipmaps: false,
                wrap: solstice::texture::WrapMode::Repeat.into(),
                ..Default::default()
            },
        )?)
    }
}

pub struct Resources {
    pub debug_font_data: Vec<u8>,
    pub sprites_data: ImageData,
    pub noise_data: ImageData,
    pub sprites_metadata: SpriteSheet,
    pub aesthetic_shader_src: String,
    pub menu_shader_src: String,
}

impl Resources {
    pub fn try_into_loaded(
        self,
        ctx: &mut Context,
        gfx: &mut solstice_2d::Graphics,
    ) -> eyre::Result<LoadedResources> {
        Ok(LoadedResources {
            debug_font: gfx.add_font(self.debug_font_data.try_into()?),
            sprites: self.sprites_data.try_into_image(ctx)?,
            noise: self.noise_data.try_into_image(ctx)?,
            sprites_metadata: {
                let src_dim = self.sprites_metadata.meta.size.clone();
                self.sprites_metadata
                    .frames
                    .into_iter()
                    .map(|entry| {
                        let quad = entry.frame.into_quad(&src_dim);
                        (entry.filename, quad)
                    })
                    .collect()
            },
            shaders: Shaders {
                aesthetic: solstice_2d::Shader::with(&self.aesthetic_shader_src, ctx)?,
                menu: solstice_2d::Shader::with(&self.menu_shader_src, ctx)?,
            },
        })
    }
}

pub struct Shaders {
    pub aesthetic: solstice_2d::Shader,
    pub menu: solstice_2d::Shader,
}

impl Shaders {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut solstice_2d::Shader> + '_ {
        std::array::IntoIter::new([&mut self.aesthetic, &mut self.menu])
    }
}

pub struct LoadedResources {
    pub debug_font: solstice_2d::FontId,
    pub sprites: solstice::image::Image,
    pub noise: solstice::image::Image,
    pub sprites_metadata: std::collections::HashMap<String, Quad<(f32, f32)>>,
    pub shaders: Shaders,
}
