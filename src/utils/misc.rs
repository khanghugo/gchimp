use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn maybe_add_extension_to_string(s: &str, ext: &str) -> String {
    let ext_with_dot = format!(".{}", ext);

    if s.ends_with(&ext_with_dot) {
        s.to_string()
    } else {
        format!("{}.{}", s, ext)
    }
}

pub fn find_files_with_ext_in_folder(path: &Path, ext: &str) -> std::io::Result<Vec<PathBuf>> {
    let rd = fs::read_dir(path)?;
    let paths = rd.filter_map(|path| path.ok()).map(|path| path.path());
    let ext_paths = paths
        .filter(|path| path.extension().is_some() && path.extension().unwrap() == ext)
        .collect();

    Ok(ext_paths)
}

pub fn relative_to_less_relative(root: &Path, relative: &Path) -> PathBuf {
    root.join(relative)
}

// i use linux to do things
pub fn fix_backslash(i: &str) -> String {
    i.replace("\\", "/")
}

#[macro_export]
macro_rules! err {
    ($e: ident) => {{
        use eyre::eyre;

        Err(eyre!($e))
    }};

    ($format_string: literal) => {{
        use eyre::eyre;

        Err(eyre!($format_string))
    }};

    ($($arg:tt)*) => {{
        use eyre::eyre;

        Err(eyre!($($arg)*))
    }};
}
