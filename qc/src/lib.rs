use core::fmt;
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
}

impl fmt::Display for RenderMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Masked => "masked",
                Self::Additive => "additive",
                Self::FlatShade => "flatshade",
                Self::FullBright => "fullbright",
                Self::Chrome => "chrome",
            }
        )
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
pub struct CBox(BBox);

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
    WorldSpaceBlend,
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

impl SequenceOption {
    fn get_name(&self) -> String {
        (match self {
            SequenceOption::Frame { .. } => "frame",
            SequenceOption::Origin(_) => todo!(),
            SequenceOption::Angles(_) => todo!(),
            SequenceOption::Rotate(_) => todo!(),
            SequenceOption::Scale(_) => todo!(),
            SequenceOption::Reverse => "reverse",
            SequenceOption::Loop => "loop",
            SequenceOption::Hidden => "hidden",
            SequenceOption::NoAnimation => "noanimation",
            SequenceOption::Fps(_) => "fps",
            SequenceOption::MotionExtractAxis { .. } => todo!(),
            SequenceOption::Activity { .. } => "activity",
            SequenceOption::AutoPlay => "autoplay",
            SequenceOption::AddLayer(_) => todo!(),
            SequenceOption::BlendLayer(_) => "blendlayer",
            SequenceOption::WorldSpace => "worldspace",
            SequenceOption::WorldSpaceBlend => "worldspaceblend",
            SequenceOption::Snap => "snap",
            SequenceOption::RealTime => "realtime",
            SequenceOption::FadeIn(_) => todo!(),
            SequenceOption::FadeOut(_) => todo!(),
            SequenceOption::WeightList(_) => todo!(),
            SequenceOption::WorldRelative => todo!(),
            SequenceOption::LocalHierarchy(_) => todo!(),
            SequenceOption::Compress(_) => todo!(),
            SequenceOption::PoseCycle(_) => todo!(),
            SequenceOption::NumFrames(_) => todo!(),
        })
        .to_string()
    }
}

impl fmt::Display for SequenceOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_name())?;
        write!(f, " ")?;

        match self {
            SequenceOption::Frame { start, end } => write!(f, "{} {}", start, end),
            SequenceOption::Origin(_) => todo!(),
            SequenceOption::Angles(_) => todo!(),
            SequenceOption::Rotate(_) => todo!(),
            SequenceOption::Scale(x) => write!(f, "{}", x),
            SequenceOption::Reverse => Ok(()),
            SequenceOption::Loop => Ok(()),
            SequenceOption::Hidden => Ok(()),
            SequenceOption::NoAnimation => Ok(()),
            SequenceOption::Fps(x) => write!(f, "{}", x),
            SequenceOption::MotionExtractAxis { .. } => todo!(),
            SequenceOption::Activity { name, weight } => write!(f, "{} {}", name, weight),
            SequenceOption::AutoPlay => Ok(()),
            SequenceOption::AddLayer(_) => todo!(),
            SequenceOption::BlendLayer(_) => todo!(),
            SequenceOption::WorldSpace => Ok(()),
            SequenceOption::WorldSpaceBlend => Ok(()),
            SequenceOption::Snap => Ok(()),
            SequenceOption::RealTime => Ok(()),
            SequenceOption::FadeIn(_) => todo!(),
            SequenceOption::FadeOut(_) => todo!(),
            SequenceOption::WeightList(_) => todo!(),
            SequenceOption::WorldRelative => todo!(),
            SequenceOption::LocalHierarchy(_) => todo!(),
            SequenceOption::Compress(_) => todo!(),
            SequenceOption::PoseCycle(_) => todo!(),
            SequenceOption::NumFrames(_) => todo!(),
        }
    }
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
    CdMaterials(String),
    ClipToTextures,
    Scale(f64),
    TextureRenderMode {
        texture: String,
        render: RenderMode,
    },
    Gamma(f64),
    Origin(Origin),
    BBox(BBox),
    CBox(CBox),
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
    CollisionModel {
        physics: String,
        options: Vec<CollisionModelOption>,
    },
}

