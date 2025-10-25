use std::collections::HashMap;

use cgmath::EuclideanSpace;
use eframe::wgpu::util::DeviceExt;
use egui_wgpu::wgpu;
use image::RgbaImage;
use mdl::Trivert;

use crate::gui::programs::mdlscrub::{
    render::{
        camera::ScrubCamera,
        mdl_buffer::{dynamic_buffer::BatchLookup, MdlVertexBuffer},
        mipmap_array::MipmapTexture,
    },
    tile::SCRUB_TILE_SIZE,
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

pub fn get_mdl_mipmaps(mdl: &mdl::Mdl) -> Vec<MipmapTexture> {
    mdl.textures
        .iter()
        .map(|texture| MipmapTexture {
            image: texture.image.clone(),
            palette: texture.palette,
            width: texture.dimensions().0,
            height: texture.dimensions().1,
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

pub fn set_up_camera_values_for_mdl(mdl: &mdl::Mdl) -> ScrubCamera {
    let (lowest, highest) = mdl
        .bodyparts
        .iter()
        .flat_map(|bodypart| &bodypart.models)
        .take(1)
        .flat_map(|model| &model.meshes)
        .flat_map(|mesh| &mesh.triangles)
        .flat_map(|mesh_triangle| mesh_triangle.get_triverts())
        .fold(
            (
                cgmath::Vector3::<f32>::new(f32::MAX, f32::MAX, f32::MAX),
                cgmath::Vector3::<f32>::new(f32::MIN, f32::MIN, f32::MIN),
            ),
            |(low, high), trivert| {
                let vertex = trivert.vertex;

                (
                    cgmath::Vector3 {
                        x: low.x.min(vertex.x),
                        y: low.y.min(vertex.y),
                        z: low.z.min(vertex.z),
                    },
                    cgmath::Vector3 {
                        x: high.x.max(vertex.x),
                        y: high.y.max(vertex.y),
                        z: high.z.max(vertex.z),
                    },
                )
            },
        );

    // basic target to look at
    let mut camera = ScrubCamera::default();

    let center = (highest + lowest) / 2.;
    camera.target = cgmath::Point3::from_vec(center);

    // now i want to change things a bit so that it looks nicer, will be moving the camera position on a sphere with "center" as the center
    // and "distance" as the radius
    let box_size = highest - lowest;
    let max_dimension = box_size.x.max(box_size.y).max(box_size.z);
    const FOV_Y: f32 = 90.;

    let distance = max_dimension
        / 2.
        / (FOV_Y / 2.)
            .to_radians()
            // use sin for the worst case
            .sin();
    let distance = distance * 1.2; // add some more padding

    const VERTICAL_ROT: f32 = 45f32.to_radians();
    const HORIZONTAL_ROT: f32 = 45f32.to_radians();

    let camera_pos = cgmath::point3(
        center.x + distance * VERTICAL_ROT.sin() * HORIZONTAL_ROT.cos(),
        center.y + distance * VERTICAL_ROT.cos() * HORIZONTAL_ROT.sin(),
        center.z + distance * VERTICAL_ROT.cos(),
    );

    camera.pos = camera_pos;

    // now i have to make sure other values are correct
    camera.fovy = cgmath::Deg(FOV_Y);
    camera.aspect = SCRUB_TILE_SIZE / SCRUB_TILE_SIZE;

    camera
}
