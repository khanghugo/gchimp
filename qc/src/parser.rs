use glam::DVec3;
use nom::branch::alt;
use nom::bytes::complete::take;
use nom::character::complete::{multispace0, space0};
use nom::combinator::{all_consuming, fail, map, opt, peek, rest};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::{delimited, terminated, tuple};
use nom::{bytes::complete::tag, sequence::preceded};

use crate::types::{
    BBox, Body, BodyGroup, CBox, CResult, CollisionModelOption, HBox, IResult, Qc, QcCommand,
    RenderMode, Sequence, SequenceOption,
};
use crate::utils::{
    between_braces, between_space, discard_comment_lines, double, dvec3, line, name_string, number,
};

fn command<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    preceded(tuple((multispace0, tag(s), multispace0)), f)
}

fn qc_command<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<T>,
    qc: impl Fn(T) -> QcCommand,
) -> impl FnMut(&'a str) -> CResult {
    map(terminated(command(s, f), multispace0), qc)
}

fn parse_modelname(i: &str) -> CResult {
    qc_command("$modelname", name_string, |modelname| {
        QcCommand::ModelName(modelname.to_string())
    })(i)
}

fn parse_cd(i: &str) -> CResult {
    qc_command("$cd", name_string, |cd| QcCommand::Cd(cd.to_string()))(i)
}

// $cdtexture is for GoldSrc
fn parse_cd_texture(i: &str) -> CResult {
    qc_command("$cdtexture", name_string, |cd_texture| {
        QcCommand::CdTexture(cd_texture.to_string())
    })(i)
}

// $cdmaterials is for Source
fn parse_cd_materials(i: &str) -> CResult {
    qc_command("$cdmaterials", name_string, |cd_materials| {
        QcCommand::CdMaterials(cd_materials.to_string())
    })(i)
}

fn parse_scale(i: &str) -> CResult {
    qc_command("$scale", double, QcCommand::Scale)(i)
}

fn parse_texrendermode(i: &str) -> CResult {
    qc_command(
        "$texrendermode",
        tuple((name_string, preceded(space0, between_space))),
        |(texture, render)| QcCommand::TextureRenderMode {
            texture: texture.to_string(),
            render: RenderMode::from(render),
        },
    )(i)
}

fn parse_cbox(i: &str) -> CResult {
    qc_command("$cbox", tuple((dvec3, dvec3)), |(mins, maxs)| {
        QcCommand::CBox(CBox(BBox { mins, maxs }))
    })(i)
}

fn parse_bbox(i: &str) -> CResult {
    qc_command("$bbox", tuple((dvec3, dvec3)), |(mins, maxs)| {
        QcCommand::BBox(BBox { mins, maxs })
    })(i)
}

fn body(i: &str) -> IResult<Body> {
    map(
        tuple((
            name_string,
            preceded(space0, name_string),
            opt(preceded(space0, tag("reverse"))),
            opt(preceded(space0, double)),
        )),
        |(name, mesh, reverse, scale)| Body {
            name: name.to_string(),
            mesh: mesh.to_string(),
            reverse: reverse.is_some(),
            scale,
        },
    )(i)
}

fn parse_body(i: &str) -> CResult {
    qc_command("$body", body, QcCommand::Body)(i)
}

fn sequence_option<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<T>,
    cm: impl Fn(T) -> SequenceOption,
) -> impl FnMut(&'a str) -> IResult<SequenceOption> {
    map(terminated(command(s, f), multispace0), cm)
}

