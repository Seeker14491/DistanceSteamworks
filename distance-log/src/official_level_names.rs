use crate::{GameModeId, OFFICIAL_LEVELS_FILENAME};
use failure::Error;
use serde_derive::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize)]
pub struct OfficialLevelNames {
    sprint: Box<[String]>,
    challenge: Box<[String]>,
    stunt: Box<[String]>,
}

impl OfficialLevelNames {
    pub fn read() -> Result<Self, Error> {
        Ok(serde_json::from_reader(File::open(
            OFFICIAL_LEVELS_FILENAME,
        )?)?)
    }

    pub fn total_count(&self) -> usize {
        let OfficialLevelNames {
            sprint,
            challenge,
            stunt,
        } = self;

        sprint.len() + challenge.len() + stunt.len()
    }
}

impl<'a> IntoIterator for &'a OfficialLevelNames {
    type Item = (&'a str, GameModeId);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            vec![
                (&self.sprint, GameModeId::Sprint),
                (&self.challenge, GameModeId::Challenge),
                (&self.stunt, GameModeId::Stunt),
            ]
            .into_iter()
            .flat_map(|(names, mode)| names.iter().map(move |x| (x.as_str(), mode))),
        )
    }
}
