use cgmath::Zero;
use eframe::wgpu::util::DeviceExt;
use egui_wgpu::wgpu;

// each model has at most 64 bones so...
const MAX_MVP: usize = 64;

// this should work for bsp as well because we will have func_rotating_door and whatever
#[derive(Debug, Clone)]
pub struct MvpBuffer {
    pub bind_group: wgpu::BindGroup,
    // mvp buffer for basically everything in the map
    pub buffer: wgpu::Buffer,
    queue: wgpu::Queue,
}

impl Drop for MvpBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

impl MvpBuffer {
    pub fn bind_group_layout_descriptor() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("model view projection bind group layout"),
            entries: &[
                // mvp buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        }
    }

    pub fn create_mvp(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut transformations: Vec<cgmath::Matrix4<f32>>,
    ) -> Self {
        // uniform buffer has fixed and defined size
        if transformations.len() > MAX_MVP {
            println!("There are more transformations than MAX_MVP");
        }

        transformations.resize(MAX_MVP, cgmath::Matrix4::zero());

        let transformations_casted: Vec<[[f32; 4]; 4]> =
            transformations.into_iter().map(|x| x.into()).collect();

        let mvp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model view projection buffer"),
            contents: bytemuck::cast_slice(&transformations_casted),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&MvpBuffer::bind_group_layout_descriptor());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model view projection array bind group"),
            layout: &bind_group_layout,
            entries: &[
                // mvp buffer
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: mvp_buffer.as_entire_binding(),
                },
            ],
        });

        MvpBuffer {
            bind_group,
            buffer: mvp_buffer,
            queue: queue.clone(),
        }
    }

    /// mvp_index is the index of the mvp, no need to calculate offset from the caller
    pub fn update_mvp_buffer(&self, mvp: cgmath::Matrix4<f32>, mvp_index: usize) {
        let mvp_cast: [[f32; 4]; 4] = mvp.into();
        let offset = mvp_index as u64 * 64;

        self.queue
            .write_buffer(&self.buffer, offset, bytemuck::cast_slice(&mvp_cast));
    }

    pub fn update_mvp_buffer_many(&self, mvps: Vec<cgmath::Matrix4<f32>>, mvp_index_start: usize) {
        let mvps_cast: Vec<[[f32; 4]; 4]> = mvps.into_iter().map(|x| x.into()).collect();
        let offset = mvp_index_start as u64 * 64;

        self.queue
            .write_buffer(&self.buffer, offset, bytemuck::cast_slice(&mvps_cast));
    }
}
