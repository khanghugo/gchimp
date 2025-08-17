use std::{array::from_fn, ffi::OsStr, fs::OpenOptions, io::Read, path::Path};

use nom::{
    Parser,
    bytes::complete::take,
    combinator::map,
    multi::count,
    number::complete::{le_f32, le_i16, le_i32, le_u8, le_u16},
};

use crate::{
    Attachment, Blend, Bodypart, BodypartHeader, Bone, BoneController, Hitbox, Mesh, MeshHeader,
    MeshTriangles, Model, ModelHeader, PALETTE_COUNT, Sequence, SequenceFlag, SequenceGroup,
    SkinFamilies, Trivert, TrivertHeader, VEC3_T_SIZE,
    error::MdlError,
    nom_helpers::{IResult, vec3},
    types::{Header, Mdl, SequenceHeader, Texture, TextureFlag, TextureHeader},
};

impl Mdl {
    pub fn open_from_bytes(bytes: &[u8]) -> Result<Mdl, MdlError> {
        parse_mdl(bytes)
    }

    pub fn open_from_file(path: impl AsRef<OsStr> + AsRef<Path>) -> Result<Mdl, MdlError> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|op| MdlError::IOError { source: op })?;
        let mut bytes = vec![];

        file.read_to_end(&mut bytes)
            .map_err(|op| MdlError::IOError { source: op })?;

        Self::open_from_bytes(&bytes)
    }
}

fn parse_mdl(i: &[u8]) -> Result<Mdl, MdlError> {
    let start = i;
    let (_, mdl_header) = parse_header(start).map_err(|_| MdlError::ParseHeader)?;

    let (_, textures) = parse_textures(start, &mdl_header).map_err(|_| MdlError::ParseTextures)?;

    let (_, bodyparts) =
        parse_bodyparts(start, &mdl_header).map_err(|_| MdlError::ParseBodyparts)?;

    let (_, bones) = parse_bones(start, &mdl_header).map_err(|_| MdlError::ParseBones)?;

    let (_, bone_controllers) =
        parse_bone_controllers(start, &mdl_header).map_err(|_| MdlError::ParseBoneControllers)?;

    let (_, hitboxes) = parse_hitboxes(start, &mdl_header).map_err(|_| MdlError::ParseHitboxes)?;

    let (_, sequence_groups) =
        parse_sequence_groups(start, &mdl_header).map_err(|_| MdlError::ParseSequenceGroups)?;

    let (_, skin_families) =
        parse_skin_families(start, &mdl_header).map_err(|_| MdlError::ParseSkinFamilies)?;

    let (_, attachments) =
        parse_attachments(start, &mdl_header).map_err(|_| MdlError::ParseAttachments)?;

    let (_, sequences) =
        parse_sequences(start, &mdl_header).map_err(|_| MdlError::ParseSequences)?;

    Ok(Mdl {
        header: mdl_header,
        sequences,
        textures,
        bodyparts,
        bones,
        bone_controllers,
        hitboxes,
        sequence_groups,
        skin_families,
        attachments,
    })
}

fn parse_header(i: &[u8]) -> IResult<Header> {
    map(
        (
            (
                le_i32,
                le_i32,
                count(le_u8, 64),
                le_i32,
                vec3,
                vec3,
                vec3,
                vec3,
                vec3,
                le_i32,
            ),
            (
                le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32,
                le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32, le_i32,
            ),
            (le_i32, le_i32, le_i32, le_i32, le_i32, le_i32),
        ),
        |(
            (id, version, name, length, eye_position, min, max, bbmin, bbmax, flags),
            (
                num_bones,
                bone_index,
                num_bone_controllers,
                bone_controller_index,
                num_hitboxes,
                hitbox_index,
                num_seq,
                seq_index,
                num_seq_group,
                seq_group_index,
                num_textures,
                texture_index,
                texture_data_index,
                num_skin_ref,
                num_skin_families,
                skin_index,
                num_body_parts,
                body_part_index,
                num_attachments,
                attachment_index,
            ),
            (
                sound_table,
                sound_index,
                sound_groups,
                sound_group_index,
                num_transitions,
                transition_index,
            ),
        )| Header {
            id,
            version,
            name: from_fn(|i| name[i]),
            length,
            eye_position,
            min,
            max,
            bbmin,
            bbmax,
            flags,
            num_bones,
            bone_index,
            num_bone_controllers,
            bone_controller_index,
            num_hitboxes,
            hitbox_index,
            num_seq,
            seq_index,
            num_seq_group,
            seq_group_index,
            num_textures,
            texture_index,
            texture_data_index,
            num_skin_ref,
            num_skin_families,
            skin_index,
            num_bodyparts: num_body_parts,
            bodypart_index: body_part_index,
            num_attachments,
            attachment_index,
            sound_table,
            sound_index,
            sound_groups,
            sound_group_index,
            num_transitions,
            transition_index,
        },
    )
    .parse(i)
}