// TODO parse all of the options just in case
fn parse_sequence_option(i: &str) -> IResult<SequenceOption> {
    context(
        format!("Parse command not supported yet {}", i).leak(),
        alt((
            // map(preceded(tag("fps"), double), |fps| SequenceOption::Fps(fps)),
            sequence_option("fps", double, SequenceOption::Fps),
            sequence_option("frame", tuple((number, number)), |(start, end)| {
                SequenceOption::Frame { start, end }
            }),
            sequence_option("origin", dvec3, SequenceOption::Origin),
            sequence_option("angles", dvec3, SequenceOption::Angles),
            sequence_option("rotate", double, SequenceOption::Rotate),
            sequence_option("reverse", take(0usize), |_| SequenceOption::Reverse),
            sequence_option("loop", take(0usize), |_| SequenceOption::Loop),
            sequence_option("hidden", take(0usize), |_| SequenceOption::Hidden),
            sequence_option("noanimation", take(0usize), |_| SequenceOption::NoAnimation),
            sequence_option("fadein", double, SequenceOption::FadeIn),
            sequence_option("fadeout", double, SequenceOption::FadeOut),
            sequence_option(
                "activity",
                tuple((name_string, double)),
                |(name, weight)| SequenceOption::Activity {
                    name: name.to_string(),
                    weight,
                },
            ), // This should be last because it will match anything.
               // TODO i dont understnad the format
               // map(
               //     tuple((name_string, opt(number), preceded(space0, between_space))),
               //     |(motion, endframe, axis)| SequenceOption::MotionExtractAxis {
               //         motion: motion.to_string(),
               //         endframe,
               //         axis: axis.to_string(),
               //     },
               // ),
        )),
    )(i)
}

// TODO: make it works like how studiomdl works (very complicated)
fn parse_sequence(i: &str) -> CResult {
    // I am not going to sugarcoat it.
    let (i, _) = terminated(tag("$sequence"), space0)(i)?;

    // They might or might not have quotation mark. Very great.
    let (i, name) = terminated(name_string, multispace0)(i)?;

    // Now check if we have brackets because it is very problematic.
    let (i, is_bracket) = map(opt(peek(tag("{"))), |s| s.is_some())(i)?;

    // If not is simple, it means the next one will definitely be the smd file.
    // For now smd is synonymous with the skeletal
    // It could be another linked animation
    // TODO: care about more things
    let (i, smd, options) = if is_bracket {
        let (i, between) = between_braces(rest)(i)?;

        // TODO for the time being we won't care about activity being the first thing
        let (between, _) = opt(tag("activity"))(between)?;

        let (between, smd) = delimited(multispace0, name_string, multispace0)(between)?;

        let (between, options) =
            many0(delimited(multispace0, parse_sequence_option, multispace0))(between)?;

        // just in case
        if !between.is_empty() {
            return context(
                format!("Sequence parse between bracket did not consume all: {between}").leak(),
                fail,
            )(i);
        }

        (i, smd, options)
    } else {
        let (i, smd) = terminated(name_string, space0)(i)?;
        let (i, options) = delimited(
            space0,
            many0(preceded(space0, parse_sequence_option)),
            multispace0,
        )(i)?;

        (i, smd, options)
    };

    // Consume all end lines to be paritiy with the other commands
    let (i, _) = multispace0(i)?;

    Ok((
        i,
        QcCommand::Sequence(Sequence {
            name: name.to_string(),
            skeletal: smd.to_string(),
            options,
        }),
    ))
}

fn parse_clip_to_textures(i: &str) -> CResult {
    qc_command("$cliptotextures", take(0usize), |_| {
        QcCommand::ClipToTextures
    })(i)
}

fn parse_eye_position(i: &str) -> CResult {
    qc_command("$eyeposition", dvec3, QcCommand::EyePosition)(i)
}

fn parse_bodygroup(i: &str) -> CResult {
    qc_command(
        "$bodygroup",
        tuple((
            name_string,
            between_braces(many0(delimited(multispace0, body, multispace0))),
        )),
        |(name, bodies)| {
            QcCommand::BodyGroup(BodyGroup {
                name: name.to_string(),
                bodies,
            })
        },
    )(i)
}

fn parse_static_prop(i: &str) -> CResult {
    qc_command("$staticprop", take(0usize), |_| QcCommand::StaticProp)(i)
}

fn parse_surface_prop(i: &str) -> CResult {
    qc_command("$surfaceprop", name_string, |name| {
        QcCommand::SurfaceProp(name.to_string())
    })(i)
}

fn parse_contents(i: &str) -> CResult {
    qc_command("$contents", line(rest), |content| {
        QcCommand::Content(content.to_string())
    })(i)
}

fn parse_illum_position(i: &str) -> CResult {
    qc_command(
        "$illumposition",
        line(tuple((dvec3, opt(preceded(space0, name_string))))),
        |(pos, bone_name)| QcCommand::IllumPosition {
            pos,
            bone_name: bone_name.map(|x| x.to_string()),
        },
    )(i)
}

