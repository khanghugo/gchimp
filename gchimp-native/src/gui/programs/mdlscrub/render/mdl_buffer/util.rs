use std::collections::HashMap;

use eframe::wgpu::util::DeviceExt;
use egui_wgpu::wgpu;
use image::RgbaImage;
use mdl::Trivert;

use crate::gui::programs::mdlscrub::render::mdl_buffer::{
    dynamic_buffer::BatchLookup, MdlVertexBuffer,
};

pub fn triangulate_mdl_triverts(
    index_buffer: &mut Vec<u32>,
    triverts: &Vec<Trivert>,
    is_strip: bool,
    offset: usize,
) {
    if is_strip {
        for i in 0..triverts.len().saturating_sub(2) {
            let v1 = offset + i;
            let v2 = offset + i + 1;
            let v3 = offset + i + 2;

            if i % 2 == 0 {
                // Even-indexed triangles
                index_buffer.push(v1 as u32);
                index_buffer.push(v2 as u32);
                index_buffer.push(v3 as u32);
            } else {
                // Odd-indexed triangles (flip winding order)
                index_buffer.push(v2 as u32);
                index_buffer.push(v1 as u32);
                index_buffer.push(v3 as u32);
            }
        }
    } else {
        let first_index = offset as u32;
        for i in 1..triverts.len().saturating_sub(1) {
            index_buffer.push(first_index);
            index_buffer.push((offset + i) as u32);
            index_buffer.push((offset + i + 1) as u32);
        }
    }
}

pub fn get_mdl_textures(mdl: &mdl::Mdl) -> Vec<RgbaImage> {
    mdl.textures
        .iter()
        .map(|texture| {
            eightbpp_to_rgba8(
                &texture.image,
                &texture.palette,
                texture.dimensions().0,
                texture.dimensions().1,
                None,
            )
        })
        .collect()
}

fn most_repeating_number<T>(a: &[T]) -> T
where
    T: std::hash::Hash + Eq + Copy,
{
    let mut h: HashMap<T, u32> = HashMap::new();
    for x in a {
        *h.entry(*x).or_insert(0) += 1;
    }
    let mut r: Option<T> = None;
    let mut m: u32 = 0;
    for (x, y) in h.iter() {
        if *y > m {
            m = *y;
            r = Some(*x);
        }
    }
    r.unwrap()
}

const VERY_BLUE: [u8; 3] = [0, 0, 255];

/// This does some tricks to render masked texture, read the code
pub fn eightbpp_to_rgba8(
    img: &[u8],
    palette: &[[u8; 3]],
    width: u32,
    height: u32,
    override_alpha: Option<u8>,
) -> RgbaImage {
    // very dumb hack, but what can i do
    // the alternative way i can think of is to do two textures, 1 for index, 1 for palette
    // but with that, it will be very hard to do simple thing such as texture filtering
    let is_probably_masked_image = most_repeating_number(img) == 255;

    RgbaImage::from_raw(
        width,
        height,
        img.iter()
            .flat_map(|&idx| {
                let color = palette[idx as usize];

                // due to how we do our data, we don't know how to render entities
                // we only know the texture at this stage
                // that means, we cannot assume that the texture is supposed to be alpha tested
                // so here, we will go against our idea and assume it anyway
                // maybe in the future, we might need to add more colors
                let is_blue = color == VERY_BLUE;

                if idx == 255 && (is_probably_masked_image || is_blue) {
                    [0, 0, 0, 0]
                } else {
                    [color[0], color[1], color[2], override_alpha.unwrap_or(255)]
                }
            })
            .collect(),
    )
    .expect("cannot create rgba8 from 8pp")
}

pub fn create_mdl_vertex_buffer(
    device: &wgpu::Device,
    batch_lookup: BatchLookup,
) -> Vec<MdlVertexBuffer> {
    batch_lookup
        .into_iter()
        .map(|(texture_array_index, (vertices, indices))| {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world vertex buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

            MdlVertexBuffer {
                vertex_buffer,
                index_buffer,
                index_count: indices.len(),
                texture_array_index,
            }
        })
        .collect()
}