// https://github.com/LogicAndTrick/sledge-formats/blob/7a3bfb33562aece483e15796b8573b23d71319ab/Sledge.Formats.Model/Goldsource/MdlFile.cs#L442
fn _parse_animation_frame_rle_sledge(br: &[u8], read_count: usize) -> IResult<Vec<i16>> {
    let mut values: Vec<i16> = vec![0; read_count];

    let mut i = 0;

    let mut reader = br;

    while i < read_count {
        let (br, run) = take(2usize)(reader)?;
        let (br, vals) = count(le_i16, run[0] as usize).parse(br)?;

        reader = br;

        let mut j = 0;

        while j < run[1] && i < read_count {
            if !vals.is_empty() {
                let idx: u8 = (run[0] - 1).min(j);
                values[i] = vals[idx as usize];
            }

            i += 1;
            j += 1;
        }
    }

    Ok((reader, values))
}

// gemini2.5 solution. just send it the entire mdl documentation and send it the compression algorithm
fn _parse_animation_frame_rle_gemini(
    mut reader: &[u8], // Track input slice consumption
    num_frames: usize,
) -> IResult<Vec<i16>> {
    let mut output_values: Vec<i16> = vec![0; num_frames]; // Pre-allocate result
    let mut current_frame_index = 0; // Track how many frames we've filled

    // Loop until all expected frames are filled
    while current_frame_index < num_frames {
        // Read the RLE header (valid_count, total_frames_in_run)
        let (next_reader, header_bytes) = take(2usize)(reader)?;
        let valid_count = header_bytes[0] as usize;
        let total_frames_in_run = header_bytes[1] as usize;

        // Basic sanity check for potentially corrupt data
        // A run should ideally have at least one frame and one valid value.
        if total_frames_in_run == 0 {
            // eprintln!("Warning: Encountered RLE run with total_frames_in_run = 0. Stopping parse for this channel.");
            break; // Stop processing this channel if run is empty
        }
        if valid_count == 0 {
            // eprintln!("Warning: Encountered RLE run with valid_count = 0. Treating as 0s for {} frames.", total_frames_in_run);
            // If valid is 0, we technically don't have values. Fill with 0?
            for _ in 0..total_frames_in_run {
                if current_frame_index >= num_frames {
                    break;
                }
                output_values[current_frame_index] = 0;
                current_frame_index += 1;
            }
            reader = next_reader; // Consume header, but no values read.
            continue; // Move to next run
        }

        // Read the 'valid_count' actual i16 values for this run
        let (next_reader, run_values) = count(le_i16, valid_count).parse(next_reader)?;

        // Apply the run to the output_values buffer
        for j in 0..total_frames_in_run {
            // Check if we've already filled all required frames
            if current_frame_index >= num_frames {
                // eprintln!("Warning: RLE data seems longer than expected num_frames ({}). Stopping early.", num_frames);
                // Update reader state to where we stopped reading *values*
                reader = next_reader;
                // Return Ok, but the caller should be aware data might be truncated/extra
                return Ok((reader, output_values));
            }

            // Determine the index into run_values: repeat the last one if j >= valid_count
            let value_index = j.min(valid_count - 1); // Correct indexing
            output_values[current_frame_index] = run_values[value_index];

            current_frame_index += 1; // Move to the next frame slot
        }

        // Update the reader to point after the values read for this run
        reader = next_reader;
    }

    // Check if we exactly filled the buffer (optional sanity check)
    // if current_frame_index != num_frames {
    //     eprintln!(
    //         "Warning: RLE parsing filled {} frames, but expected {}.",
    //         current_frame_index, num_frames
    //     );
    // }

    // Return the remaining unparsed slice and the decoded values
    Ok((reader, output_values))
}

