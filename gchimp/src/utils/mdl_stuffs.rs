use std::{any::Any, path::Path, process::Output, str::from_utf8};

use eyre::eyre;

use super::constants::STUDIOMDL_ERROR_PATTERN;

pub fn handle_studiomdl_output(
    res: Result<Result<Output, eyre::Report>, Box<dyn Any + Send>>,
    path: Option<&Path>,
) -> eyre::Result<()> {
    match res {
        Ok(res) => {
            let output = res.unwrap();
            let stdout = from_utf8(&output.stdout).unwrap();

            let maybe_err = stdout.find(STUDIOMDL_ERROR_PATTERN);

            if let Some(err_index) = maybe_err {
                let err = stdout[err_index + STUDIOMDL_ERROR_PATTERN.len()..].to_string();

                let err_str = if let Some(path) = path {
                    format!("cannot compile {}: {}", path.display(), err.trim())
                } else {
                    format!("cannot compile mdl: {}", err.trim())
                };

                return Err(eyre!(err_str));
            }

            Ok(())
        }
        Err(_) => {
            let err_str = "No idea what happens with running studiomdl. Probably just a dream.";

            Err(eyre!(err_str))
        }
    }
}
