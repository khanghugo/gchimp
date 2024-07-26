use std::path::PathBuf;

use qc::Qc;
use rayon::prelude::*;

use smd::Smd;

use crate::{
    err,
    utils::{
        constants::MAX_SMD_PER_MODEL,
        smd_stuffs::{maybe_split_smd, source_smd_to_goldsrc_smd},
    },
};

/// Splits an SMD into multiple SMD if exceeding vertices count.
pub fn split_smd(smd_path: &str) -> eyre::Result<()> {
    let smd_path = PathBuf::from(smd_path);
    let smd_file_name = smd_path.file_stem().unwrap().to_str().unwrap();

    let smd = Smd::from_file(&smd_path)?;

    let smds = maybe_split_smd(&smd);

    smds.par_iter().enumerate().for_each(|(idx, x)| {
        // the outpath does not have .smd extension lol
        let curr_smd = smd_path.with_file_name(format!("{}{}.smd", smd_file_name, idx));

        x.write(curr_smd).unwrap();
    });

    Ok(())
}

/// The QC file must contain a "master" SMD where it contains all of the triangles.
/// That SMD file must be under the $body command
///
/// THe QC file must contain $modelname, $cd, and $cdtexture
pub fn split_model(qc_path: &str) -> eyre::Result<()> {
    let qc_path = PathBuf::from(qc_path);
    let qc_file_name = qc_path.file_stem().unwrap().to_str().unwrap();

    let qc = Qc::from_file(&qc_path)?;

    let modelname = qc.commands().iter().find_map(|command| {
        if let qc::QcCommand::ModelName(x) = command {
            Some(x)
        } else {
            None
        }
    });

    if modelname.is_none() {
        return err!("Does not contain $modelname");
    }

    let cd = qc.commands().iter().find_map(|command| {
        if let qc::QcCommand::Cd(x) = command {
            Some(x)
        } else {
            None
        }
    });

    if cd.is_none() {
        return err!("Does not contain $cd");
    }

    let cdtexture = qc.commands().iter().find_map(|command| {
        if let qc::QcCommand::CdTexture(x) = command {
            Some(x)
        } else {
            None
        }
    });

    if cdtexture.is_none() {
        return err!("Does not contain $cdtexture");
    }

    let body = qc.commands().iter().find_map(|command| {
        if let qc::QcCommand::Body(x) = command {
            Some(x)
        } else {
            None
        }
    });

    if body.is_none() {
        return err!("Does not contain $body");
    }

    let cd = PathBuf::from(cd.unwrap());
    let body = body.unwrap();
    let modelname = modelname.unwrap();

    let smd = Smd::from_file(cd.join(body.mesh.clone()).with_extension("smd"))?;
    let smd_file_name = body.mesh.clone();

    // this just conveniently fixes lots of things soooooooooooooooooo
    let smds = source_smd_to_goldsrc_smd(&smd);

    // TODO split based on teture count as well
    // subtracting 1 is beacuse if we have 2 smds and 2 smds per model, we only want 1 model
    let model_count = (smds.len() - 1) / MAX_SMD_PER_MODEL + 1;

    (0..model_count).into_par_iter().for_each(|model_idx| {
        let mut new_qc = qc.clone();

        let body_idx = new_qc
            .commands()
            .iter()
            .position(|command| matches!(command, qc::QcCommand::Body(_)))
            .unwrap();

        // we dont need the old body
        new_qc.commands_mut().remove(body_idx);

        // add index to the split model output
        new_qc.set_model_name(
            modelname
                .replace(".mdl", format!("{}.mdl", model_idx).as_str())
                .as_str(),
        );

        // for each smd, we add it into the qc and write the smd file
        smds.chunks(MAX_SMD_PER_MODEL)
            .nth(model_idx)
            .unwrap()
            .iter()
            .enumerate()
            .for_each(|(smd_idx, smd)| {
                let current_smd_name = format!("{}{}{}", smd_file_name, model_idx, smd_idx);

                // TODO maybe insert the command correctly instead of just appending it
                new_qc.add_body(
                    format!("studio{}", smd_idx).as_str(),
                    &current_smd_name,
                    false,
                    None,
                );

                smd.write(cd.join(current_smd_name).with_extension("smd"))
                    .unwrap();
            });

        new_qc
            .write(
                qc_path
                    .with_file_name(format!("{}{}", qc_file_name, model_idx))
                    .with_extension("qc"),
            )
            .unwrap();
    });

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run() {
        split_model("/home/khang/gchimp/examples/split_smd/porunga.qc").unwrap();
    }
}
