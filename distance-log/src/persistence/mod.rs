pub mod impls;

use crate::{ChangelistEntry, LevelInfo};
use failure::{Error, Fail};

#[derive(Fail, Debug)]
pub enum LoadError {
    #[fail(display = "The requested item does not exist.")]
    DoesNotExist,

    #[fail(display = "{}", _0)]
    Other(#[fail(cause)] Error),
}

pub trait Persistence {
    fn load_query_results(&self) -> Result<Vec<LevelInfo>, LoadError>;
    fn save_query_results(&self, query_results: &[LevelInfo]) -> Result<(), Error>;
    fn load_changelist(&self) -> Result<Vec<ChangelistEntry>, LoadError>;
    fn save_changelist(&self, changelist: &[ChangelistEntry]) -> Result<(), Error>;
}
