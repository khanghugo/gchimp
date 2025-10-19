use std::{array::from_fn, collections::HashSet};

use cgmath::{One, Rotation, Rotation3, VectorSpace, Zero};
use mdl::{BlendBone, Bone, Mdl};

/// `[[[[(position, rotation); bone count]; frame count]; blend count]; sequence count]`
// first vec is sequences
pub type MdlPosRot = Vec<
    // blends
    Vec<
        // frames
        // the order is swapped when we compare it to the studiomdl implementatino
        // this seems like a better order when we make the data
        // just remember that we go frame before bone
        Vec<
            // bones
            Vec<PosRot>,
        >,
    >,
>;

pub type PosRot = (
    // position
    cgmath::Vector3<f32>,
    // rotation
    cgmath::Quaternion<f32>,
);

#[derive(Debug, Clone)]
pub struct WorldTransformationSkeletal {
    pub current_sequence_index: usize,
    // storing base world transformation
    pub world_transformation: PosRot,
    // storing model transformation on top of that
    pub model_transformations: MdlPosRot,
    // data related to each model transformation
    pub model_transformation_infos: Vec<ModelTransformationInfo>,
}

impl WorldTransformationSkeletal {
    pub fn build_playermodel_mvp(
        &self,
        time: f32,
        gaitsequence: usize,
        blending: [u8; 2],
    ) -> Vec<cgmath::Matrix4<f32>> {
        let sequence = get_sequence_transformations(
            &self.model_transformations,
            &self.model_transformation_infos,
            self.world_transformation,
            self.current_sequence_index,
            blending,
            time,
        );

        if gaitsequence == 0 {
            return sequence;
        }

        let gait = get_sequence_transformations(
            &self.model_transformations,
            &self.model_transformation_infos,
            self.world_transformation,
            gaitsequence,
            blending,
            time,
        );

        let gait_bones: HashSet<usize> = (40..54).into_iter().chain([1]).collect();

        let bone_count = sequence.len();

        (0..bone_count)
            .map(|bone_idx| {
                if gait_bones.contains(&bone_idx) {
                    gait[bone_idx]
                } else {
                    sequence[bone_idx]
                }
            })
            .collect()
    }

    pub fn build_mvp(&self, time: f32) -> Vec<cgmath::Matrix4<f32>> {
        self.build_playermodel_mvp(time, 0, [0u8; 2])
    }
}

pub type WorldTransformationEntity = PosRot;

pub enum BuildMvpResult {
    Entity(cgmath::Matrix4<f32>),
    Skeletal(Vec<cgmath::Matrix4<f32>>),
}

pub fn build_mvp_from_pos_and_rot(
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
) -> cgmath::Matrix4<f32> {
    let rotation: cgmath::Matrix4<f32> = rotation.into();

    cgmath::Matrix4::from_translation(position.into()) * rotation
}

pub enum WorldTransformation {
    /// For entity brushes, they only have one transformation, so that is good.
    Entity(PosRot),
    /// For skeletal system, multiple transformations means there are multiple bones.
    ///
    /// So, we store all bones transformation and then put it back in shader when possible.
    ///
    /// And we also store all information related to the model. Basically a lite mdl format
    Skeletal(WorldTransformationSkeletal),
}

impl WorldTransformation {
    pub fn worldspawn() -> Self {
        Self::Entity(origin_posrot())
    }

    pub fn get_entity(&self) -> &WorldTransformationEntity {
        match self {
            WorldTransformation::Entity(x) => x,
            WorldTransformation::Skeletal(_) => unreachable!(),
        }
    }

    pub fn get_skeletal_mut(&mut self) -> &mut WorldTransformationSkeletal {
        match self {
            WorldTransformation::Entity(_) => unreachable!(),
            WorldTransformation::Skeletal(x) => x,
        }
    }

