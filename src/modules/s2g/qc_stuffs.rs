use super::*;

// Base Qc includes ModelName, Cd, CdTexture, CBox, and Scale
pub fn create_goldsrc_base_qc_from_source(source_qc: &Qc) -> Qc {
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

    new_qc.add(QcCommand::Cd(".".to_string()));
    new_qc.add(QcCommand::CdTexture(".".to_string()));

    new_qc
}