fn parse_texture_group(i: &str) -> CResult {
    qc_command(
        "$texturegroup",
        tuple((
            name_string,
            between_braces(many0(between_braces(many0(preceded(
                space0,
                map(name_string, |x| x.to_string()),
            ))))),
        )),
        |(name, groups)| QcCommand::TextureGroup {
            name: name.to_string(),
            groups,
        },
    )(i)
}

fn parse_define_bone(i: &str) -> CResult {
    qc_command(
        "$definebone",
        tuple((
            name_string,
            preceded(space0, name_string),
            dvec3,
            dvec3,
            dvec3,
            dvec3,
        )),
        |(name, parent, origin, rotation, fixup_origin, fixup_rotation)| QcCommand::DefineBone {
            name: name.to_string(),
            parent: parent.to_string(),
            origin,
            rotation,
            fixup_origin,
            fixup_rotation,
        },
    )(i)
}

fn collision_model_option<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<T>,
    cm: impl Fn(T) -> CollisionModelOption,
) -> impl FnMut(&'a str) -> IResult<CollisionModelOption> {
    map(terminated(command(s, f), multispace0), cm)
}

fn parse_collision_model_option(i: &str) -> IResult<CollisionModelOption> {
    context(
        format!("Collision model options not yet supported {}", i).leak(),
        alt((
            collision_model_option("$mass", double, CollisionModelOption::Mass),
            collision_model_option("$inertia", double, CollisionModelOption::Inertia),
            collision_model_option("$damping", double, CollisionModelOption::Damping),
            collision_model_option(
                "$rotdamping",
                double,
                CollisionModelOption::RotationalDamping,
            ),
            collision_model_option("$rootbone", name_string, |str| {
                CollisionModelOption::RootBone(str.to_string())
            }),
            collision_model_option("$concave", take(0usize), |_| CollisionModelOption::Concave),
            collision_model_option(
                "$maxconvexpieces",
                number,
                CollisionModelOption::MaxConvexPieces,
            ),
        )),
    )(i)
}

fn parse_collision_model(i: &str) -> CResult {
    qc_command(
        "$collisionmodel",
        tuple((
            name_string,
            between_braces(many0(delimited(
                multispace0,
                parse_collision_model_option,
                multispace0,
            ))),
        )),
        |(physics, options)| QcCommand::CollisionModel {
            physics: physics.to_string(),
            options,
        },
    )(i)
}

fn parse_mostly_opaque(i: &str) -> CResult {
    qc_command("$mostlyopaque", take(0usize), |_| QcCommand::MostlyOpaque)(i)
}

fn parse_lod(i: &str) -> CResult {
    qc_command(
        "$lod",
        tuple((name_string, between_braces(rest))),
        |(threshold, _)| {
            // threshold might be in quotation mark so we parse it here
            let threshold = threshold.parse::<f64>().unwrap();

            // TODO: parse option
            QcCommand::LoD {
                threshold,
                options: vec![],
            }
        },
    )(i)
}

fn parse_hbox_set(i: &str) -> CResult {
    qc_command("$hboxset", name_string, |name| {
        QcCommand::HBoxSet(name.to_owned())
    })(i)
}

fn parse_hbox(i: &str) -> CResult {
    qc_command(
        "$hbox",
        tuple((
            number,
            preceded(space0, name_string),
            double,
            double,
            double,
            double,
            double,
            double,
        )),
        |(group, bone_name, minx, miny, minz, maxx, maxy, maxz)| {
            QcCommand::HBox(HBox {
                group,
                bone_name: bone_name.to_string(),
                mins: DVec3::new(minx, miny, minz),
                maxs: DVec3::new(maxx, maxy, maxz),
            })
        },
    )(i)
}

fn parse_cast_texture_shadows(i: &str) -> CResult {
    qc_command("$casttextureshadows", take(0usize), |_| {
        QcCommand::CastTextureShadows
    })(i)
}

