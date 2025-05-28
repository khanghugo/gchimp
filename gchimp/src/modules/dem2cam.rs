//! Converts a .dem file to HLAE .cam campath format

// portion copied from bxt-rs
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use dem::open_demo;
use dem::types::Demo;
use glam::Vec3Swizzles;

use crate::utils::dem_stuffs::get_ghost::get_ghost_demo;

#[derive(Clone, Copy)]
pub struct ViewInfo {
    pub vieworg: glam::Vec3,
    pub viewangles: glam::Vec3,
}

#[derive(Clone, Copy)]
pub struct ViewInfoCamIO {
    pub viewinfo: ViewInfo,
    pub time: f32,
    pub fov: f32,
}

#[derive(Clone)]
pub struct CamIO {
    pub campaths: Vec<ViewInfoCamIO>,
}

#[derive(Clone)]
pub struct Exporter {
    data: CamIO,
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter {
    pub fn new() -> Self {
        Self {
            data: CamIO {
                campaths: Vec::new(),
            },
        }
    }

    fn header(&self) -> String {
        "\
advancedfx Cam
version 2
channels time xPosition yPosition zPositon xRotation yRotation zRotation fov
DATA
"
        .to_string()
    }

    pub fn append_entry(
        &mut self,
        time: f32,
        vieworg: glam::Vec3,
        viewangles: glam::Vec3,
        fov: f32,
    ) -> &Self {
        self.data.campaths.push(ViewInfoCamIO {
            viewinfo: ViewInfo {
                vieworg,
                viewangles,
            },
            time,
            fov,
        });
        self
    }

    fn entry_to_string(&self, idx: usize) -> String {
        let curr = self.data.campaths[idx];
        format!(
            "{} {} {} {} {} {} {} {}\n",
            curr.time,
            curr.viewinfo.vieworg[0],
            curr.viewinfo.vieworg[1],
            curr.viewinfo.vieworg[2],
            curr.viewinfo.viewangles[0],
            curr.viewinfo.viewangles[1],
            curr.viewinfo.viewangles[2],
            curr.fov
        )
    }

    pub fn write_to_string(&self) -> String {
        let mut res = String::new();

        res += self.header().as_str();

        for idx in 0..self.data.campaths.len() {
            res += self.entry_to_string(idx).as_str();
        }

        res
    }
}

// end portion copied from bxt-rs

pub struct Dem2CamOptions {
    pub frametime: Option<f32>,
    pub rotation: Option<f32>,
}

impl Default for Dem2CamOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Dem2CamOptions {
    pub fn new() -> Self {
        Self {
            frametime: None,
            rotation: None,
        }
    }
}

// for wasm stuffs
pub fn _dem2cam_string(
    demo: &Demo,
    demo_path: impl AsRef<Path> + Into<PathBuf>,
    options: &Dem2CamOptions,
) -> eyre::Result<String> {
    let filename = demo_path.as_ref().file_stem().unwrap().to_str().unwrap();
    let ghost = get_ghost_demo(filename, demo)?;
    let mut exporter = Exporter::new();

    let Dem2CamOptions {
        frametime,
        rotation: _,
    } = options;

    // if no frametime specified, will use frametime from the demo
    if let Some(frametime) = frametime {
        let mut cum_time: f32 = 0.;

        while let Some(frame) = ghost.get_frame(cum_time as f64, None) {
            exporter.append_entry(
                cum_time,
                frame.origin,
                frame.viewangles.zxy(),
                frame.fov.unwrap(),
            );
            cum_time += *frametime;
        }
    } else {
        let mut cum_time = 0.;

        for frame in ghost.frames {
            exporter.append_entry(cum_time, frame.origin, frame.viewangles, frame.fov.unwrap());
            cum_time += frame.frametime.unwrap() as f32;
        }
    };

    Ok(exporter.write_to_string())
}

pub fn dem2cam(
    demo_path: impl AsRef<Path> + Into<PathBuf>,
    options: &Dem2CamOptions,
) -> eyre::Result<()> {
    let demo = open_demo(demo_path.as_ref())?;
    let res = _dem2cam_string(&demo, demo_path.as_ref(), options)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(demo_path.as_ref().with_extension("cam"))?;
    file.write_all(res.as_bytes())?;
    file.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::{dem2cam, Dem2CamOptions};

    #[test]
    fn run() {
        let path = PathBuf::from("/tmp/aaaaaa/kz_hb_Hopez45MIN.dem");
        // let demo = dem::open_demo(path.as_path()).unwrap();
        dem2cam(
            path,
            &Dem2CamOptions {
                frametime: Some(1.0),
                // frametime: None,
                rotation: None,
            },
        )
        .unwrap();
    }
}
