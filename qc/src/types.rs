use std::{
    fmt,
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use glam::DVec3;

use bitflags::bitflags;

use eyre::eyre;

use nom::IResult as _IResult;

use crate::parser::parse_qc;

pub type IResult<'a, T> = _IResult<&'a str, T>;
pub type CResult<'a> = IResult<'a, QcCommand>;

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
    pub fn from(i: &str) -> Self {
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
pub struct CBox(pub BBox);

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
pub enum LoDOption {
    ReplaceModel {
        reference: String,
        lod: String,
        reverse: Option<bool>,
    },
    RemoveModel {
        reference: String,
    },
    ReplaceMaterial {
        reference: String,
        lod: String,
    },
    RemoveMesh {
        reference: String,
    },
    NoFacial,
    BoneTreeCollapse {
        reference: String,
    },
    ReplaceBone {
        reference: String,
        lod: String,
    },
    UseShadowLoDMaterials,
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
    MostlyOpaque,
    LoD {
        threshold: f64,
        options: Vec<LoDOption>,
    },
    HBoxSet(String),
    CastTextureShadows,
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
            QcCommand::MostlyOpaque => "$mostlyopaque",
            QcCommand::LoD { .. } => "$lod",
            QcCommand::HBoxSet(_) => "$hboxset",
            QcCommand::CastTextureShadows => "$casttextureshadows",
        }
        .to_string()
    }
}

impl fmt::Display for QcCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_name())?;
        write!(f, " ")?;

        match self {
            QcCommand::ModelName(x) => write!(f, "\"{}\"", x),
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
            QcCommand::Cd(x) => write!(f, "\"{}\"", x),
            QcCommand::CdTexture(x) => write!(f, "\"{}\"", x),
            QcCommand::CdMaterials(x) => write!(f, "\"{}\"", x),
            QcCommand::ClipToTextures => Ok(()),
            QcCommand::Scale(x) => write!(f, "{}", x),
            QcCommand::TextureRenderMode { texture, render } => {
                write!(f, "\"{}\" {}", texture, render)
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
            QcCommand::StaticProp => Ok(()),
            QcCommand::SurfaceProp(_) => todo!(),
            QcCommand::Content(_) => todo!(),
            QcCommand::IllumPosition { .. } => todo!(),
            QcCommand::DefineBone { .. } => todo!(),
            QcCommand::CollisionModel { .. } => todo!(),
            QcCommand::MostlyOpaque => todo!(),
            QcCommand::LoD { .. } => todo!(),
            QcCommand::HBoxSet(_) => todo!(),
            QcCommand::CastTextureShadows => Ok(()),
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
/// ```no-run
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
    pub commands: Vec<QcCommand>,
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

    pub fn from_file(path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<Self> {
        let text = std::fs::read_to_string(path)?;

        Self::from(&text)
    }

    pub fn write(&self, path: impl AsRef<Path> + Into<PathBuf>) -> io::Result<()> {
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

    pub fn commands_mut(&mut self) -> &mut Vec<QcCommand> {
        &mut self.commands
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
    pub fn set_model_name(&mut self, name: &str) -> &mut Self {
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

    pub fn set_cd(&mut self, cd_path: &str) -> &mut Self {
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

    pub fn set_cd_texture(&mut self, cd_path: &str) -> &mut Self {
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

    pub fn add_origin(&mut self, x: f64, y: f64, z: f64, rotation: Option<f64>) -> &mut Self {
        let command = QcCommand::Origin(Origin {
            origin: DVec3::from_array([x, y, z]),
            rotation,
        });

        self.add(command)
    }
}
