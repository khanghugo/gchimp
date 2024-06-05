use glam::{DVec2, DVec3};

use crate::{Smd, Triangle, Vertex};

pub trait SmdExtras {
    fn square(material: &str, min: &[f64], max: &[f64], norm: &[f64]) -> [Triangle; 2];
}

impl SmdExtras for Smd {
    fn square(material: &str, min: &[f64], max: &[f64], norm: &[f64]) -> [Triangle; 2] {
        let norm = DVec3::from_slice(norm);
        let parent = 0;
        // start painting from top left to down right
        // top left uv is 0,0
        // down right uv is 1, -1
        // not sure if this is correct but we can come back later
        // give a square
        // A ---- B
        // |      |
        // D ---- C
        // A has coordinate of `min`
        // C has coordinate of `max`

        // ABC
        let tri1 = Triangle {
            material: material.to_owned(),
            vertices: vec![
                // A
                Vertex {
                    parent,
                    pos: DVec3::from_slice(min),
                    norm,
                    uv: DVec2 { x: 1., y: -1. },
                    source: None,
                },
                // C
                Vertex {
                    parent,
                    pos: DVec3::from_slice(max),
                    norm,
                    uv: DVec2 { x: 0., y: 0. },
                    source: None,
                },
                // B
                Vertex {
                    parent,
                    pos: DVec3 {
                        x: min[0],
                        y: max[1],
                        z: min[2],
                    },
                    norm,
                    uv: DVec2 { x: 1., y: 0. },
                    source: None,
                },
            ],
        };

        // ADC
        let tri2 = Triangle {
            material: material.to_owned(),
            vertices: vec![
                // A
                Vertex {
                    parent,
                    pos: DVec3::from_slice(min),
                    norm,
                    uv: DVec2 { x: 1., y: -1. },
                    source: None,
                },
                // D
                Vertex {
                    parent,
                    pos: DVec3 {
                        x: max[0],
                        y: min[1],
                        z: max[2],
                    },
                    norm,
                    uv: DVec2 { x: 0., y: -1. },
                    source: None,
                },
                // C
                Vertex {
                    parent,
                    pos: DVec3::from_slice(max),
                    norm,
                    uv: DVec2 { x: 0., y: 0. },
                    source: None,
                },
            ],
        };

        [tri1, tri2]
    }
}