// Based on the comments from this
// https://github.com/LogicAndTrick/sledge-formats/blob/7a3bfb33562aece483e15796b8573b23d71319ab/Sledge.Formats.Model/Goldsource/MdlFile.cs#L442
// And then make it more idiomatic rust
pub fn parse_animation_frame_rle(rle_start: &[u8], num_frame: usize) -> IResult<Vec<i16>> {
    let mut res = vec![0i16; num_frame];
    let mut next = rle_start;

    let mut curr_frame = 0;

    while curr_frame < num_frame {
        let (mut _next, run) = take(2usize)(next)?;

        let mut valid = run[0];
        let mut total = run[1];

        // // invalid case
        // let mut invalid_advance = curr_frame;

        // while invalid_advance >= total as usize {
        //     invalid_advance -= total as usize;
        //     let (__next, _) = count(le_i16, valid as usize)(_next)?;

        //     let (mut __next, run) = take(2usize)(__next)?;

        //     valid = run[0];
        //     total = run[1];

        //     _next = __next;
        // }

        // compressed values
        let (_next, compressed_values) = count(le_i16, valid as usize).parse(_next)?;

        // advancing reader
        next = _next;

        let last_value = compressed_values.last();

        // uncompressed values
        // repeating the last element
        match last_value {
            Some(x) => compressed_values.iter().chain(std::iter::repeat(x)),
            None => {
                // no values, it is all 0
                break;
            }
        }
        .take(total as usize)
        .for_each(|&value| {
            // this loop isn't aware of the exit
            if curr_frame >= num_frame {
                return;
            }

            res[curr_frame] = value;

            // advancing the outside loop
            curr_frame += 1;
        });
    }

    Ok((next, res))
}

// parse starting from animation offset based off what I see in the bone setup function
fn _parse_blend_studiomdl_impl<'a>(
    panim: &'a [u8],
    mdl_header: &Header,
    sequence_header: &SequenceHeader,
) -> IResult<'a, Blend> {
    let offset_parser = map(count(le_u16, 6 as usize), |res| {
        [res[0], res[1], res[2], res[3], res[4], res[5]]
    });

    let (end_of_blend, blends_offets) =
        count(offset_parser, mdl_header.num_bones as usize).parse(panim)?;

    let mut res: Blend = vec![];
    let num_frames = sequence_header.num_frames as usize;

    for blend_offsets in blends_offets.into_iter() {
        let mut bone_values: [Vec<i16>; 6] = from_fn(|_| vec![0; num_frames]);

        for (motion_idx, offset) in blend_offsets.into_iter().enumerate() {
            // if no offset, the whole run of bone blend value is 0
            if offset == 0 {
                // TODO: bone controller
                continue;
            }

            //  typedef union
            // {
            // 	struct {
            // 		unsigned char	valid;
            // 		unsigned char	total;
            // 	} num;
            // 	short		value;
            // } mstudioanimvalue_t;

            for frame_index in 0..num_frames {
                let mut panimvalue = &panim[offset as usize..];

                // "find span of values that includes the frame we want"
                let mut k = frame_index as u8;

                while panimvalue[1] <= k {
                    let num_valid = panimvalue[0];

                    k -= panimvalue[1];
                    panimvalue = &panimvalue[(num_valid as usize + 1) * 2..];
                }

                let mut anim_value = 0;

                // "if we're inside the span"
                if panimvalue[0] > k {
                    // "and there's more data in the span"
                    let idx = (k as usize + 1) * 2;

                    anim_value = i16::from_le_bytes([panimvalue[idx], panimvalue[idx + 1]]);
                } else {
                    // "are we at the end of the repeating values section and there's another section with data?"
                    let idx = panimvalue[0] as usize * 2;

                    anim_value = i16::from_le_bytes([panimvalue[idx], panimvalue[idx + 1]]);
                }

                bone_values[motion_idx][frame_index] = anim_value;
            }
        }

        res.push(bone_values);
    }

    Ok((end_of_blend, res))
}