impl QcCommand {
    fn get_name(&self) -> String {
        match self {
            QcCommand::ModelName(_) => "$modelname",
            QcCommand::Body(_) => "$body",
            QcCommand::Cd(_) => "$cd",
            QcCommand::CdTexture(_) => "$cdtexture",
            QcCommand::ClipToTextures => "$cliptotextures",
            QcCommand::Scale(_) => "$scale",
            QcCommand::TextureRenderMode { .. } => "$texrendermode",
            QcCommand::Gamma(_) => "$gamma",
            QcCommand::Origin(_) => "$origin",
            QcCommand::BBox(_) => "$bbox",
            QcCommand::CBox(_) => "$cbox",
            QcCommand::EyePosition(_) => "$eyeposition",
            QcCommand::BodyGroup(_) => "$bodygroup",
            QcCommand::Flags(_) => "$flags",
            QcCommand::TextureGroup { .. } => "$texturegroup",
            QcCommand::RenameBone(_) => "$renamebone",
            QcCommand::MirrorBone(_) => "$mirrorbone",
            QcCommand::Include(_) => "$include",
            QcCommand::Attachment(_) => "$attachment",
            QcCommand::HBox(_) => "$hbox",
            QcCommand::Controller(_) => "$controller",
            QcCommand::Sequence(_) => "$sequence",
            QcCommand::StaticProp => "$staticprop",
            QcCommand::SurfaceProp(_) => "$surfaceprop",
            QcCommand::Content(_) => "$content",
            QcCommand::IllumPosition { .. } => "$illumposition",
            QcCommand::DefineBone { .. } => "$definebone",
            QcCommand::CollisionModel { .. } => "$collisionmodel",
            QcCommand::CdMaterials(_) => "$cdmaterials",
        }
        .to_string()
    }
}

