use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use bitflags::bitflags;
use glam::DVec3;
use nom::branch::alt;
use nom::bytes::complete::{take, take_till};
use nom::character::complete::{digit1, multispace0, space0};
use nom::combinator::{all_consuming, fail, map, map_parser, map_res, opt, peek, recognize, rest};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::{delimited, terminated, tuple};
use nom::{
    bytes::complete::tag, number::complete::double as _double, sequence::preceded,
    IResult as _IResult,
};

type IResult<'a, T> = _IResult<&'a str, T>;
type CResult<'a> = IResult<'a, QcCommand>;
type SResult<'a> = IResult<'a, SequenceOption>;

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

#[derive(Debug, Clone, PartialEq)]
pub enum RenderMode {
    Masked,
    Additive,
    FlatShade,
    FullBright,
    Chrome,
}

impl RenderMode {
    fn from(i: &str) -> Self {
        match i {
            "masked" => Self::Masked,
            "additive" => Self::Additive,
            "flatshade" => Self::FlatShade,
            "fullbright" => Self::FullBright,
            "chrome" => Self::Chrome,
            _ => unreachable!(
                "\
Invalid string for conversion to RenderMode `{}`
Check your QC file to make sure $texrendermode is correct.",
                i
            ),
        }
    }

    fn to_string(self) -> String {
        match self {
            Self::Masked => "masked".to_string(),
            Self::Additive => "additive".to_string(),
            Self::FlatShade => "flatshade".to_string(),
            Self::FullBright => "fullbright".to_string(),
            Self::Chrome => "chrome".to_string(),
        }
    }
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
    pub skeletal: String,
    pub options: Vec<SequenceOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SequenceOption {
    Frame {
        start: i32,
        end: i32,
    },
    Origin(DVec3),
    Angles(DVec3),
    Rotate(f64),
    Scale(f64),
    Reverse,
    Loop,
    Hidden,
    NoAnimation,
    Fps(f64),
    MotionExtractAxis {
        motion: String,
        endframe: Option<i32>,
        axis: String,
    },
    Activity {
        name: String,
        weight: f64,
    },
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
    TextureGroup {
        name: String,
        // Vec of vec because each texture name is corresponding with another one
        groups: Vec<Vec<String>>,
    },
    RenameBone(RenameBone),
    MirrorBone(String),
    Include(String),
    Attachment(Attachment),
    HBox(HBox),
    Controller(Controller),
    Sequence(Sequence),
    StaticProp,
    SurfaceProp(String),
    // TODO make it a vector or something
    Content(String),
    IllumPosition {
        pos: DVec3,
        bone_name: Option<String>,
    },
    DefineBone {
        name: String,
        // parent should explicitly be quoted text when write
        parent: String,
        origin: DVec3,
        rotation: DVec3,
        fixup_origin: DVec3,
        fixup_rotation: DVec3,
    },
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

fn number(i: &str) -> IResult<i32> {
    preceded(space0, _number)(i)
}

fn signed_double(i: &str) -> IResult<f64> {
    map(recognize(preceded(opt(tag("-")), _double)), |what: &str| {
        what.parse().unwrap()
    })(i)
}

pub fn double(i: &str) -> IResult<f64> {
    preceded(space0, signed_double)(i)
}

fn quoted_text(i: &str) -> IResult<&str> {
    terminated(preceded(tag("\""), take_till(|c| c == '\"')), tag("\""))(i)
}

fn dvec3(i: &str) -> IResult<DVec3> {
    map(tuple((double, double, double)), |(x, y, z)| {
        DVec3::new(x, y, z)
    })(i)
}

// Do not consume space at the end because we don't know if we are at the end of line or not.
// This is pretty dangerous and it might take braces or any kind of arbitrary delimiter.
fn between_space(i: &str) -> IResult<&str> {
    let (i, res) = take_till(|c| c == ' ' || c == '\n' || c == '\r')(i)?;

    if res.is_empty() {
        Ok(fail(i)?)
    } else {
        Ok((i, res))
    }
}

// Filee name may or may not have quotation mark.
fn name_string(i: &str) -> IResult<&str> {
    alt((quoted_text, between_space))(i)
}

fn discard_comment_line(i: &str) -> IResult<&str> {
    terminated(
        preceded(tuple((multispace0, tag("//"))), take_till(|c| c == '\n')),
        multispace0,
    )(i)
}

fn discard_comment_lines(i: &str) -> IResult<&str> {
    map(many0(discard_comment_line), |_| "")(i)
}

// https://github.com/getreu/parse-hyperlinks/blob/5af034d14aa72ffb9e705da13bf557a564b1bebf/parse-hyperlinks/src/lib.rs#L41
pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&str) -> IResult<&str> {
    move |i: &str| {
        let mut index = 0;
        let mut bracket_counter = 0;
        while let Some(n) = &i[index..].find(&[opening_bracket, closing_bracket, '\\'][..]) {
            index += n;
            let mut it = i[index..].chars();
            match it.next() {
                Some(c) if c == '\\' => {
                    // Skip the escape char `\`.
                    index += '\\'.len_utf8();
                    // Skip also the following char.
                    if let Some(c) = it.next() {
                        index += c.len_utf8();
                    }
                }
                Some(c) if c == opening_bracket => {
                    bracket_counter += 1;
                    index += opening_bracket.len_utf8();
                }
                Some(c) if c == closing_bracket => {
                    // Closing bracket.
                    bracket_counter -= 1;
                    index += closing_bracket.len_utf8();
                }
                // Can not happen.
                _ => unreachable!(),
            };
            // We found the unmatched closing bracket.
            if bracket_counter == -1 {
                // We do not consume it.
                index -= closing_bracket.len_utf8();
                return Ok((&i[index..], &i[0..index]));
            };
        }

        if bracket_counter == 0 {
            Ok(("", i))
        } else {
            Ok(fail(i)?)
        }
    }
}

fn between_braces<'a, T>(
    f: impl FnMut(&'a str) -> IResult<T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    // Look ahead approach to avoid using name_string / between_space
    // between_space is very bad for things like this.

    map_parser(
        preceded(
            tuple((multispace0, tag("{"), multispace0)),
            terminated(
                take_until_unbalanced('{', '}'),
                tuple((tag("}"), multispace0)),
            ),
        ),
        f,
    )
}

fn line<'a, T>(f: impl FnMut(&'a str) -> IResult<T>) -> impl FnMut(&'a str) -> IResult<'a, T> {
    // Take a line separated by either \r\n or just \n by looking ahead.
    map_parser(
        preceded(
            space0,
            terminated(take_till(|c| c == '\n' || c == '\r'), multispace0),
        ),
        f,
    )
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
    qc_command("$modelname", name_string, |modelname| {
        QcCommand::ModelName(modelname.to_string())
    })(i)
}

fn parse_cd(i: &str) -> CResult {
    qc_command("$cd", name_string, |cd| QcCommand::Cd(cd.to_string()))(i)
}

fn parse_cd_texture(i: &str) -> CResult {
    qc_command("$cdtexture", name_string, |cd_texture| {
        QcCommand::CdTexture(cd_texture.to_string())
    })(i)
}

fn parse_scale(i: &str) -> CResult {
    qc_command("$scale", double, QcCommand::Scale)(i)
}

fn parse_texrendermode(i: &str) -> CResult {
    qc_command(
        "$texrendermode",
        tuple((name_string, preceded(space0, between_space))),
        |(texture, render)| {
            QcCommand::TextureRenderMode(TextureRenderMode {
                texture: texture.to_string(),
                render: RenderMode::from(render),
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
    qc_command("$body", body, |body| QcCommand::Body(body))(i)
}

// TODO parse all of the options just in case
fn parse_sequence_option(i: &str) -> SResult {
    context(
        format!("Parse command not supported yet {}", i).leak(),
        alt((
            map(preceded(tag("fps"), double), |fps| SequenceOption::Fps(fps)),
            map(
                preceded(tag("frame"), tuple((number, number))),
                |(start, end)| SequenceOption::Frame { start, end },
            ),
            map(preceded(tag("origin"), dvec3), |what| {
                SequenceOption::Origin(what)
            }),
            map(preceded(tag("angles"), dvec3), |what| {
                SequenceOption::Angles(what)
            }),
            map(preceded(tag("rotate"), double), |what| {
                SequenceOption::Rotate(what)
            }),
            map(tag("reverse"), |_| SequenceOption::Reverse),
            map(tag("loop"), |_| SequenceOption::Loop),
            map(tag("hidden"), |_| SequenceOption::Hidden),
            map(tag("noanimation"), |_| SequenceOption::NoAnimation),
            map(preceded(tag("fadein"), double), |seconds| {
                SequenceOption::FadeIn(seconds)
            }),
            map(preceded(tag("fadeout"), double), |seconds| {
                SequenceOption::FadeOut(seconds)
            }),
            // This should be last because it will match anything.
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

        let (between, smd) = delimited(multispace0, name_string, multispace0)(between)?;

        let (between, options) =
            many0(delimited(multispace0, parse_sequence_option, multispace0))(between)?;

        // just in case
        assert!(between.is_empty());

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
    qc_command("$eyeposition", dvec3, |pos| QcCommand::EyePosition(pos))(i)
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
            parse_bodygroup,
            parse_bbox,
            parse_cbox,
            parse_cd_texture,
            parse_cd,
            parse_modelname,
            parse_scale,
            parse_sequence,
            parse_texrendermode,
            parse_clip_to_textures,
            parse_eye_position,
            parse_body,
            parse_static_prop,
            parse_surface_prop,
            parse_contents,
            parse_illum_position,
            parse_texture_group,
            parse_define_bone,
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

fn parse_qc(i: &str) -> IResult<Qc> {
    map(all_consuming(parse_qc_commands), |commands| Qc { commands })(i)
}

// Remember to add new line
// Without quotation mark works just fine.
fn write_qc_command(i: QcCommand) -> Vec<u8> {
    (match i {
        QcCommand::ModelName(x) => format!("$modelname \"{}\"\n", x),
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
        ),
        QcCommand::Cd(x) => format!("$cd \"{}\"\n", x),
        QcCommand::CdTexture(x) => format!("$cdtexture \"{}\"\n", x),
        QcCommand::ClipToTextures => "$cliptotextures \n".to_string(),
        QcCommand::Scale(x) => format!("$scale {}\n", x),
        QcCommand::TextureRenderMode(TextureRenderMode { texture, render }) => {
            format!("$texrendermode {} {}\n", texture, render.to_string(),)
        }
        QcCommand::Gamma(x) => format!("$gamma {}\n", x),
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
        ),
        QcCommand::BBox(BBox { mins, maxs }) => format!(
            "$bbox {} {} {} {} {} {}\n",
            mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
        ),
        QcCommand::CBox(BBox { mins, maxs }) => format!(
            "$cbox {} {} {} {} {} {}\n",
            mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
        ),
        QcCommand::Flags(Flags(x)) => format!("$flags {}\n", x),
        QcCommand::HBox(HBox {
            group,
            bone_name,
            mins,
            maxs,
        }) => format!(
            "$hbox {} {} {} {} {} {} {} {}\n",
            group, bone_name, mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
        ),
        QcCommand::Sequence(Sequence {
            name,
            skeletal,
            options,
        }) => {
            let mut res = format!("$sequence {} ", name);

            res += format!("{} ", skeletal).as_str();

            // For a very lazy reason, everything is INLINE
            // TODO maybe don't do inline to make it look prettier
            for option in options {
                // No need to add space. Will add space after this
                res += (match option {
                    SequenceOption::Frame { start, end } => format!("frame {} {}", start, end),
                    SequenceOption::Origin(_) => todo!(),
                    SequenceOption::Angles(_) => todo!(),
                    SequenceOption::Rotate(_) => todo!(),
                    SequenceOption::Scale(scale) => format!("scale {}", scale),
                    SequenceOption::Reverse => format!("reverse "),
                    SequenceOption::Loop => format!("loop"),
                    SequenceOption::Hidden => format!("hidden"),
                    SequenceOption::NoAnimation => format!("noanimation"),
                    SequenceOption::Fps(fps) => format!("fps {}", fps),
                    SequenceOption::MotionExtractAxis {
                        motion,
                        endframe,
                        axis,
                    } => format!(
                        "{} {} {}",
                        motion,
                        endframe.map(|x| x.to_string()).unwrap_or("".to_string()),
                        axis
                    ),
                    SequenceOption::Activity { name, weight } => {
                        format!("activity {} {}", name, weight)
                    }
                    SequenceOption::AutoPlay => format!("autoplay"),
                    SequenceOption::AddLayer(_) => todo!(),
                    SequenceOption::BlendLayer(_) => todo!(),
                    SequenceOption::WorldSpace => format!("worldspace"),
                    SequenceOption::WorldBlendSpace => format!("worldspaceblend"),
                    SequenceOption::Snap => format!("snap"),
                    SequenceOption::RealTime => format!("realtime"),
                    SequenceOption::FadeIn(_) => todo!(),
                    SequenceOption::FadeOut(_) => todo!(),
                    SequenceOption::WeightList(_) => todo!(),
                    SequenceOption::WorldRelative => todo!(),
                    SequenceOption::LocalHierarchy(_) => todo!(),
                    SequenceOption::Compress(_) => todo!(),
                    SequenceOption::PoseCycle(_) => todo!(),
                    SequenceOption::NumFrames(_) => todo!(),
                })
                .as_str();

                // Add space at the end for each.
                res += " ";
            }

            res += "\n";
            res
        }
        QcCommand::EyePosition(what) => format!("$eyeposition {} {} {}\n", what.x, what.y, what.z),
        QcCommand::BodyGroup(BodyGroup { name, bodies }) => {
            let mut res = format!("$bodygroup {} {{\n", name);

            for body in bodies {
                res += format!(
                    "{} {} {} {}\n",
                    body.name,
                    body.mesh,
                    if body.reverse { "reverse" } else { "" },
                    body.scale.map(|x| x.to_string()).unwrap_or("".to_string())
                )
                .as_str();
            }

            res += "}\n";
            res
        }
        _ => unimplemented!(
            "Write command `{:?}` not implemented. Go ask me. I will do it.",
            i
        ),
    })
    .into_bytes()
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
        let file = Qc::new("./test/some.qc").unwrap();

        let _ = file.write("./test/out/some_out.qc");

        let file1 = Qc::new("./test/some.qc").unwrap();
        let file2 = Qc::new("./test/out/some_out.qc").unwrap();

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
    fn texturegroup_parse() {
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
        };
    }
}