// parse starting from animation offset
fn parse_blend<'a>(
    // panimvalue points to the current blend
    // the layout goes
    // - blend 1 offsets
    // - - bone 0 offsets
    // - - bone 1 offsets
    // - - ...
    // - blend 2 offsets
    // - - bone 0 offsets
    // - - ...
    //
    // bone N offsets starts from panimvalue
    // starting from the offset is a RLE
    // this RLE contains all animation values for that one motion type
    //
    // so, the result for 1 blend is: X amount of bone for 6 arrays of Y animation value for that motion type
    // the type is [[[short animation value; animation count]; 6 motion types]; X bone]
    panim: &'a [u8],
    mdl_header: &Header,
    sequence_header: &SequenceHeader,
) -> IResult<'a, Blend> {
    let offset_parser = map(count(le_u16, 6 as usize), |res| {
        [res[0], res[1], res[2], res[3], res[4], res[5]]
    });

    let (end_of_blend, blends_offsets) =
        count(offset_parser, mdl_header.num_bones as usize).parse(panim)?;

    // the animation frame is offset from the beginning of the panim "struct", which is anim_offset + current blend number
    // https://github.com/ValveSoftware/halflife/blob/c7240b965743a53a29491dd49320c88eecf6257b/utils/mdlviewer/studio_render.cpp#L190
    let mut res: Blend = vec![];
    let num_frames = sequence_header.num_frames as usize;

    // at the moment, we have the bone count and the offsets
    // now we have to fit animations inside the bone count
    for (bone_idx, blend_offsets) in blends_offsets.into_iter().enumerate() {
        let mut bone_values: [Vec<i16>; 6] = from_fn(|_| vec![0; num_frames]);

        for (motion_idx, offset) in blend_offsets.into_iter().enumerate() {
            if offset == 0 {
                continue;
            }

            let panimvalue = &panim[
                // not sure why, but i have to offset this by this
                // thanks to newbspguy for easy compilation so that i can debug this
                (offset as usize + bone_idx * 12)..];

            let (_, values) = parse_animation_frame_rle(panimvalue, num_frames)?;

            bone_values[motion_idx] = values;
        }

        res.push(bone_values);
    }

    Ok((end_of_blend, res))
}

fn parse_sequence<'a>(start: &'a [u8], i: &'a [u8], mdl_header: &Header) -> IResult<'a, Sequence> {
    let (sequence_header_end, header) = parse_sequence_description(i).unwrap();

    let animation_frame_parser = |i| parse_blend(i, mdl_header, &header);

    let (_, anim_blends) = count(animation_frame_parser, header.num_blends as usize)
        .parse(&start[header.anim_index as usize..])?;

    Ok((
        sequence_header_end,
        Sequence {
            header,
            anim_blends,
        },
    ))
}

fn parse_sequences<'a>(start: &'a [u8], mdl_header: &Header) -> IResult<'a, Vec<Sequence>> {
    let parser = |i| parse_sequence(start, i, mdl_header);
    count(parser, mdl_header.num_seq as usize).parse(&start[mdl_header.seq_index as usize..])
}

fn parse_sequence_description(i: &[u8]) -> IResult<SequenceHeader> {
    map(
        (
            (
                count(le_u8, 32),
                le_f32,
                le_i32,
                le_i32,
                le_i32,
                le_i32,
                le_i32,
                le_i32,
                le_i32,
                le_i32,
                le_i32,
                le_i32,
                vec3,
                le_i32,
                le_i32,
            ),
            (
                vec3,
                vec3,
                le_i32,
                le_i32,
                count(le_i32, 2),
                count(le_f32, 2),
                count(le_f32, 2),
                le_i32,
            ),
            (le_i32, le_i32, le_i32, le_i32, le_i32),
        ),
        |(
            (
                label,
                fps,
                flags,
                activity,
                act_weight,
                num_events,
                event_index,
                num_frames,
                num_pivots,
                pivot_index,
                motion_type,
                motion_bone,
                linear_movement,
                auto_move_pos_index,
                auto_move_angle_index,
            ),
            (
                bbmin,
                bbmax,
                num_blends,
                anim_index,
                blend_type,
                blend_start,
                blend_end,
                blend_parent,
            ),
            (seq_group, entry_node, exit_node, node_flags, next_seq),
        )| SequenceHeader {
            label: from_fn(|i| label[i]),
            fps,
            flags: SequenceFlag::from_bits(flags).unwrap(),
            activity,
            act_weight,
            num_events,
            event_index,
            num_frames,
            num_pivots,
            pivot_index,
            motion_type,
            motion_bone,
            linear_movement,
            auto_move_pos_index,
            auto_move_angle_index,
            bbmin,
            bbmax,
            num_blends,
            anim_index,
            blend_type: from_fn(|i| blend_type[i]),
            blend_start: from_fn(|i| blend_start[i]),
            blend_end: from_fn(|i| blend_end[i]),
            blend_parent,
            seq_group,
            entry_node,
            exit_node,
            node_flags,
            next_seq,
        },
    )
    .parse(i)
}