// Main functions
fn parse_qc_command(i: &str) -> CResult {
    context(
        format!("Parse command not supported yet {}", i).leak(),
        alt((
            // For commands with similar name
            // Either put the longer command first,
            // or have the tag taking the trailing space.
            // Otherwise, it would always fail.
            // I learnt it the hard way.
            alt((parse_bodygroup, parse_body)),
            alt((parse_bbox, parse_cbox, parse_hbox_set, parse_hbox)),
            alt((parse_cd_texture, parse_cd_materials, parse_cd)),
            alt((
                parse_cast_texture_shadows,
                parse_mostly_opaque,
                parse_clip_to_textures,
                parse_static_prop,
            )),
            parse_modelname,
            parse_scale,
            parse_sequence,
            parse_texrendermode,
            parse_eye_position,
            parse_surface_prop,
            parse_contents,
            parse_illum_position,
            parse_texture_group,
            parse_define_bone,
            parse_collision_model,
            parse_lod,
        )),
    )(i)
}

fn parse_qc_commands(i: &str) -> IResult<Vec<QcCommand>> {
    many0(delimited(
        discard_comment_lines,
        parse_qc_command,
        discard_comment_lines,
    ))(i)
}

pub fn parse_qc(i: &str) -> IResult<Qc> {
    map(all_consuming(parse_qc_commands), |commands| Qc { commands })(i)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn name_string_parse1() {
        let i = "what";
        let (rest, s) = name_string(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(s, "what");
    }

    #[test]
    fn name_string_parse2() {
        let i = "\"what\"";
        let (rest, s) = name_string(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(s, "what");
    }

    #[test]
    fn between_space_parse() {
        let i = "\
what
";
        let (rest, s) = between_space(i).unwrap();

        assert_eq!(rest, "\n");
        assert_eq!(s, "what");
    }

    #[test]
    fn between_braces_parse1() {
        let i = "\
{
abc
}";
        let (rest, a) = between_braces(tag("abc"))(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(a, "abc");
    }

    #[test]
    fn between_braces_parse2() {
        let i = "{
    abc
}";
        let (rest, a) = between_braces(tag("abc"))(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(a, "abc");
    }

    #[test]
    fn between_braces_parse3() {
        let i = "{
    abc
    123
    321
}";
        let (rest, a) =
            between_braces(many0(delimited(multispace0, name_string, multispace0)))(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(a.len(), 3);
        assert_eq!(a[0], "abc");
        assert_eq!(a[1], "123");
        assert_eq!(a[2], "321");
    }

    #[test]
    fn between_braces_parse4() {
        let i = "{{}}";
        let (rest, _) = between_braces(rest)(i).unwrap();

        println!("{}", rest);

        assert!(rest.is_empty());
    }

    #[test]
    fn line_parse1() {
        let i = "hahaha
";

        let (rest, a) = line(tag("hahaha"))(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(a, "hahaha")
    }

    #[test]
    fn line_parse2() {
        let i = "\
hohaha

";

        let (rest, a) = line(tag("ho"))(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(a, "ho")
    }

    #[test]
    fn modelname_parse() {
        let i = "$modelname \"/home/khang/map_compiler/model_tools/S2GConverter/test/s1_r012-goldsrc.mdl\"";
        let (rest, modelname) = parse_modelname(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::ModelName(name) = modelname {
            assert_eq!(
                name,
                "/home/khang/map_compiler/model_tools/S2GConverter/test/s1_r012-goldsrc.mdl"
            );
        } else {
            unreachable!()
        }
    }

    #[test]
    fn texrendermode_parse1() {
        let i = "$texrendermode \"metal_light_01_dark.bmp\" fullbright";
        let (rest, rendermode) = parse_texrendermode(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::TextureRenderMode { texture, render } = rendermode {
            assert_eq!(texture, "metal_light_01_dark.bmp");

            assert_eq!(render, RenderMode::FullBright);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn texrendermode_parse2() {
        let i = "$texrendermode \"metal_light_01_dark.bmp\"     flatshade    ";
        let (rest, rendermode) = parse_texrendermode(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::TextureRenderMode { texture, render } = rendermode {
            assert_eq!(texture, "metal_light_01_dark.bmp");

            assert_eq!(render, RenderMode::FlatShade);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn body_parse1() {
        let i = "$body \"studio0\" \"s1_r012_ref_decompiled_part_nr_1_submodel_0\"";
        let (rest, body) = parse_body(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::Body(body) = body {
            assert_eq!(body.name, "studio0");
            assert_eq!(body.mesh, "s1_r012_ref_decompiled_part_nr_1_submodel_0");

            assert!(!body.reverse);
            assert!(body.scale.is_none());
        } else {
            unreachable!()
        }
    }

    #[test]
    fn body_parse2() {
        let i = "$body \"studio0\" \"s1_r012_ref_decompiled_part_nr_1_submodel_0\" reverse 69.10";
        let (rest, body) = parse_body(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::Body(body) = body {
            assert_eq!(body.name, "studio0");
            assert_eq!(body.mesh, "s1_r012_ref_decompiled_part_nr_1_submodel_0");

            assert!(body.reverse);
            assert_eq!(body.scale.unwrap(), 69.1);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn sequence_parse1() {
        let i = "$sequence idle \"idle\"";
        let (rest, sequence) = parse_sequence(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::Sequence(Sequence {
            name,
            skeletal,
            options,
        }) = sequence
        {
            assert_eq!(name, "idle");
            assert_eq!(skeletal, "idle");
            assert!(rest.is_empty());
            assert!(options.is_empty());
        } else {
            unreachable!()
        }
    }

    #[test]
    fn sequence_parse2() {
        let i = "$sequence idle \"idle\" fps 30 loop";
        let (rest, sequence) = parse_sequence(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::Sequence(Sequence {
            name,
            skeletal,
            options,
        }) = sequence
        {
            assert_eq!(name, "idle");
            assert_eq!(skeletal, "idle");
            assert!(rest.is_empty());

            assert_eq!(options.len(), 2);
            assert!(matches!(options[0], SequenceOption::Fps(30.0)));
            assert!(matches!(options[1], SequenceOption::Loop))
        } else {
            unreachable!()
        }
    }

    #[test]
    fn sequence_parse3() {
        let i = "\
$sequence \"idle\" {
	\"arrowframe_anims\\idle.smd\"
	fadein 0.2
	fadeout 0.2
	fps 30
}
";
        let (rest, sequence) = parse_sequence(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(sequence, QcCommand::Sequence { .. }));

        if let QcCommand::Sequence(Sequence {
            name,
            skeletal,
            options,
        }) = sequence
        {
            assert_eq!(name, "idle");
            assert_eq!(skeletal, "arrowframe_anims\\idle.smd");

            assert_eq!(options.len(), 3);
            assert!(matches!(options[2], SequenceOption::Fps(30.0)));
            assert!(matches!(options[1], SequenceOption::FadeOut(0.2)));
            assert!(matches!(options[0], SequenceOption::FadeIn(0.2)))
        }
    }

    #[test]
    fn command_parse() {
        let i = "$cbox 0 0 0 0 0 0";
        let (rest, rv) = parse_qc_command(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::CBox(CBox(BBox { mins, maxs })) = rv {
            assert_eq!(mins, maxs);
            assert_eq!(mins, DVec3::new(0., 0., 0.));
        }
    }

    #[test]
    fn commands_parse() {
        let i = "\
$modelname \"/home/khang/map_compiler/model_tools/S2GConverter/test/s1_r012-goldsrc.mdl\"
$cd \".\"
$cdtexture \".\"
$scale 1.0
$texrendermode \"metal_light_01_dark.bmp\" fullbright 
$texrendermode \"metal_light_01_dark.bmp\" flatshade 
$texrendermode \"mefl2_02_dark.bmp\" fullbright 
$texrendermode \"mefl2_02_dark.bmp\" flatshade 
        ";

        let (rest, qc) = parse_qc_commands(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(qc.len(), 8);

        let qc1 = &qc[1];

        if let QcCommand::Cd(path) = qc1 {
            assert_eq!(path, ".");
        } else {
            unreachable!()
        }
    }

    #[test]
    fn commands_parse2() {
        let i = "\
// hello
$modelname \"/home/khang/map_compiler/model_tools/S2GConverter/test/s1_r012-goldsrc.mdl\"
// //
// I am n idiotot
$cd \".\"
$cdtexture \".\"
$scale 1.0
$texrendermode \"metal_light_01_dark.bmp\" fullbright // here
$texrendermode \"metal_light_01_dark.bmp\" flatshade 
// yes?
$texrendermode \"mefl2_02_dark.bmp\" fullbright 
$texrendermode \"mefl2_02_dark.bmp\" flatshade 
// good night
        ";

        let (rest, qc) = parse_qc_commands(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(qc.len(), 8);

        let qc1 = &qc[1];

        if let QcCommand::Cd(path) = qc1 {
            assert_eq!(path, ".");
        } else {
            unreachable!()
        }
    }

    #[test]
    fn read_goldsrc() {
        assert!(Qc::from_file("./test/s1_r012-goldsrc.qc").is_ok());
    }

    // TODO: test source file

    #[test]
    fn write_goldsrc() {
        let file = Qc::from_file("./test/s1_r012-goldsrc.qc").unwrap();

        file.write("./test/out/s1_r012-goldsrc_out.qc").unwrap();
    }

    #[test]
    fn fail_read() {
        let file = Qc::from_file("./dunkin/do.nut");

        assert!(file.is_err());
    }
    #[test]
    fn just_body_parse() {
        let i = "studio chest_with_no_armor";
        let (rest, body) = body(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(body.name, "studio");
        assert_eq!(body.mesh, "chest_with_no_armor");
    }

    #[test]
    fn bodygroup_parse1() {
        let i = "\
$bodygroup body
{
studio \"t1_surf02_nown\"
}";

        let (rest, bodygroup) = parse_bodygroup(i).unwrap();

        assert!(rest.is_empty());

        assert!(matches!(bodygroup, QcCommand::BodyGroup(_)));

        if let QcCommand::BodyGroup(BodyGroup { name, bodies }) = bodygroup {
            assert_eq!(name, "body");
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies[0].name, "studio");
            assert_eq!(bodies[0].mesh, "t1_surf02_nown");
            assert_eq!(bodies[0].reverse, false);
            assert!(bodies[0].scale.is_none());
        }
    }

    #[test]
    fn bodygroup_parse2() {
        let i = "\
$bodygroup chest
{
	studio chest_with_no_armor
	studio chest_with_light_armor
	studio chest_with_heavy_armor
	studio chest_with_super_armor
}";

        let (rest, bodygroup) = parse_bodygroup(i).unwrap();

        assert!(rest.is_empty());

        assert!(matches!(bodygroup, QcCommand::BodyGroup(_)));

        if let QcCommand::BodyGroup(BodyGroup { name, bodies }) = bodygroup {
            assert_eq!(name, "chest");
            assert_eq!(bodies.len(), 4);
            assert_eq!(bodies[3].name, "studio");
            assert_eq!(bodies[3].mesh, "chest_with_super_armor");
            assert_eq!(bodies[3].reverse, false);
            assert!(bodies[3].scale.is_none());
        }
    }

    #[test]
    fn some_read_string() {
        let i = "\
$modelname \"t1_surf02_nown100wn.mdl\"
$cliptotextures
$scale 1.0
$texrendermode glass_cyber_stripes_grey.bmp masked
$texrendermode CHROME_1.bmp additive

$bbox 0.000000 0.000000 0.000000 0.000000 0.000000 0.000000
$cbox 0.000000 0.000000 0.000000 0.000000 0.000000 0.000000
$eyeposition 0.000000 0.000000 0.000000
$body \"mesh\" \"t1_surf02_nown\"
$sequence \"idle\" \"idle\" fps 30

$bodygroup body
{
studio \"t1_surf02_nown\"
}";

        let (rest, _) = parse_qc_commands(i).unwrap();

        assert!(rest.is_empty())
    }

    #[test]
    fn some_out_read_string() {
        let i = "\
$modelname \"t1_surf02_nown100wn.mdl\"
$cd \"\\users\\Keita\\Documents\\VHE\\J.A.C.K\\bspsrc_1.4.3\\surf_lt_omnific_d\\models\\props\\surf_lt\\nyro\"
$cdtexture \"\\users\\Keita\\Documents\\VHE\\J.A.C.K\\bspsrc_1.4.3\\surf_lt_omnific_d\\models\\props\\surf_lt\\nyro\"
$cliptotextures
";

        let (rest, commands) = parse_qc_commands(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(commands.len(), 4);
    }

    #[test]
    fn some_read_write() {
        let file = Qc::from_file("./test/some.qc").unwrap();

        let _ = file.write("./test/out/some_out.qc");

        let file1 = Qc::from_file("./test/some.qc").unwrap();
        let file2 = Qc::from_file("./test/out/some_out.qc").unwrap();

        assert_eq!(file1, file2);
    }

    #[test]
    fn content_parse() {
        let i = "$contents \"monster\" \"grate\"";

        let (rest, a) = parse_contents(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(a, QcCommand::Content(_)));

        if let QcCommand::Content(contents) = a {
            assert_eq!(contents, "\"monster\" \"grate\"")
        }
    }

    #[test]
    fn illum_pos_parse() {
        let i = "$illumposition 0.001 48.002 0";

        let (rest, a) = parse_illum_position(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(a, QcCommand::IllumPosition { .. }));

        if let QcCommand::IllumPosition { pos, bone_name } = a {
            assert_eq!(pos, DVec3::new(0.001, 48.002, 0.));
            assert!(bone_name.is_none());
        }
    }

    #[test]
    fn illum_pos_parse2() {
        let i = "$illumposition 0.001 48.002 0 sneed";

        let (rest, a) = parse_illum_position(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(a, QcCommand::IllumPosition { .. }));

        if let QcCommand::IllumPosition { pos, bone_name } = a {
            assert_eq!(pos, DVec3::new(0.001, 48.002, 0.));
            assert_eq!(bone_name.unwrap(), "sneed");
        }
    }

    #[test]
    fn texture_group_parse() {
        let i = "\
$texturegroup skinfamilies
{
	{ heavy_head_red        eyeball_r      eyeball_l      hvyweapon_red               hvyweapon_red_sheen               }
	{ heavy_head_blue       eyeball_r      eyeball_l      hvyweapon_blue              hvyweapon_blue_sheen              }

	{ heavy_head_red_invun  eyeball_invun  eyeball_invun  hvyweapon_red_invun         hvyweapon_red_invun               }
	{ heavy_head_blue_invun eyeball_invun  eyeball_invun  hvyweapon_blue_invun        hvyweapon_blue_invun              }

	{ heavy_head_zombie     eyeball_zombie eyeball_zombie heavy_red_zombie_alphatest  heavy_red_zombie_alphatest_sheen  }
	{ heavy_head_zombie     eyeball_zombie eyeball_zombie heavy_blue_zombie_alphatest heavy_blue_zombie_alphatest_sheen }

	{ heavy_head_red_invun  eyeball_invun  eyeball_invun  hvyweapon_red_zombie_invun  hvyweapon_red_zombie_invun        }
	{ heavy_head_blue_invun eyeball_invun  eyeball_invun  hvyweapon_blue_zombie_invun hvyweapon_blue_zombie_invun       }
}
";

        let (rest, a) = parse_texture_group(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(a, QcCommand::TextureGroup { .. }));

        if let QcCommand::TextureGroup { name, groups } = a {
            assert_eq!(name, "skinfamilies");

            assert_eq!(groups.len(), 8);
            assert_eq!(groups[0].len(), 5);

            assert_eq!(groups[7][2], "eyeball_invun");
        }
    }

    #[test]
    fn texture_group_parse2() {
        let i = "\
$texturegroup \"skinfamilies\"
{
	{ \"tilefloor01\" \"marblefloor001b\" \"metalflat\" \"gridwall_glow\" \"unbreakable\" }
}

";

        let (rest, i) = parse_texture_group(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(i, QcCommand::TextureGroup { .. }));

        if let QcCommand::TextureGroup { name, groups } = i {
            assert_eq!(name, "skinfamilies");
            assert!(groups.len() == 1);
            assert!(groups[0].len() == 5);
        }
    }

    #[test]
    fn define_bone_parse() {
        let i = "$definebone \"static_prop\" \"\" 0 0 0 0 0 0 0 0 0 0 1 0";

        let (rest, a) = parse_define_bone(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(a, QcCommand::DefineBone { .. }));

        if let QcCommand::DefineBone {
            name,
            parent,
            origin,
            rotation,
            fixup_origin,
            fixup_rotation,
        } = a
        {
            assert_eq!(name, "static_prop");
            assert_eq!(parent, "");
            assert_eq!(fixup_origin, DVec3::new(0., 0., 0.));
            assert_eq!(origin, rotation);
            assert_eq!(origin, fixup_origin);
            assert_eq!(fixup_rotation, DVec3::new(0., 1., 0.));
        }
    }

    #[test]
    fn collision_model_parse() {
        let i = "\
$collisionmodel \"arrowframe_physics.smd\"
{
	$mass 47850.57
	$inertia 1
	$damping 0
	$rotdamping 0
	$rootbone \" \"
	$concave
	$maxconvexpieces 7

}
";
        let (rest, a) = parse_collision_model(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(a, QcCommand::CollisionModel { .. }));

        if let QcCommand::CollisionModel { physics, options } = a {
            assert_eq!(physics, "arrowframe_physics.smd");

            assert!(options.len() == 7)
        }
    }

    #[test]
    fn parse_qc_test() {
        let i = "\
$modelname t1_surf02_nown100wn.mdl
$cd \\users\\Keita\\Documents\\VHE\\J.A.C.K\\bspsrc_1.4.3\\surf_lt_omnific_d\\models\\props\\surf_lt\\nyro
$cdtexture \\users\\Keita\\Documents\\VHE\\J.A.C.K\\bspsrc_1.4.3\\surf_lt_omnific_d\\models\\props\\surf_lt\\nyro
$cliptotextures 
$scale 1
$texrendermode glass_cyber_stripes_grey.bmp masked
$texrendermode CHROME_1.bmp additive
$bbox 0 0 0 0 0 0
$cbox 0 0 0 0 0 0
$eyeposition 0 0 0
$body mesh t1_surf02_nown  
$sequence idle idle fps 30 
$bodygroup body
{
studio t1_surf02_nown  
}
";

        parse_qc(i).unwrap();
    }

    #[test]
    fn cd_material_parse() {
        let i = "\
$cdmaterials \"models\\props\\willbreakanyway_001\\\"";

        let (rest, a) = parse_cd_materials(i).unwrap();

        assert!(rest.is_empty());
        assert!(matches!(a, QcCommand::CdMaterials(_)));

        if let QcCommand::CdMaterials(path) = a {
            assert_eq!(path, "models\\props\\willbreakanyway_001\\")
        }
    }

    #[test]
    fn lod_parse() {
        let i = "\
$lod 225
{
	replacemodel \"pacific_palm004.smd\" \"pacific_palm004_lod3.smd\"
	nofacial
}";

        let (rest, _) = parse_lod(i).unwrap();

        assert!(rest.is_empty());
    }

    #[test]
    fn hbox_parse() {
        let i = "\
$hbox 0 \"static_prop\" 0 0 0 0 0 0";

        let (rest, _) = parse_hbox(i).unwrap();

        assert!(rest.is_empty());
    }

    #[test]
    fn parse_epiphany() {
        let file = Qc::from_file("test/willbreakanyway_001.qc");

        assert!(file.is_ok())
    }

    #[test]
    fn masked_texture() {
        let i = "\
$texturegroup \"skinfamilies\"
{
	{ \"conc_c01_blk2\" \"conc_c01_blu2\" \"{grass2\" \"conc_t02_blk1\" \"conc_t02_blk2\" \"conc_f02_blk2\" \"conc_c01_blk1\" \"{conc_d08_gry3\" \"sky_blue_3\" \"conc_f02_blk1\" \"slime02\" \"{conc_b03_light\" \"conc_f01_wht1\" \"{conc_d06_gry3\" \"32_cyan_3\" \"conc_t04_blk1\" \"stn_p01_wht1\" \"ground_brwn03\" \"conc_t02_wht1\" \"{fern_b\" \"{conc_d03_gry3\" \"conc_c01_red2\" \"{conc_d05_gry3\" \"*telelun\" \"conc_c01_wht1\" \"*lava01\" \"trigger\" \"gilbertthedoggo\" \"stn_p03_blk1\" \"{conc_b01_light\" \"blue_3\" }
}
";

        let (rest, res) = parse_texture_group(i).unwrap();
        assert!(rest.is_empty());

        assert!(matches!(res, QcCommand::TextureGroup { .. }));

        if let QcCommand::TextureGroup { name, groups } = res {
            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].len(), 31);
            assert_eq!(name, "skinfamilies");
        }
    }
}
