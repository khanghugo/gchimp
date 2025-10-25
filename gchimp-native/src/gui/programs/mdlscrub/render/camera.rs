use bytemuck::{Pod, Zeroable};
use egui_wgpu::wgpu;

use cgmath::{perspective, Deg, InnerSpace, Matrix4, Point3, Rotation3, Vector3, Zero};

pub struct CameraBuffer {
    pub view: wgpu::Buffer,
    pub projection: wgpu::Buffer,
    pub position: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl Drop for CameraBuffer {
    fn drop(&mut self) {
        self.view.destroy();
        self.projection.destroy();
        self.position.destroy();
    }
}

pub const FOV_MIN: f32 = 1.;
pub const FOV_MAX: f32 = 179.;
pub const FOV_DEFAULT: f32 = 90.;

impl CameraBuffer {
    pub fn bind_group_layout_descriptor() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("camera bind group layout"),
            entries: &[
                // view
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
                // projection
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // position
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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

    pub fn create(device: &wgpu::Device) -> Self {
        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera view buffer"),
            size: 4 * 4 * 4, // 4x4 matrix
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false, // empty buffer
        });

        let proj_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera projection buffer"),
            size: 4 * 4 * 4, // 4x4 matrix
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false, // empty buffer
        });

        let position_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera position buffer"),
            size: 4 * 3, // [f32; 3]
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false, // empty buffer
        });

        let bind_group_layout =
            device.create_bind_group_layout(&Self::bind_group_layout_descriptor());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &bind_group_layout,
            entries: &[
                // view
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view_buffer.as_entire_binding(),
                },
                // projection
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: proj_buffer.as_entire_binding(),
                },
                // position
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: position_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            view: view_buffer,
            projection: proj_buffer,
            bind_group,
            position: position_buffer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScrubCamera {
    pub pos: Point3<f32>,
    // dont change target directly
    // it should be derived from quaternion
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    /// This is the result FOV for the renderer
    pub fovy: Deg<f32>,
    /// This is the input FOV from the game. Remember to calculate `fovy` and store it.
    pub fovx: Deg<f32>,
    pub znear: f32,
    pub zfar: f32,
    // use getters
    yaw: Deg<f32>,
    pitch: Deg<f32>,
    // rotation state and everything in the camera depends on this
    pub orientation: cgmath::Quaternion<f32>,
}

// const CAM_START_POS: [f32; 3] = [-300., -1000., -2000.];
// const CAM_START_POS: [f32; 3] = [1000., 300., 500.];
const CAM_START_POS: [f32; 3] = [0f32; 3];

impl Default for ScrubCamera {
    fn default() -> Self {
        Self::new()
    }
}

const MAX_PITCH: f32 = 89.0;

impl ScrubCamera {
    pub fn new() -> Self {
        let up = Vector3::unit_z(); // using the game up vector
        let start_pos = Point3::<f32>::from(CAM_START_POS);
        let target_pos = start_pos + Vector3::unit_x();

        // zero orientation but then build it after initializing so that the first frame is correct
        let orientation = cgmath::Quaternion::zero();

        let mut res = Self {
            pos: start_pos,
            target: target_pos,
            up,
            aspect: 640. / 480.,
            fovy: Deg(FOV_DEFAULT),
            fovx: Deg(FOV_DEFAULT),
            znear: 1.0,
            zfar: 131072.0,
            orientation,
            yaw: Deg(0.),
            pitch: Deg(0.),
        };

        res.rebuild_orientation();

        res
    }

    pub fn view(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.pos, self.target, self.up)
    }

    pub fn proj(&self) -> Matrix4<f32> {
        perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }

    pub fn proj_view(&self) -> Matrix4<f32> {
        self.proj() * self.view()
    }

    pub fn rotate_in_place_yaw(&mut self, angle: Deg<f32>) {
        self.yaw += angle;
        self.rebuild_orientation();
    }

    pub fn rotate_in_place_pitch(&mut self, angle: Deg<f32>) {
        self.pitch += angle;
        self.rebuild_orientation();
    }

    pub fn rebuild_orientation(&mut self) {
        self.pitch = Deg(self.pitch.0.clamp(-MAX_PITCH, MAX_PITCH));

        let yaw_quat = cgmath::Quaternion::from_axis_angle(self.up.normalize(), self.yaw);

        let forward = yaw_quat * Vector3::unit_x();
        let right = forward.cross(self.up).normalize();

        let pitch_quat = cgmath::Quaternion::from_axis_angle(right, self.pitch);

        // update orientation
        self.orientation = pitch_quat * yaw_quat;

        // update target
        // need to use this forward to get the correct forward vector to offset pos for target
        let final_forward = self.orientation * Vector3::unit_x();

        self.target = self.pos + final_forward * 1.;
    }

    pub fn move_along_view(&mut self, distance: f32) {
        let v = self.target - self.pos;
        let offset = v.normalize() * distance;

        self.target += offset;
        self.pos += offset;
    }

    pub fn move_along_view_orthogonal(&mut self, distance: f32) {
        let v = self.target - self.pos;
        let up = self.up;
        let orthogonal = v.cross(up);

        let offset = orthogonal.normalize() * distance;

        self.target += offset;
        self.pos += offset;
    }

    pub fn yaw(&self) -> Deg<f32> {
        self.yaw
    }

    pub fn pitch(&self) -> Deg<f32> {
        self.pitch
    }

    /// Needs to manually build orientation afterward
    pub fn set_yaw(&mut self, yaw: Deg<f32>) {
        self.yaw = yaw;
    }

    /// Needs to manually build orientation afterward
    pub fn set_pitch(&mut self, pitch: Deg<f32>) {
        self.pitch = pitch;
    }

    pub fn set_position(&mut self, pos: [f32; 3]) {
        self.pos = pos.into();
    }

    // HMMMMMMMMM
    // This is from HL25. It is the same as legacy.
    // float CalcFov(float fov_x,float width,float height)

    // {
    //   float fVar1;
    //   double dVar2;

    //   if ((fov_x < 1.0) || (179.0 < fov_x)) {
    //     fVar1 = 1.0;
    //   }
    //   else {
    //     dVar2 = tan((double)((fov_x / 360.0) * 3.141593));
    //     fVar1 = (float)dVar2;
    //   }
    //   dVar2 = atan((double)(height / (width / fVar1)));
    //   return ((float)dVar2 * 360.0) / 3.141593;
    // }
    pub fn calculate_y_fov(x_fov: Deg<f32>, width: f32, height: f32) -> Deg<f32> {
        let x_fov = x_fov.0;
        let x_fov = x_fov.clamp(FOV_MIN, FOV_MAX);

        // width and height are hardcoded to 4/3 for reasons
        Deg(((x_fov.to_radians() / 2.) * (4. / 3.)).atan().to_degrees() * 2.)
        // Deg(((x_fov.to_radians() / 2.) * (width / height))
        //     .atan()
        //     .to_degrees()
        //     * 2.)
    }
}

// camera changes a lot between tiles so just make it push constants instead
#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct CameraPushConstant {
    pub camera_matrix: [[f32; 4]; 4],
}