impl fmt::Display for QcCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_name())?;
        write!(f, " ")?;

        match self {
            QcCommand::ModelName(x) => write!(f, "{}", x),
            QcCommand::Body(Body {
                name,
                mesh,
                reverse,
                scale,
            }) => write!(
                f,
                "{} {} {} {}",
                name,
                mesh,
                if *reverse { "reverse" } else { "" },
                if let Some(scale) = scale {
                    scale.to_string()
                } else {
                    "".to_string()
                }
            ),
            QcCommand::Cd(x) => write!(f, "{}", x),
            QcCommand::CdTexture(x) => write!(f, "{}", x),
            QcCommand::CdMaterials(x) => write!(f, "{}", x),
            QcCommand::ClipToTextures => Ok(()),
            QcCommand::Scale(x) => write!(f, "{}", x),
            QcCommand::TextureRenderMode { texture, render } => {
                write!(f, "{} {}", texture, render)
            }
            QcCommand::Gamma(x) => write!(f, "{}", x),
            QcCommand::Origin(x) => write!(
                f,
                "{} {} {} {}",
                x.origin.x,
                x.origin.y,
                x.origin.z,
                if let Some(rotation) = x.rotation {
                    rotation.to_string()
                } else {
                    "".to_string()
                }
            ),
            QcCommand::BBox(BBox { mins, maxs }) => write!(
                f,
                "{} {} {} {} {} {}",
                mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
            ),
            QcCommand::CBox(CBox(BBox { mins, maxs })) => write!(
                f,
                "{} {} {} {} {} {}",
                mins.x, mins.y, mins.z, maxs.x, maxs.y, maxs.z,
            ),
            QcCommand::EyePosition(x) => write!(f, "{} {} {}", x.x, x.y, x.z),
            QcCommand::BodyGroup(BodyGroup { name, bodies }) => {
                // use writeln! here because we want new line for these.
                writeln!(f, "{}", name)?;
                writeln!(f, "{{")?;

                for body in bodies {
                    // writeln! because each body is one line
                    writeln!(
                        f,
                        "{} {} {} {}",
                        body.name,
                        body.mesh,
                        if body.reverse { "reverse" } else { "" },
                        // body.scale.map(|x| x.to_string()).unwrap_or("".to_string())
                        body.scale.map_or("".to_string(), |x| x.to_string())
                    )?;
                }

                write!(f, "}}")
            }
            QcCommand::Flags(Flags(x)) => write!(f, "{}", x),
            QcCommand::TextureGroup { .. } => todo!(),
            QcCommand::RenameBone(_) => todo!(),
            QcCommand::MirrorBone(_) => todo!(),
            QcCommand::Include(_) => todo!(),
            QcCommand::Attachment(_) => todo!(),
            QcCommand::HBox(_) => todo!(),
            QcCommand::Controller(_) => todo!(),
            QcCommand::Sequence(Sequence {
                name,
                skeletal,
                options,
            }) => {
                // Adding space because lazy
                write!(f, "{} ", name)?;
                write!(f, "{} ", skeletal)?;

                // For a very lazy reason, everything is INLINE
                // TODO maybe don't do inline to make it look prettier
                for option in options {
                    // No need to add space. Will add space after this
                    write!(f, "{}", option)?;

                    // Add space at the end for each.
                    write!(f, " ")?;
                }

                Ok(())
            }
            QcCommand::StaticProp => todo!(),
            QcCommand::SurfaceProp(_) => todo!(),
            QcCommand::Content(_) => todo!(),
            QcCommand::IllumPosition { .. } => todo!(),
            QcCommand::DefineBone { .. } => todo!(),
            QcCommand::CollisionModel { .. } => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollisionModelOption {
    Mass(f64),
    Inertia(f64),
    Damping(f64),
    RotationalDamping(f64),
    RootBone(String),
    Concave,
    MaxConvexPieces(i32),
}

/// The Qc data.
///
/// To access the data, follow this.
/// ```no_run
/// for command in qc.commands {
///     match command {
///         QcCommand::ModelName(modelname) => (),
///         QcCommand::Body(body) => (),
///         _ => (),
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Qc {
    commands: Vec<QcCommand>,
}

impl Default for Qc {
    fn default() -> Self {
        Self::new()
    }
}

impl Qc {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    /// Basic Qc with $scale, $cbox, $bbox
    pub fn new_basic() -> Self {
        let mut qc = Self::new();

        qc.commands.push(QcCommand::Scale(1.));
        qc.commands.push(QcCommand::CBox(CBox(BBox {
            mins: DVec3::ZERO,
            maxs: DVec3::ZERO,
        })));
        qc.commands.push(QcCommand::BBox(BBox {
            mins: DVec3::ZERO,
            maxs: DVec3::ZERO,
        }));

        qc
    }

    pub fn from(text: &str) -> eyre::Result<Self> {
        match parse_qc(text) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse text: {}", err.to_string())),
        }
    }

    pub fn from_file(file_name: &str) -> eyre::Result<Self> {
        let path = Path::new(file_name);
        let text = std::fs::read_to_string(path)?;

        Self::from(&text)
    }

    pub fn write(&self, file_name: &str) -> io::Result<()> {
        let path = Path::new(file_name);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut file = BufWriter::new(file);

        for command in &self.commands {
            file.write_all(command.to_string().as_bytes())?;
            // explicitly write newline
            file.write_all("\n".as_bytes())?;
        }

        file.flush()?;

        Ok(())
    }

    /// Add a [`QcCommand`]
    pub fn add(&mut self, command: QcCommand) -> &mut Self {
        self.commands.push(command);

        self
    }

    pub fn commands(&self) -> &Vec<QcCommand> {
        &self.commands
    }

    /// Add a [`QcCommand::Body`]
    pub fn add_body(
        &mut self,
        name: &str,
        mesh: &str,
        reverse: bool,
        scale: Option<f64>,
    ) -> &mut Self {
        let body = Body {
            name: name.to_string(),
            mesh: mesh.to_string(),
            reverse,
            scale,
        };
        self.add(QcCommand::Body(body))
    }

    pub fn add_sequence(
        &mut self,
        name: &str,
        skeletal: &str,
        options: Vec<SequenceOption>,
    ) -> &mut Self {
        let sequence = Sequence {
            name: name.to_string(),
            skeletal: skeletal.to_string(),
            options,
        };
        self.add(QcCommand::Sequence(sequence))
    }

    /// Sets [`QcCommand::ModelName`] if exists or adds new one
    pub fn add_model_name(&mut self, name: &str) -> &mut Self {
        let model_name = self
            .commands
            .iter_mut()
            .find(|command| matches!(command, QcCommand::ModelName(_)));

        if let Some(QcCommand::ModelName(model_name)) = model_name {
            *model_name = name.to_string();
            self
        } else {
            self.add(QcCommand::ModelName(name.to_string()))
        }
    }

    pub fn add_cd(&mut self, cd_path: &str) -> &mut Self {
        let cd = self
            .commands
            .iter_mut()
            .find(|command| matches!(command, QcCommand::Cd(_)));

        if let Some(QcCommand::Cd(cd)) = cd {
            *cd = cd_path.to_string();
            self
        } else {
            self.add(QcCommand::Cd(cd_path.to_string()))
        }
    }

    pub fn add_cd_texture(&mut self, cd_path: &str) -> &mut Self {
        let cd = self
            .commands
            .iter_mut()
            .find(|command| matches!(command, QcCommand::CdTexture(_)));

        if let Some(QcCommand::CdTexture(cd)) = cd {
            *cd = cd_path.to_string();
            self
        } else {
            self.add(QcCommand::CdTexture(cd_path.to_string()))
        }
    }

    pub fn add_texrendermode(&mut self, texture: &str, render: RenderMode) -> &mut Self {
        let command = QcCommand::TextureRenderMode {
            texture: texture.to_owned(),
            render,
        };

        self.add(command)
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
                Some('\\') => {
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
            parse_cd_materials,
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
            parse_collision_model,
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
    fn parse_epiphany() {
        let file = Qc::from_file("test/willbreakanyway_001.qc");

        assert!(file.is_ok())
    }
}