    pub fn build_mvp(&self, time: f32) -> BuildMvpResult {
        match &self {
            Self::Entity((position, rotation)) => {
                BuildMvpResult::Entity(build_mvp_from_pos_and_rot(*position, *rotation))
            }
            Self::Skeletal(x) => BuildMvpResult::Skeletal(x.build_mvp(time)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelTransformationInfo {
    pub frame_per_second: f32,
    pub looping: bool,
}

pub fn setup_studio_model_transformations(mdl: &Mdl) -> MdlPosRot {
    let bone_order = get_traversal_order(mdl);

    mdl.sequences
        .iter()
        .map(|sequence| {
            sequence
                .anim_blends
                .iter()
                .map(|blend| {
                    let frame_count = sequence.header.num_frames as usize;

                    // iterate over frame and then iterate over bone
                    // moving to a different frame doesn't change the numbers
                    // but the numbers within a frame are used together
                    // eg, bone hierarchy
                    (0..frame_count)
                        .map(|frame_idx| {
                            // caching the result
                            // based on the bone count
                            let mut transforms = vec![
                                (
                                    cgmath::Vector3::<f32>::zero(),
                                    cgmath::Quaternion::<f32>::one()
                                );
                                bone_order.len()
                            ];

                            bone_order.iter().for_each(|&bone_idx| {
                                let blend_bone = &blend[bone_idx];
                                let bone = &mdl.bones[bone_idx];

                                let (local_pos, local_rot) =
                                    compute_local_transformation(bone, blend_bone, frame_idx);

                                let (parent_pos, parent_rot) = if bone.parent == -1 {
                                    origin_posrot()
                                } else {
                                    transforms[bone.parent as usize]
                                };

                                // compute hierarchy transformation
                                let rotated_local_pos =
                                    parent_rot.rotate_vector(cgmath::Vector3::from(local_pos));

                                let accum_pos = parent_pos + rotated_local_pos;
                                let accum_rot = parent_rot * local_rot;

                                transforms[bone_idx] = (accum_pos, accum_rot);
                            });

                            transforms
                        })
                        .collect::<Vec<Vec<PosRot>>>()
                })
                .collect()
        })
        .collect()
}

// visiting parents and then its children so that we can nicely cache parent's result to avoid duplicated calculations
fn get_traversal_order(mdl: &Mdl) -> Vec<usize> {
    let mut order = Vec::with_capacity(mdl.bones.len());
    let mut visited = vec![false; mdl.bones.len()];

    // need to be a function to be recursive
    fn visit(bone_idx: usize, mdl: &Mdl, order: &mut Vec<usize>, visited: &mut Vec<bool>) {
        if visited[bone_idx] {
            return;
        }

        let parent = mdl.bones[bone_idx].parent;

        // if has parent and parent is not visited
        if parent != -1 && !visited[parent as usize] {
            // then visit parent
            visit(parent as usize, mdl, order, visited);
        }

        // add current bone to the order and then mark bone visited
        order.push(bone_idx as usize);
        visited[bone_idx] = true;
    }

    for bone_idx in 0..mdl.bones.len() {
        visit(bone_idx, mdl, &mut order, &mut visited);
    }

    order
}

fn compute_local_transformation(bone: &Bone, blend_bone: &BlendBone, frame_idx: usize) -> PosRot {
    let pos: [f32; 3] = from_fn(|i| {
        blend_bone[i] // motion type
                    [frame_idx] // frame animation
            as f32 // casting
                * bone.scale[i] // scale factor
                + bone.value[i] // bone default value
    });

    let angles: [f32; 3] =
        from_fn(|i| blend_bone[i + 3][frame_idx] as f32 * bone.scale[i + 3] + bone.value[i + 3]);

    let rot = cgmath::Quaternion::from_angle_z(cgmath::Rad(angles[2]))
        * cgmath::Quaternion::from_angle_y(cgmath::Rad(angles[1]))
        * cgmath::Quaternion::from_angle_x(cgmath::Rad(angles[0]));

    (pos.into(), rot)
}

pub fn model_to_world_transformation(
    (model_pos, model_rot): PosRot,
    world_pos: cgmath::Vector3<f32>,
    world_rot: cgmath::Quaternion<f32>,
) -> PosRot {
    // welp, if the world rot is 0, which is intentional, then no model rendered
    if world_rot == cgmath::Quaternion::zero() {
        return (cgmath::Vector3::zero(), cgmath::Quaternion::zero());
    }

    let new_rot = world_rot * model_rot;

    let entity_world_rotated_origin = world_rot.rotate_vector(model_pos);

    let new_pos = world_pos + entity_world_rotated_origin;

    (new_pos, new_rot)
}

pub fn origin_posrot() -> PosRot {
    (cgmath::Vector3::zero(), cgmath::Quaternion::one())
}

// https://github.com/SamVanheer/HalfLifeAssetManager/blob/4df74a58a50438b8f4b974e04ab9f24fdfcbb811/src/hlam/formats/studiomodel/BoneTransformer.cpp#L15
fn get_sequence_transformations(
    model_transformations: &MdlPosRot,
    model_transformation_infos: &Vec<ModelTransformationInfo>,
    world_transformation: PosRot,
    sequence_idx: usize,
    blending_factor: [u8; 2],
    time: f32,
) -> Vec<cgmath::Matrix4<f32>> {
    if model_transformations.len() == 0 {
        return vec![];
    }

    let blends = &model_transformations[sequence_idx];
    let current_info = &model_transformation_infos[sequence_idx];

    let blend_count = blends.len();

    let ((from_frame_idx, to_frame_idx), lerp_target) = get_lerp_target(
        time,
        blends[0].len(), // all blends have the same frame count
        current_info.looping,
        current_info.frame_per_second,
    );

    let lerp_frame = |i: usize| {
        get_lerp_frame(
            &blends[i][from_frame_idx],
            &blends[i][to_frame_idx],
            lerp_target,
        )
    };

    let lerp_frame_world_mvp = |i: usize| {
        get_lerp_frame_world_mvp(
            &blends[i][from_frame_idx],
            &blends[i][to_frame_idx],
            world_transformation,
            lerp_target,
        )
    };

    let [blend_x, blend_y] = blending_factor;

    if blend_count == 9 {
        let mut blend_res = vec![];
        let mut target_x = 0.;
        let mut target_y = 0.;

        let mut shorter_code = |a, b, c, d| {
            // closure can't recursive so this functin can jus have the same name and it works just fine
            let blend0 = lerp_frame(a);
            let blend1 = lerp_frame(b);
            let blend2 = lerp_frame(c);
            let blend3 = lerp_frame(d);

            blend_res = vec![blend0, blend1, blend2, blend3];
        };

        if blend_x > 127 {
            target_x = (blend_x - 127) as f32 * 2.;

            if blend_y > 127 {
                target_y = (blend_y as f32 - 127.) * 2.;
                shorter_code(4, 5, 7, 8);
            } else {
                target_y = blend_y as f32 * 2.;
                shorter_code(1, 2, 4, 5);
            }
        } else {
            target_x = blend_x as f32 * 2.;

            if blend_y <= 127 {
                target_y = blend_y as f32 * 2.;
                shorter_code(0, 1, 3, 4);
            } else {
                target_y = (blend_y - 127) as f32 * 2.;
                shorter_code(3, 4, 6, 7);
            }
        }

        target_x /= 255.;
        target_y /= 255.;

        let lerped1 = get_lerp_frame(&blend_res[0], &blend_res[1], target_x);
        let lerped2 = get_lerp_frame(&blend_res[2], &blend_res[3], target_x);

        return get_lerp_frame_world_mvp(&lerped1, &lerped2, world_transformation, target_y);
    } else if blend_count > 0 {
        // use blend 0 and that's it
        // TODO, all the blends
        return lerp_frame_world_mvp(0);
    } else {
        unreachable!("model does not have any blends");
    }
}

fn get_lerp_target(
    time: f32,
    frame_count: usize,
    looping: bool,
    fps: f32,
) -> ((usize, usize), f32) {
    let total_time = frame_count as f32 / fps;
    let actual_time = if looping { time % total_time } else { time };

    let from_index = ((actual_time * fps).floor() as usize).min(frame_count - 1);
    let to_index = (from_index + 1).min(frame_count - 1);
    let lerp_target = (actual_time * fps).fract();

    ((from_index, to_index), lerp_target)
}

fn get_lerp_frame(
    from_frame: &Vec<PosRot>,
    to_frame: &Vec<PosRot>,
    lerp_target: f32,
) -> Vec<PosRot> {
    from_frame
        .iter()
        .zip(to_frame)
        .map(|((from_pos, from_rot), (to_pos, to_rot))| {
            (
                from_pos.lerp(*to_pos, lerp_target),
                from_rot.nlerp(*to_rot, lerp_target),
            )
        })
        .collect()
}

fn get_lerp_frame_world_mvp(
    from_frame: &Vec<PosRot>,
    to_frame: &Vec<PosRot>,
    (world_pos, world_rot): PosRot,
    lerp_target: f32,
) -> Vec<cgmath::Matrix4<f32>> {
    from_frame
        .iter()
        .zip(to_frame)
        .map(|((from_pos, from_rot), (to_pos, to_rot))| {
            let lerped_posrot = (
                from_pos.lerp(*to_pos, lerp_target),
                from_rot.nlerp(*to_rot, lerp_target),
            );

            let (pos, rot) = model_to_world_transformation(lerped_posrot, world_pos, world_rot);
            build_mvp_from_pos_and_rot(pos, rot)
        })
        .collect()
}
