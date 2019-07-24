use crate::{
    persistence::{LoadError, Persistence},
    ChangelistEntry, LevelInfo,
};
use failure::Error;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct FileJson {
    query_results_path: PathBuf,
    changelist_path: PathBuf,
}

impl FileJson {
    pub fn new(
        query_results_path: impl Into<PathBuf>,
        changelist_path: impl Into<PathBuf>,
    ) -> Self {
        FileJson {
            query_results_path: query_results_path.into(),
            changelist_path: changelist_path.into(),
        }
    }
}

impl Persistence for &FileJson {
    fn load_query_results(&self) -> Result<Vec<LevelInfo>, LoadError> {
        load_file(&self.query_results_path)
    }

    fn save_query_results(&self, query_results: &[LevelInfo]) -> Result<(), Error> {
        save_file(query_results, &self.query_results_path)
    }

    fn load_changelist(&self) -> Result<Vec<ChangelistEntry>, LoadError> {
        load_file(&self.changelist_path)
    }

    fn save_changelist(&self, changelist: &[ChangelistEntry]) -> Result<(), Error> {
        save_file(changelist, &self.changelist_path)
    }
}

fn load_file<T>(path: &Path) -> Result<T, LoadError>
where
    for<'de> T: Deserialize<'de>,
{
    match File::open(path) {
        Ok(mut handle) => {
            serde_json::from_reader(&mut handle).map_err(|e| LoadError::Other(e.into()))
        }
        Err(e) => {
            if let io::ErrorKind::NotFound = e.kind() {
                Err(LoadError::DoesNotExist)
            } else {
                Err(LoadError::Other(e.into()))
            }
        }
    }
}

fn save_file(data: impl Serialize, path: &Path) -> Result<(), Error> {
    Ok(serde_json::to_writer(&mut File::create(path)?, &data)?)
}
