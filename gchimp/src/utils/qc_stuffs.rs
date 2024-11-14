use std::path::Path;

use qc::{Qc, QcCommand};

// Base Qc includes ModelName, Cd, CdTexture, CBox, and Scale
// Input `root` means the folder containing the original qc file
// That would help with studiomdl to cd into the correct directory
// Though this is just the bandaid solution for now
// If this were to work with bspsrc decompiled result, might need two paths
pub fn create_goldsrc_base_qc_from_source(source_qc: &Qc, root: &Path) -> Qc {
    let mut new_qc = Qc::new();

    source_qc
        .commands()
        .iter()
        .for_each(|command| match command {
            QcCommand::ModelName(modelname) => {
                new_qc.add(QcCommand::ModelName(modelname.to_string()));
            }
            QcCommand::CBox(cbox) => {
                new_qc.add(QcCommand::CBox(cbox.clone()));
            }
            QcCommand::Scale(scale) => {
                new_qc.add(QcCommand::Scale(scale.to_owned()));
            }
            _ => (),
        });

    new_qc.add(QcCommand::Cd(root.display().to_string()));
    new_qc.add(QcCommand::CdTexture(root.display().to_string()));

    new_qc
}