fn parse_texture_header(i: &[u8]) -> IResult<TextureHeader> {
    map(
        (count(le_u8, 64), le_i32, le_i32, le_i32, le_i32),
        |(name, flags, width, height, index)| TextureHeader {
            name: from_fn(|i| name[i]),
            flags: TextureFlag::from_bits(flags).unwrap_or_else(|| {
                // println!("unknown texture flag {flags}");
                TextureFlag::empty()
            }),
            width,
            height,
            index,
        },
    )
    .parse(i)
}

fn parse_texture<'a>(i: &'a [u8], start: &'a [u8]) -> IResult<'a, Texture> {
    let (end_of_header, texture_header) = parse_texture_header(i)?;

    let (end_of_texture, texture_bytes): (_, &[u8]) =
        take((texture_header.width * texture_header.height) as usize)(
            &start[texture_header.index as usize..],
        )?;

    let (_, palette) = take(PALETTE_COUNT * 3)(end_of_texture)?;
    let palette: [[u8; 3]; PALETTE_COUNT] = from_fn(|i| {
        palette
            .chunks(3)
            .map(|i| [i[0], i[1], i[2]])
            .collect::<Vec<[u8; 3]>>()[i]
    });

    Ok((
        end_of_header,
        Texture {
            header: texture_header,
            image: texture_bytes.to_vec(),
            palette,
        },
    ))
}

fn parse_textures<'a>(start: &'a [u8], mdl_header: &Header) -> IResult<'a, Vec<Texture>> {
    let parser = |i| parse_texture(i, start);

    count(parser, mdl_header.num_textures as usize)
        .parse(&start[mdl_header.texture_index as usize..])
}

fn parse_trivert_header(i: &[u8]) -> IResult<TrivertHeader> {
    map(
        (le_i16, le_i16, le_i16, le_i16),
        |(vert_index, norm_index, s, t)| TrivertHeader {
            vert_index,
            norm_index,
            s,
            t,
        },
    )
    .parse(i)
}

fn parse_trivert<'a>(
    i: &'a [u8],
    start: &'a [u8],
    model_header: &ModelHeader,
) -> IResult<'a, Trivert> {
    let (end_of_header, trivert_header) = parse_trivert_header(i)?;

    let vert_offset = VEC3_T_SIZE * trivert_header.vert_index as usize;
    let norm_offset = VEC3_T_SIZE * trivert_header.norm_index as usize;

    let (_, vertex) = vec3(&start[(model_header.vert_index as usize + vert_offset)..])?;
    let (_, normal) = vec3(&start[(model_header.norm_index as usize + norm_offset)..])?;

    Ok((
        end_of_header,
        Trivert {
            header: trivert_header,
            vertex,
            normal,
        },
    ))
}

fn parse_mesh_triangles<'a>(
    start: &'a [u8],
    model_header: &ModelHeader,
    mesh_header: &MeshHeader,
) -> IResult<'a, Vec<MeshTriangles>> {
    let mut res: Vec<MeshTriangles> = vec![];

    let parser = |i| parse_trivert(i, start, model_header);

    let mut trivert_run_start = &start[mesh_header.tri_index as usize..];

    loop {
        let (i, trivert_count) = le_i16(trivert_run_start)?;
        let trivert_count_abs = trivert_count.abs();

        if trivert_count_abs == 0 {
            break;
        }

        let (i, triverts) = count(parser, trivert_count_abs as usize).parse(i)?;

        let triangles = if trivert_count.is_positive() {
            MeshTriangles::Strip(triverts)
        } else {
            MeshTriangles::Fan(triverts)
        };

        res.push(triangles);

        trivert_run_start = i;
    }

    Ok((trivert_run_start, res))
}

