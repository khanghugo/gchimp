use std::{
    env,
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

pub const PERSISTENT_STORAGE_FILE_NAME: &str = "data.toml";

#[derive(Debug)]
pub struct PersistentStorage {
    file: File,
    data: PersistentStorageData,
}

pub const PERSISTENT_STORAGE_WADDY_RECENT_WADS_COUNT: usize = 10;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistentStorageData {
    waddy_recent_wads: Option<Vec<String>>,
}

impl PersistentStorage {
    pub fn start() -> eyre::Result<Self> {
        let path = match env::current_exe() {
            Ok(path) => path.parent().unwrap().join(PERSISTENT_STORAGE_FILE_NAME),
            Err(_) => PathBuf::from(PERSISTENT_STORAGE_FILE_NAME),
        };

        #[allow(clippy::suspicious_open_options)]
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            // .truncate(true) // dont truncate otherwise we cannot read the file
            .open(path.clone())?;

        let mut buf = String::new();

        match file.read_to_string(&mut buf) {
            Ok(_) => (),
            Err(_) => {
                file.seek(std::io::SeekFrom::Start(0)).unwrap();
                file.set_len(0).unwrap();
            }
        };

        let data: PersistentStorageData = toml::from_str(&buf).unwrap();

        Ok(PersistentStorage { file, data })
    }

    pub fn get_waddy_recent_wads(&self) -> Option<&Vec<String>> {
        self.data.waddy_recent_wads.as_ref()
    }

    pub fn push_waddy_recent_wads(&mut self, s: &str) -> eyre::Result<()> {
        if let Some(waddy_recent_wads) = &mut self.data.waddy_recent_wads {
            // remove duplicate
            waddy_recent_wads.retain(|e| e != s);

            // insert at the beginning
            // might be a bit anal because this could be better by pushing
            // but it is not that ergonomic to truncate later
            waddy_recent_wads.insert(0, s.to_string());

            // truncate
            waddy_recent_wads.truncate(PERSISTENT_STORAGE_WADDY_RECENT_WADS_COUNT);
        } else {
            self.data.waddy_recent_wads = Some(vec![s.to_string()])
        }

        self.update()
    }

    // Removes an element that is sure in here
    pub fn remove_waddy_recent_wads(&mut self, s: &str) -> eyre::Result<()> {
        if let Some(waddy_recent_wads) = &mut self.data.waddy_recent_wads {
            waddy_recent_wads.retain(|e| e != s);
        }

        self.update()
    }

    /// Writes into file
    pub fn update(&mut self) -> eyre::Result<()> {
        let res = toml::to_string(&self.data)?;

        // hacky shit to clear file
        // let's hope there's no race condition from this from the ui part
        self.clear_data()?;

        self.file.write_all(res.as_bytes())?;
        self.file.flush()?;

        Ok(())
    }

    pub fn clear_data(&mut self) -> eyre::Result<()> {
        self.file.seek(std::io::SeekFrom::Start(0))?;
        self.file.set_len(0)?;

        Ok(())
    }
}
