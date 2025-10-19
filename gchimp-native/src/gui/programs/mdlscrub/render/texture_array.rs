//! Code from kdr
//!
//! Disabled mipmapping
use egui_wgpu::wgpu;

#[derive(Debug, Clone)]
pub struct TextureArrayBuffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
}

impl Drop for TextureArrayBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}

impl TextureArrayBuffer {
    pub fn bind_group_layout_descriptor() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("texture array bind group layout descriptor"),
            entries: &[
                // texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // linear sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // nearest sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        }
    }
}

use std::sync::{Arc, LazyLock, Mutex};

use eyre::eyre;
use image::RgbaImage;

// use super::mipmap::{calculate_mipmap_count, MipMapGenerator};

static TEXTURE_ARRAY_COUNT: LazyLock<Arc<Mutex<u32>>> = LazyLock::new(|| Arc::new(Mutex::new(0)));

// this is assuming that they all have the same dimensions
pub fn create_texture_array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: &[&RgbaImage],
) -> Option<TextureArrayBuffer> {
    // some checks just to make sure
    if textures.is_empty() {
        println!("texture array length is 0");
        return None;
    }

    let gles_texture_array_fix = textures.len() % 6 == 0 || textures.len() == 1;

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
    let texture_len = textures.len();
    let texture_len = if gles_texture_array_fix {
        texture_len + 1
    } else {
        texture_len
    };

    let tex0 = &textures[0];
    let (width, height) = tex0.dimensions();

    if !textures
        .iter()
        .all(|texture| tex0.dimensions() == texture.dimensions())
    {
        println!("not all textures have the same dimensions");
        return None;
    }

    // TODO: mipmapping
    let _mip_level_count = calculate_mipmap_count(width, height);
    let mip_level_count = 1;

    let texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;

    // TODO: mipmapping
    // let mipmap_generator = MipMapGenerator::create_render_pipeline(device, queue, texture_format);

    let get_texture_array_label = || {
        let mut texture_array_count = TEXTURE_ARRAY_COUNT.lock().unwrap();

        let label = format!("texture array {}", texture_array_count);

        *texture_array_count += 1;

        label
    };

    let label = get_texture_array_label();

    let texture_descriptor = wgpu::TextureDescriptor {
        label: Some(label.as_str()),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: texture_len as u32,
        },
        mip_level_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: texture_format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT, // to generate mipmap
        view_formats: &[],
    };

    let texture_array = device.create_texture(&texture_descriptor);

    textures
        .iter()
        .enumerate()
        .for_each(|(layer_idx, texture)| {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture_array,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer_idx as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &texture,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
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
    let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("texture array linear sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        anisotropy_clamp: 16,
        lod_min_clamp: 0.0,
        lod_max_clamp: (mip_level_count as f32 - 5.0).max(0.0), // change the max mipmap level here
        ..Default::default()
    });

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

    let view = texture_array.create_view(&wgpu::TextureViewDescriptor {
        label: Some("texture array view"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(mip_level_count),
        base_array_layer: 0,
        array_layer_count: Some(texture_len as u32),
        usage: None,
    });

    let bind_group_layout =
        device.create_bind_group_layout(&TextureArrayBuffer::bind_group_layout_descriptor());

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("texture bind group"),
        layout: &bind_group_layout,
        entries: &[
            // texture
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            // linear sampler
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&linear_sampler),
            },
            // nearest sampler
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&nearest_sampler),
            },
        ],
    });

    Some(TextureArrayBuffer {
        texture: texture_array,
        view,
        bind_group,
    })
}

pub fn calculate_mipmap_count(width: u32, height: u32) -> u32 {
    (width.max(height) as f32).log2().floor() as u32 + 1
}