fn parse_mesh_header(i: &[u8]) -> IResult<MeshHeader> {
    map(
        (le_i32, le_i32, le_i32, le_i32, le_i32),
        |(num_tris, tri_index, skin_ref, num_norms, norm_index)| MeshHeader {
            num_tris,
            tri_index,
            skin_ref,
            num_norms,
            norm_index,
        },
    )
    .parse(i)
}

fn parse_mesh<'a>(i: &'a [u8], start: &'a [u8], model_header: &ModelHeader) -> IResult<'a, Mesh> {
    let (end_of_header, mesh_header) = parse_mesh_header(i)?;
    let (_end_of_triverts, triangles) = parse_mesh_triangles(start, model_header, &mesh_header)?;

    Ok((
        end_of_header,
        Mesh {
            header: mesh_header,
            triangles,
        },
    ))
}

fn parse_meshes<'a>(start: &'a [u8], model_header: &ModelHeader) -> IResult<'a, Vec<Mesh>> {
    let parser = |i| parse_mesh(i, start, model_header);

    count(parser, model_header.num_mesh as usize).parse(&start[model_header.mesh_index as usize..])
}

fn parse_model_header(i: &[u8]) -> IResult<ModelHeader> {
    map(
        (
            count(le_u8, 64),
            le_i32,
            le_f32,
            le_i32,
            le_i32,
            le_i32,
            le_i32,
            le_i32,
            le_i32,
            le_i32,
            le_i32,
            le_i32,
            le_i32,
        ),
        |(
            name,
            type_,
            bounding_radius,
            num_mesh,
            mesh_index,
            num_verts,
            vert_info_index,
            vert_index,
            num_norms,
            norm_info_index,
            norm_index,
            num_groups,
            group_index,
        )| ModelHeader {
            name: from_fn(|i| name[i]),
            type_,
            bounding_radius,
            num_mesh,
            mesh_index,
            num_verts,
            vert_info_index,
            vert_index,
            num_norms,
            norm_info_index,
            norm_index,
            num_groups,
            group_index,
        },
    )
    .parse(i)
}

fn parse_vertex_info<'a>(start: &'a [u8], model_header: &ModelHeader) -> IResult<'a, Vec<u8>> {
    count(le_u8, model_header.num_verts as usize)
        .parse(&start[model_header.vert_info_index as usize..])
}

fn parse_model<'a>(i: &'a [u8], start: &'a [u8]) -> IResult<'a, Model> {
    let (end_of_header, model_header) = parse_model_header(i)?;
    let (_end_of_meshes, meshes) = parse_meshes(start, &model_header)?;
    let (_end_of_vertex_info, vertex_info) = parse_vertex_info(start, &model_header)?;

    Ok((
        end_of_header,
        Model {
            header: model_header,
            meshes,
            vertex_info,
        },
    ))
}

fn parse_models<'a>(start: &'a [u8], bodypart_header: &BodypartHeader) -> IResult<'a, Vec<Model>> {
    let parser = |i| parse_model(i, start);

    count(parser, bodypart_header.num_models as usize)
        .parse(&start[bodypart_header.model_index as usize..])
}

fn parse_bodypart_header(i: &[u8]) -> IResult<BodypartHeader> {
    map(
        (count(le_u8, 64), le_i32, le_i32, le_i32),
        |(name, num_models, base, model_index)| BodypartHeader {
            name: from_fn(|i| name[i]),
            num_models,
            base,
            model_index,
        },
    )
    .parse(i)
}

fn parse_bodypart<'a>(i: &'a [u8], start: &'a [u8]) -> IResult<'a, Bodypart> {
    let (end_of_header, bodypart_header) = parse_bodypart_header(i)?;
    let (_end_of_models, models) = parse_models(start, &bodypart_header)?;

    Ok((
        end_of_header,
        Bodypart {
            header: bodypart_header,
            models,
        },
    ))
}

