use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use bitflags::bitflags;
use glam::DVec3;
use nom::branch::alt;
use nom::bytes::complete::take_till;
use nom::character::complete::{digit1, multispace0, space0};
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::multi::many0;
use nom::sequence::{delimited, terminated, tuple};
use nom::{
    bytes::complete::tag, number::complete::double as _double, sequence::preceded,
    IResult as _IResult,
};

type IResult<'a, T> = _IResult<&'a str, T>;
type CResult<'a> = IResult<'a, QcCommand>;

use eyre::eyre;

#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub name: String,
    pub mesh: String,
    // Both of these are optional.
    // If there is "reverse", set true.
    // No need to write reverse
    pub reverse: bool,
    pub scale: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureRenderMode {
    pub texture: String,
    pub render: RenderMode,
}

// Note to self: add new variants to [`parse_rendermode`]
#[derive(Debug, Clone, PartialEq)]
pub enum RenderMode {
    Masked,
    Additive,
    FlatShade,
    FullBright,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Origin {
    pub origin: DVec3,
    pub rotation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BBox {
    pub mins: DVec3,
    pub maxs: DVec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodyGroup {
    pub name: String,
    pub bodies: Vec<Body>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flags(u32);

bitflags! {
    impl Flags: u32 {
        const RocketTrail = 1 << 0;
        const GSmokeTrail = 1 << 1;
        const BloodTrail = 1 << 2;
        const ModelRotation = 1 << 3;
        const Skarg = 1 << 4;
        const ZombieBlood = 1 << 5;
        const DeathKnight = 1 << 6;
        const BSmokeTrail = 1 << 7;
        const NoShadeLight = 1 << 8;
        const ComplexCollision = 1 << 9;
        const ForceSkylight = 1 << 10;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureGroup {
    pub name: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenameBone {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attachment {
    pub id: i32,
    pub bone_name: String,
    // Offset from `bone_name` coordinate
    pub offset: DVec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HBox {
    pub group: i32,
    pub bone_name: String,
    pub mins: DVec3,
    pub maxs: DVec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Controller {
    pub id: i32,
    pub bone_name: String,
    pub axis: String,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub name: String,
    pub mode: SequenceMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SequenceSimpleOption {
    Frame(i32, i32),
    Origin(DVec3),
    Angles(DVec3),
    Rotate(f64),
    Scale(f64),
    Reverse,
    Loop,
    Hidden,
    NoAnimation,
    Fps(f64),
    MotionExtractAxis(String),
    Activity(String, f64),
    AutoPlay,
    AddLayer(String),
    BlendLayer(SequenceOptionBlendLayer),
    WorldSpace,
    WorldBlendSpace,
    Snap,
    RealTime,
    FadeIn(f64),
    FadeOut(f64),
    WeightList(String),
    WorldRelative,
    LocalHierarchy(SequenceOptionLocalHierarchy),
    Compress(i32),
    PoseCycle(String),
    NumFrames(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceOptionLocalHierarchy {
    pub bone: String,
    pub new_parent: String,
    pub range: Option<(i32, i32, i32, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceOptionBlendLayer {
    pub other: String,
    pub startframe: i32,
    pub peakframe: i32,
    pub tailframe: i32,
    pub endframe: i32,
    pub options: Vec<SequenceOptionBlendLayerOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SequenceOptionBlendLayerOption {
    Spline,
    Xfade,
    PoseParameter(String),
    NoBlend,
    Local,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SequenceMode {
    Immedidate(SequenceImmediate),
    Intermediate(SequenceIntermediate),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceImmediate {
    pub smd: String,
    // TODO
    pub rest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceIntermediate {
    pub animation: String,
    // TODO
    pub rest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QcCommand {
    ModelName(String),
    Body(Body),
    Cd(String),
    CdTexture(String),
    ClipToTextures,
    Scale(f64),
    TextureRenderMode(TextureRenderMode),
    Gamma(f64),
    Origin(Origin),
    BBox(BBox),
    CBox(BBox),
    EyePosition(DVec3),
    BodyGroup(BodyGroup),
    Flags(Flags),
    TextureGroup(TextureGroup),
    RenameBone(RenameBone),
    MirrorBone(String),
    Include(String),
    Attachment(Attachment),
    HBox(HBox),
    Controller(Controller),
    Sequence(Sequence),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Qc {
    pub commands: Vec<QcCommand>,
}

impl Qc {
    pub fn new(file_name: &str) -> eyre::Result<Self> {
        let path = Path::new(file_name);
        let file = std::fs::read_to_string(path)?;

        match parse_qc(&file) {
            Ok((_, res)) => Ok(res),
            Err(_) => Err(eyre!("Cannot read file `{}`", file_name)),
        }
    }

    pub fn write(self, file_name: &str) -> io::Result<()> {
        let path = Path::new(file_name);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut file = BufWriter::new(file);

        for command in self.commands {
            file.write_all(&write_qc_command(command))?;
        }

        file.flush()?;

        Ok(())
    }
}

fn _number(i: &str) -> IResult<i32> {
    map_res(recognize(preceded(opt(tag("-")), digit1)), |s: &str| {
        s.parse::<i32>()
    })(i)
}

// fn number(i: &str) -> IResult<i32> {
//     preceded(space0, _number)(i)
// }

fn signed_double(i: &str) -> IResult<f64> {
    map(recognize(preceded(opt(tag("-")), _double)), |what: &str| {
        what.parse().unwrap()
    })(i)
}

pub fn double(i: &str) -> IResult<f64> {
    preceded(space0, signed_double)(i)
}

fn quoted_text(i: &str) -> IResult<&str> {
    terminated(preceded(tag("\""), take_till(|c| c == '"')), tag("\""))(i)
}

fn dvec3(i: &str) -> IResult<DVec3> {
    map(tuple((double, double, double)), |(x, y, z)| {
        DVec3::new(x, y, z)
    })(i)
}

// Do not consume space at the end because we don't know if we are at the end of line or not.
fn between_space(i: &str) -> IResult<&str> {
    take_till(|c| c == ' ' || c == '\n')(i)
}

// Filee name may or may not have quotation mark.
fn name_string(i: &str) -> IResult<&str> {
    alt((quoted_text, between_space))(i)
}

fn discard_comment_line(i: &str) -> IResult<&str> {
    terminated(
        preceded(tuple((space0, tag("//"))), take_till(|c| c == '\n')),
        multispace0,
    )(i)
}

fn discard_comment_lines(i: &str) -> IResult<&str> {
    map(many0(discard_comment_line), |_| "")(i)
}

// Main commands
fn command<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    preceded(tuple((space0, tag(s), space0)), f)
}

fn qc_command<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<T>,
    qc: impl Fn(T) -> QcCommand,
) -> impl FnMut(&'a str) -> CResult {
    map(terminated(command(s, f), multispace0), qc)
}

fn parse_modelname(i: &str) -> CResult {
    qc_command("$modelname", quoted_text, |modelname| {
        QcCommand::ModelName(modelname.to_string())
    })(i)
}

fn parse_cd(i: &str) -> CResult {
    qc_command("$cd", quoted_text, |cd| QcCommand::Cd(cd.to_string()))(i)
}

fn parse_cd_texture(i: &str) -> CResult {
    qc_command("$cdtexture", quoted_text, |cd_texture| {
        QcCommand::CdTexture(cd_texture.to_string())
    })(i)
}

fn parse_scale(i: &str) -> CResult {
    qc_command("$scale", double, QcCommand::Scale)(i)
}

fn parse_rendermode(i: &str) -> IResult<RenderMode> {
    alt((
        map(tag("masked"), |_| RenderMode::Masked),
        map(tag("additive"), |_| RenderMode::Additive),
        map(tag("flatshade"), |_| RenderMode::FlatShade),
        map(tag("fullbright"), |_| RenderMode::FullBright),
    ))(i)
}

fn parse_texrendermode(i: &str) -> CResult {
    qc_command(
        "$texrendermode",
        tuple((quoted_text, preceded(space0, parse_rendermode))),
        |(texture, render)| {
            QcCommand::TextureRenderMode(TextureRenderMode {
                texture: texture.to_string(),
                render,
            })
        },
    )(i)
}

fn parse_cbox(i: &str) -> CResult {
    qc_command("$cbox", tuple((dvec3, dvec3)), |(mins, maxs)| {
        QcCommand::CBox(BBox { mins, maxs })
    })(i)
}

fn parse_bbox(i: &str) -> CResult {
    qc_command("$bbox", tuple((dvec3, dvec3)), |(mins, maxs)| {
        QcCommand::BBox(BBox { mins, maxs })
    })(i)
}

fn parse_body(i: &str) -> CResult {
    qc_command(
        "$body",
        tuple((
            quoted_text,
            preceded(space0, quoted_text),
            opt(preceded(space0, tag("reverse"))),
            opt(preceded(space0, double)),
        )),
        |(name, mesh, reverse, scale)| {
            QcCommand::Body(Body {
                name: name.to_string(),
                mesh: mesh.to_string(),
                reverse: reverse.is_some(),
                scale,
            })
        },
    )(i)
}

// TODO parse all of the options just in case
// fn parse_sequence_options

// TODO: make it works like how studiomdl works (very complicated)
fn parse_sequence(i: &str) -> CResult {
    // I am not going to sugarcoat it.
    let (i, _) = terminated(tag("$sequence"), space0)(i)?;

    // They might or might not have quotation mark. Very great.
    let (i, name) = terminated(name_string, multispace0)(i)?;

    println!("name is {}", name);

    // Now check if we have brackets because it is very problematic.
    let (i, is_bracket) = map(opt(tag("{")), |s| s.is_some())(i)?;

    println!("is bracket {}", is_bracket);

    // If not is simple, it means the next one will definitely be the smd file.
    // TODO: care about more things
    let (i, smd) = if !is_bracket {
        terminated(name_string, space0)(i)?
    } else {
        unimplemented!("For possible S2GConverter rewrite");
    };

    // Consume all end lines to be paritiy with the other commands
    let (i, _) = multispace0(i)?;

    Ok((
        i,
        QcCommand::Sequence(Sequence {
            name: name.to_string(),
            mode: SequenceMode::Immedidate(SequenceImmediate {
                smd: smd.to_string(),
                rest: "".to_string(),
            }),
        }),
    ))
}

// Main functions
fn parse_qc_command(i: &str) -> CResult {
    alt((
        parse_bbox,
        parse_body,
        parse_cbox,
        parse_cd,
        parse_cd_texture,
        parse_modelname,
        parse_scale,
        parse_sequence,
        parse_texrendermode,
    ))(i)
}

fn parse_qc_commands(i: &str) -> IResult<Vec<QcCommand>> {
    many0(delimited(
        discard_comment_lines,
        parse_qc_command,
        discard_comment_lines,
    ))(i)
}

fn parse_qc(i: &str) -> IResult<Qc> {
    map(all_consuming(parse_qc_commands), |commands| Qc { commands })(i)
}

// Remember to add new line
// Without quotation mark works just fine.
fn write_qc_command(i: QcCommand) -> Vec<u8> {
    match i {
        QcCommand::ModelName(x) => format!("$modelname {}\n", x).into_bytes(),
        QcCommand::Body(Body {
            name,
            mesh,
            reverse,
            scale,
        }) => format!(
            "$body {} {} {} {}\n",
            name,
            mesh,
            if reverse { "reverse" } else { "" },
            if let Some(scale) = scale {
                scale.to_string()
            } else {
                "".to_string()
            }
        )
        .into_bytes(),
        QcCommand::Cd(x) => format!("$cd {}\n", x).into_bytes(),
        QcCommand::CdTexture(x) => format!("$cdtexture {}\n", x).into_bytes(),
        QcCommand::ClipToTextures => "$cliptotexture \n".to_string().into_bytes(),
        QcCommand::Scale(x) => format!("$scale {}\n", x).into_bytes(),
        QcCommand::TextureRenderMode(TextureRenderMode { texture, render }) => format!(
            "$texrendermode {} {}\n",
            texture,
            match render {
                RenderMode::Masked => "masked",
                RenderMode::Additive => "additive",
                RenderMode::FlatShade => "flatshade",
                RenderMode::FullBright => "fullbright",
            }
        )
        .into_bytes(),
        QcCommand::Gamma(x) => format!("$gamma {}\n", x).into_bytes(),
        QcCommand::Origin(Origin { origin, rotation }) => format!(
            "$origin {} {} {} {}\n",
            origin.x,
            origin.y,
            origin.z,
            if let Some(rotation) = rotation {
                rotation.to_string()
            } else {
                "".to_string()
            }
        )
        .into_bytes(),
        QcCommand::BBox(BBox { mins, maxs }) => format!(
            "$bbox {} {} {} {} {} {}\n",
            mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
        )
        .into_bytes(),
        QcCommand::CBox(BBox { mins, maxs }) => format!(
            "$cbox {} {} {} {} {} {}\n",
            mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
        )
        .into_bytes(),
        QcCommand::Flags(Flags(x)) => format!("$flags {}\n", x).into_bytes(),
        QcCommand::HBox(HBox {
            group,
            bone_name,
            mins,
            maxs,
        }) => format!(
            "$hbox {} {} {} {} {} {} {} {}\n",
            group, bone_name, mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
        )
        .into_bytes(),
        QcCommand::Sequence(Sequence { name, mode }) => {
            let mut res = format!("$sequence {} ", name);

            match mode {
                SequenceMode::Immedidate(SequenceImmediate { smd, rest }) => {
                    res += format!("{} {}\n", smd, rest).as_str()
                }
                SequenceMode::Intermediate(_) => todo!(),
            }

            res.into_bytes()
        }
        _ => unimplemented!("Currently not supported. Too much work. Go ask me. I will do it."),
    }
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

        if let QcCommand::TextureRenderMode(t) = rendermode {
            assert_eq!(t.texture, "metal_light_01_dark.bmp");

            assert_eq!(t.render, RenderMode::FullBright);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn texrendermode_parse2() {
        let i = "$texrendermode \"metal_light_01_dark.bmp\"     flatshade    ";
        let (rest, rendermode) = parse_texrendermode(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::TextureRenderMode(t) = rendermode {
            assert_eq!(t.texture, "metal_light_01_dark.bmp");

            assert_eq!(t.render, RenderMode::FlatShade);
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
            mode: SequenceMode::Immedidate(SequenceImmediate { smd, rest }),
        }) = sequence
        {
            assert_eq!(name, "idle");
            assert_eq!(smd, "idle");
            assert!(rest.is_empty());
        } else {
            unreachable!()
        }
    }

    #[test]
    fn command_parse() {
        let i = "$cbox 0 0 0 0 0 0";
        let (rest, rv) = parse_qc_command(i).unwrap();

        assert!(rest.is_empty());

        if let QcCommand::CBox(BBox { mins, maxs }) = rv {
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
        assert!(Qc::new("./test/s1_r012-goldsrc.qc").is_ok());
    }

    // TODO: test source file

    #[test]
    fn write_goldsrc() {
        let file = Qc::new("./test/s1_r012-goldsrc.qc").unwrap();

        file.write("./test/out/s1_r012-goldsrc_out.qc").unwrap();
    }

    #[test]
    fn fail_read() {
        let file = Qc::new("./dunkin/do.nut");

        assert!(file.is_err());
    }
}
