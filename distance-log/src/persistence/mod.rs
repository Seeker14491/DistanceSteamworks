pub mod impls;

use crate::{ChangelistEntry, LevelInfo};
use anyhow::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("The requested item does not exist.")]
    DoesNotExist,

    #[error("{0}")]
    Other(#[from] Error),
}

pub trait Persistence {
    fn load_query_results(&self) -> Result<Vec<LevelInfo>, LoadError>;
    fn save_query_results(&self, query_results: &[LevelInfo]) -> Result<(), Error>;
    fn load_changelist(&self) -> Result<Vec<ChangelistEntry>, LoadError>;
    fn save_changelist(&self, changelist: &[ChangelistEntry]) -> Result<(), Error>;
}