fn parse_bodyparts<'a>(start: &'a [u8], mdl_header: &Header) -> IResult<'a, Vec<Bodypart>> {
    let parser = |i| parse_bodypart(i, start);

    count(parser, mdl_header.num_bodyparts as usize)
        .parse(&start[mdl_header.bodypart_index as usize..])
}

fn parse_bone(i: &[u8]) -> IResult<Bone> {
    map(
        (
            count(le_u8, 32),
            le_i32,
            le_i32,
            count(le_i32, 6),
            count(le_f32, 6),
            count(le_f32, 6),
        ),
        |(name, parent, flags, bone_controller, value, scale)| Bone {
            name: from_fn(|i| name[i]),
            parent,
            flags,
            bone_controller: from_fn(|i| bone_controller[i]),
            value: from_fn(|i| value[i]),
            scale: from_fn(|i| scale[i]),
        },
    )
    .parse(i)
}

fn parse_bones<'a>(start: &'a [u8], mdl_header: &Header) -> IResult<'a, Vec<Bone>> {
    count(parse_bone, mdl_header.num_bones as usize).parse(&start[mdl_header.bone_index as usize..])
}

fn parse_bone_controller(i: &[u8]) -> IResult<BoneController> {
    map(
        (le_i32, le_i32, le_f32, le_f32, le_i32, le_i32),
        |(bone, type_, start, end, rest, index)| BoneController {
            bone,
            type_,
            start,
            end,
            rest,
            index,
        },
    )
    .parse(i)
}

fn parse_bone_controllers<'a>(
    start: &'a [u8],
    mdl_header: &Header,
) -> IResult<'a, Vec<BoneController>> {
    count(
        parse_bone_controller,
        mdl_header.num_bone_controllers as usize,
    )
    .parse(&start[mdl_header.bone_controller_index as usize..])
}

pub fn parse_hitbox(i: &[u8]) -> IResult<Hitbox> {
    map(
        (le_i32, le_i32, vec3, vec3),
        |(bone, group, bbmin, bbmax)| Hitbox {
            bone,
            group,
            bbmin,
            bbmax,
        },
    )
    .parse(i)
}

pub fn parse_hitboxes<'a>(start: &'a [u8], mdl_header: &Header) -> IResult<'a, Vec<Hitbox>> {
    count(parse_hitbox, mdl_header.num_hitboxes as usize)
        .parse(&start[mdl_header.hitbox_index as usize..])
}

pub fn parse_sequence_group(i: &[u8]) -> IResult<SequenceGroup> {
    map(
        (count(le_u8, 32), count(le_u8, 64), le_i32, le_i32),
        |(label, name, unused1, unused2)| SequenceGroup {
            label: from_fn(|i| label[i]),
            name: from_fn(|i| name[i]),
            unused1,
            unused2,
        },
    )
    .parse(i)
}

pub fn parse_sequence_groups<'a>(
    start: &'a [u8],
    mdl_header: &Header,
) -> IResult<'a, Vec<SequenceGroup>> {
    count(parse_sequence_group, mdl_header.num_seq as usize)
        .parse(&start[mdl_header.seq_group_index as usize..])
}

pub fn parse_skin_families<'a>(start: &'a [u8], mdl_header: &Header) -> IResult<'a, SkinFamilies> {
    count(
        count(le_i16, mdl_header.num_skin_ref as usize),
        mdl_header.num_skin_families as usize,
    )
    .parse(&start[mdl_header.skin_index as usize..])
}

pub fn parse_attachment(i: &[u8]) -> IResult<Attachment> {
    map(
        (count(le_u8, 32), le_i32, le_i32, vec3, count(vec3, 3)),
        |(name, type_, bone, org, vectors)| Attachment {
            name: from_fn(|i| name[i]),
            type_,
            bone,
            org,
            vectors: from_fn(|i| vectors[i]),
        },
    )
    .parse(i)
}

pub fn parse_attachments<'a>(start: &'a [u8], mdl_header: &Header) -> IResult<'a, Vec<Attachment>> {
    count(parse_attachment, mdl_header.num_attachments as usize)
        .parse(&start[mdl_header.attachment_index as usize..])
}
