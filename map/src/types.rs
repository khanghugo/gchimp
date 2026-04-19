use std::collections::HashMap;

use glam::{DVec3, DVec4};

use crate::parser::{parse_brush, parse_brush_plane, parse_entity};

#[derive(Debug, Clone, PartialEq)]
pub struct BrushPlane {
    pub p1: DVec3,
    pub p2: DVec3,
    pub p3: DVec3,
    pub texture_name: TextureName,
    /// Ux Uy Uz Uoffset
    pub u: DVec4,
    /// Vx Vy Vz Voffset
    pub v: DVec4,
    pub rotation: f64,
    pub u_scale: f64,
    pub v_scale: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureName(String);

impl TextureName {
    pub fn new(s: String) -> Self {
        Self(s)
    }
    /// Texture name is uppercase
    ///
    /// Should use this method for doing comparison
    pub fn get_string_standard(&self) -> String {
        self.0.to_uppercase()
    }

    /// Simply returns the string
    pub fn get_string(&self) -> String {
        self.0.clone()
    }
}

impl TryFrom<&str> for BrushPlane {
    type Error = &'static str;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match parse_brush_plane(value) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(err.to_string().leak()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Brush {
    pub planes: Vec<BrushPlane>,
}

impl TryFrom<&str> for Brush {
    type Error = &'static str;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match parse_brush(value) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(err.to_string().leak()),
        }
    }
}

// #[derive(Debug, Clone, PartialEq)]
pub type Attributes = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    // All entities have attributes.
    pub attributes: Attributes,
    pub brushes: Option<Vec<Brush>>,
}

impl TryFrom<&str> for Entity {
    type Error = &'static str;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match parse_entity(value) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(err.to_string().leak()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    pub tb_header: Option<Vec<String>>,
    pub entities: Vec<Entity>,
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}
