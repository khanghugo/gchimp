//! Code from kdr
//!
//! Disabled mipmapping
use egui_wgpu::wgpu;

#[derive(Debug, Clone)]
pub struct MipmapArrayBuffer {
    pub mipmap: wgpu::Texture,
    pub palette: wgpu::Texture,
    pub mipmap_view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
}

impl Drop for MipmapArrayBuffer {
    fn drop(&mut self) {
        self.mipmap.destroy();
        self.palette.destroy();
    }
}

impl MipmapArrayBuffer {
    pub fn bind_group_layout_descriptor() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("mipmap array bind group layout descriptor"),
            entries: &[
                // mipmap
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // palette
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // nearest sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        }
    }

    fn mipmap_texture_descriptor(
        label: &str,
        width: u32,
        height: u32,
        depth_or_array_layers: u32,
        mip_level_count: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureDescriptor {
        wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT, // to generate mipmap
            view_formats: &[],
        }
    }

    fn palette_texture_descriptor(
        label: &str,
        texture_count: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureDescriptor {
        wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: PALETTE_COUNT as u32, // just an array
                height: texture_count,       // storing palette in 256 x n texture
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            // | wgpu::TextureUsages::RENDER_ATTACHMENT // to generate mipmap
            view_formats: &[],
        }
    }
}
use std::sync::{Arc, LazyLock, Mutex};

use image::RgbaImage;

// use super::mipmap::{calculate_mipmap_count, MipMapGenerator};

static TEXTURE_ARRAY_COUNT: LazyLock<Arc<Mutex<u32>>> = LazyLock::new(|| Arc::new(Mutex::new(0)));

const PALETTE_COUNT: usize = 256;

pub struct MipmapTexture {
    pub image: Vec<u8>,
    pub palette: [[u8; 3]; PALETTE_COUNT],
    pub width: u32,
    pub height: u32,
}

impl MipmapTexture {
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

// this is assuming that they all have the same dimensions
pub fn create_mipmap_array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mipmaps: &[&MipmapTexture],
) -> Option<MipmapArrayBuffer> {
    // some checks just to make sure
    if mipmaps.is_empty() {
        println!("texture array length is 0");
        return None;
    }

    let gles_texture_array_fix = mipmaps.len() % 6 == 0 || mipmaps.len() == 1;

    if gles_texture_array_fix {
        //         warn!(
        //             "Creating a texture array with {} images. \
        // If backend is GLES, an additional image is added to the array to avoid being interpreted as a cube texture in case of multiples of 6 \
        // or a singular texture in case of 1 texture. \
        // This is an ongoing issue in wgpu (https://github.com/gfx-rs/wgpu/issues/4081).",
        //             textures.len()
        //         );
    }

    // wgpu fix
    let texture_count = mipmaps.len();
    let texture_len = if gles_texture_array_fix {
        texture_count + 1
    } else {
        texture_count
    };

    let tex0 = &mipmaps[0];
    let (width, height) = tex0.dimensions();

    if !mipmaps
        .iter()
        .all(|texture| tex0.dimensions() == texture.dimensions())
    {
        println!("not all textures have the same dimensions");
        return None;
    }

    // TODO: mipmapping
    let _mip_level_count = calculate_mipmap_count(width, height);
    let mip_level_count = 1;

    // mipmap is 8bpp + palette
    let mipmap_texture_format = wgpu::TextureFormat::R8Unorm;
    let mipmap_palette_format = wgpu::TextureFormat::Rgba8Unorm;

    // TODO: mipmapping
    // let mipmap_generator = MipMapGenerator::create_render_pipeline(device, queue, texture_format);

    let get_texture_array_label = || {
        let mut mipmap_array_count = TEXTURE_ARRAY_COUNT.lock().unwrap();

        let mipmap_label = format!("mipmap texture array {}", mipmap_array_count);
        let palette_label = format!("mipmap palette array {}", mipmap_array_count);

        *mipmap_array_count += 1;

        (mipmap_label, palette_label)
    };

    let (mipmap_texture_label, mipmap_palette_label) = get_texture_array_label();

    let mipmap_texture_descriptor = MipmapArrayBuffer::mipmap_texture_descriptor(
        mipmap_texture_label.as_str(),
        width,
        height,
        texture_len as u32,
        mip_level_count,
        mipmap_texture_format,
    );

    let palette_texture_descriptor = MipmapArrayBuffer::palette_texture_descriptor(
        mipmap_palette_label.as_str(),
        texture_count as u32,
        mipmap_palette_format,
    );

    let mipmap_texture = device.create_texture(&mipmap_texture_descriptor);
    let palette_texture = device.create_texture(&palette_texture_descriptor);

    mipmaps.iter().enumerate().for_each(|(layer_idx, mipmap)| {
        // mipmap textures
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &mipmap_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer_idx as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &mipmap.image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width), // u8 image
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let palette_data: Vec<u8> = mipmap
            .palette
            .iter()
            .flat_map(|x| [x[0], x[1], x[2], 255])
            .collect();

        // palette textures
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &palette_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: layer_idx as u32, // write to each row
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &palette_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PALETTE_COUNT as u32 * 4), // rgba8
                rows_per_image: Some(1),                       // single row
            },
            // region to be written
            wgpu::Extent3d {
                width: PALETTE_COUNT as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    });

    // An additional image should be added here. However, things just automagically work. so there is no need.
    // if gles_cube_texture_fix {
    //     // add a dummy image
    // }

    // TODO: mipmapping
    // mipmap_generator.generate_mipmaps_texture_array(
    //     &texture_array,
    //     mip_level_count,
    //     texture_len as u32,
    // );

    // bind layout
    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("texture array nearest sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: (mip_level_count as f32 - 5.0).max(0.0), // change the max mipmap level here
        ..Default::default()
    });

    let mipmap_view = mipmap_texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("mipmap array view"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(mip_level_count),
        base_array_layer: 0,
        array_layer_count: Some(texture_len as u32),
        usage: None,
    });

    let palette_view = palette_texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("palette array view"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
        usage: None,
    });

    let bind_group_layout =
        device.create_bind_group_layout(&MipmapArrayBuffer::bind_group_layout_descriptor());

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("texture bind group"),
        layout: &bind_group_layout,
        entries: &[
            // texture
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&mipmap_view),
            },
            // palette
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&palette_view),
            },
            // nearest sampler
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&nearest_sampler),
            },
        ],
    });

    Some(MipmapArrayBuffer {
        mipmap: mipmap_texture,
        palette: palette_texture,
        mipmap_view,
        bind_group,
    })
}

pub fn calculate_mipmap_count(width: u32, height: u32) -> u32 {
    (width.max(height) as f32).log2().floor() as u32 + 1
}
