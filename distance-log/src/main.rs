#![feature(try_blocks)]

#[macro_use]
extern crate log;

mod cli_args;
mod official_level_names;
mod rpc;

use crate::{
    cli_args::Opt,
    official_level_names::OfficialLevelNames,
    rpc::{LeaderboardResponse, Rpc, RpcRequestExt, WorkshopResponse},
};
use chrono::{DateTime, Utc};
use env_logger::{self, Builder, Env};
use failure::{Error, Fail, ResultExt};
use if_chain::if_chain;
use indicatif::ProgressBar;
use num_integer::Integer;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    fs::File,
    io, process, thread,
};
use thousands::Separable;

const QUERY_RESULTS_FILENAME: &str = "query_results.json";
const CHANGELIST_FILENAME: &str = "changelist.json";
const OFFICIAL_LEVELS_FILENAME: &str = "official_levels.json";

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum GameModeId {
    None,
    Sprint,
    Stunt,
    Soccer,
    FreeRoam,
    ReverseTag,
    LevelEditorPlay,
    CoopSprint,
    Challenge,
    Adventure,
    SpeedAndStyle,
    Trackmogrify,
    Demo,
    MainMenu,
    LostToEchoes,
    Nexus,
    Count,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LevelInfo {
    name: String,
    mode: GameModeId,
    leaderboard_name: String,
    workshop_response: Option<WorkshopResponse>,
    leaderboard_response: LeaderboardResponse,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangelistEntry {
    pub map_name: String,
    pub map_author: Option<String>,
    pub map_preview: Option<String>,
    pub mode: String,
    pub new_recordholder: String,
    pub old_recordholder: Option<String>,
    pub record_new: String,
    pub record_old: Option<String>,
    pub workshop_item_id: Option<String>,
    pub steam_id_author: Option<String>,
    pub steam_id_new_recordholder: String,
    pub steam_id_old_recordholder: Option<String>,
    pub fetch_time: String,
}

impl Display for GameModeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl ChangelistEntry {
    pub fn is_likely_a_duplicate_of(&self, other: &Self) -> bool {
        self.map_name == other.map_name
            && self.mode == other.mode
            && self.record_new == other.record_new
            && self.workshop_item_id == other.workshop_item_id
            && self.steam_id_author == other.steam_id_author
            && self.steam_id_new_recordholder == other.steam_id_new_recordholder
    }
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = cli_args::get();

    if let Err(e) = run(args) {
        print_error(e);
        process::exit(-1);
    }
}

fn run(args: Opt) -> Result<(), Error> {
    let mut client = rpc::client_connect(format!("127.0.0.1:{}", args.port))?;

    if args.delay_start {
        thread::sleep(args.update_delay.into());
    }

    loop {
        info!("Starting update procedure...");
        if let Err(e) = update(&mut client) {
            print_error(e);
        } else {
            info!("Finished update procedure");
        }
        thread::sleep(args.update_delay.into());
    }
}

fn print_error<E: Into<Error>>(e: E) {
    let e = e.into();
    error!("error: {}", e);
    for err in e.iter_causes() {
        error!(" caused by: {}", err);
    }
}

fn update(client: &mut Rpc) -> Result<(), Error> {
    let old_level_infos: Option<Vec<LevelInfo>> = match File::open(QUERY_RESULTS_FILENAME) {
        Ok(mut handle) => {
            let data = serde_json::from_reader(&mut handle)?;
            info!("Loaded previous query results");
            Some(data)
        }
        Err(e) => {
            if let io::ErrorKind::NotFound = e.kind() {
                warn!("Previous query results file not found");
                None
            } else {
                return Err(e.context("Error loading query results file").into());
            }
        }
    };

    let mut changelist: Vec<ChangelistEntry> = match File::open(CHANGELIST_FILENAME) {
        Ok(mut handle) => {
            let data = serde_json::from_reader(&mut handle)?;
            info!("Loaded changelist");
            data
        }
        Err(e) => {
            if let io::ErrorKind::NotFound = e.kind() {
                warn!("changelist file not found");
                Vec::new()
            } else {
                return Err(e.context("Error loading changelist file").into());
            }
        }
    };

    let mut new_level_infos = get_level_infos(client)?;
    if let Some(ref x) = old_level_infos {
        add_missing_entries_from(&mut new_level_infos, x.clone());
    }

    let old_level_infos = match old_level_infos {
        Some(x) => x,
        None => return Ok(()),
    };

    info!("Computing changelist");
    update_changelist(&mut changelist, &mut new_level_infos, old_level_infos);

    serde_json::to_writer(&mut File::create(CHANGELIST_FILENAME)?, &changelist)?;
    serde_json::to_writer(&mut File::create(QUERY_RESULTS_FILENAME)?, &new_level_infos)?;

    Ok(())
}

fn get_level_infos(client: &mut Rpc) -> Result<Vec<LevelInfo>, Error> {
    info!("Fetching workshop levels...");
    let mut levels = get_workshop_levels(client)?;

    info!("Fetching official levels...");
    levels.append(&mut get_official_levels(client)?);

    Ok(levels)
}

fn add_missing_entries_from(level_infos: &mut Vec<LevelInfo>, other: Vec<LevelInfo>) {
    // FIXME: quadratic runtime
    for level_info in other {
        if level_infos
            .iter()
            .find(|x| x.leaderboard_name == level_info.leaderboard_name)
            .is_none()
        {
            level_infos.push(level_info);
        }
    }
}

fn get_official_levels(client: &mut Rpc) -> Result<Vec<LevelInfo>, Error> {
    let mut level_infos = Vec::new();
    let official_levels = OfficialLevelNames::read()?;
    let pbar = ProgressBar::new(official_levels.total_count() as u64);
    for (level_name, mode) in &official_levels {
        let leaderboard_name = create_leaderboard_name_string(level_name, mode, None);
        let leaderboard_response = client
            .GetLeaderboardRange(&leaderboard_name, 1, 2)
            .get()
            .with_context(|_| format!("error downloading leaderboard '{}'", &leaderboard_name))?;
        level_infos.push(LevelInfo {
            name: level_name.to_owned(),
            mode,
            leaderboard_name,
            workshop_response: None,
            leaderboard_response,
            timestamp: Utc::now(),
        });
        pbar.inc(1);
    }

    pbar.finish_and_clear();
    Ok(level_infos)
}

fn get_workshop_levels(client: &mut Rpc) -> Result<Vec<LevelInfo>, Error> {
    let workshop_levels: Vec<WorkshopResponse> =
        client.GetWorkshopLevels(u32::max_value(), "").get()?.into();
    let level_infos: Vec<_> = workshop_levels
        .into_iter()
        .flat_map(|workshop_reponse| {
            [
                ("Sprint", GameModeId::Sprint),
                ("Challenge", GameModeId::Challenge),
                ("Stunt", GameModeId::Stunt),
            ]
            .iter()
            .filter_map(|(s, mode)| {
                if workshop_reponse.tags.iter().any(|x| x == s) {
                    Some((workshop_reponse.clone(), *mode))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
        })
        .collect();
    let pbar = ProgressBar::new(level_infos.len() as u64);
    let level_infos = level_infos
        .into_iter()
        .map(|(workshop_response, mode)| -> Result<_, Error> {
            let filename_without_extension = workshop_response
                .file_name
                .trim_end_matches(".bytes")
                .to_owned();
            let leaderboard_name = create_leaderboard_name_string(
                &filename_without_extension,
                mode,
                Some(workshop_response.steam_id_owner),
            );
            let leaderboard_response = client
                .GetLeaderboardRange(&leaderboard_name, 1, 2)
                .get()
                .with_context(|_| {
                    format!("error downloading leaderboard '{}'", &leaderboard_name)
                })?;
            pbar.inc(1);
            Ok(LevelInfo {
                name: workshop_response.title.clone(),
                mode,
                leaderboard_name,
                workshop_response: Some(workshop_response),
                leaderboard_response,
                timestamp: Utc::now(),
            })
        })
        .collect();

    pbar.finish_and_clear();
    level_infos
}

fn create_leaderboard_name_string(
    level_name: &str,
    game_mode: GameModeId,
    steam_id_owner: Option<u64>,
) -> String {
    if let Some(id) = steam_id_owner {
        format!("{}_{}_{}_stable", level_name, game_mode as u8, id)
    } else {
        format!("{}_{}_stable", level_name, game_mode as u8)
    }
}

fn update_changelist(
    changelist: &mut Vec<ChangelistEntry>,
    new: &mut [LevelInfo],
    old: Vec<LevelInfo>,
) {
    new.sort_by_key(|level_info| {
        level_info
            .workshop_response
            .as_ref()
            .map(|x| x.published_file_id)
            .unwrap_or(0)
    });
    let old: BTreeMap<_, _> = old
        .into_iter()
        .map(|level_info| (level_info.leaderboard_name.clone(), level_info))
        .collect();

    let entries = new.iter().filter_map(|level_info| {
        let LevelInfo {
            name,
            mode,
            leaderboard_name,
            workshop_response,
            leaderboard_response,
            timestamp,
        } = level_info;
        let first_entry = if let Some(x) = leaderboard_response.entries.get(0) {
            x.clone()
        } else {
            return None;
        };

        let (old_recordholder, record_old, steam_id_old_recordholder) = if_chain! {
            if let Some(level_info_old) = old.get(leaderboard_name);
            if let Some(previous_first_entry) = level_info_old.leaderboard_response.entries.get(0);
            then {
                if is_score_better(first_entry.score, previous_first_entry.score, *mode) {
                    (Some(previous_first_entry.player_name.clone()),
                        Some(format_score(previous_first_entry.score, *mode)),
                        Some(format!("{}", previous_first_entry.steam_id)))
                } else {
                    return None;
                }
            } else {
                (None, None, None)
            }
        };

        Some(ChangelistEntry {
            map_name: name.clone(),
            map_author: workshop_response.as_ref().map(|x| x.author_name.clone()),
            map_preview: workshop_response.as_ref().map(|x| x.preview_url.clone()),
            mode: format!("{}", mode),
            new_recordholder: first_entry.player_name,
            old_recordholder,
            record_new: format_score(first_entry.score, *mode),
            record_old,
            workshop_item_id: workshop_response
                .as_ref()
                .map(|x| format!("{}", x.published_file_id)),
            steam_id_author: workshop_response
                .as_ref()
                .map(|x| format!("{}", x.steam_id_owner)),
            steam_id_new_recordholder: format!("{}", first_entry.steam_id),
            steam_id_old_recordholder,
            fetch_time: timestamp.to_rfc2822(),
        })
    });

    let entries: Vec<_> = entries
        .filter(|new_entry| {
            changelist
                .iter()
                .all(|existing_entry| !new_entry.is_likely_a_duplicate_of(existing_entry))
        })
        .rev()
        .collect();

    changelist.extend(entries);
}

fn format_score(score: i32, game_mode: GameModeId) -> String {
    match game_mode {
        GameModeId::Sprint | GameModeId::Challenge => format_score_as_time(score),
        GameModeId::Stunt => format!("{} eV", score.separate_with_commas()),
        _ => unimplemented!(),
    }
}

fn format_score_as_time(score: i32) -> String {
    assert!(score >= 0);

    // `score` is in milliseconds
    let (hours, rem) = score.div_rem(&(1000 * 60 * 60));
    let (minutes, rem) = rem.div_rem(&(1000 * 60));
    let (seconds, rem) = rem.div_rem(&(1000));
    let centiseconds = rem / 10;

    format!(
        "{:02}:{:02}:{:02}.{:02}",
        hours, minutes, seconds, centiseconds
    )
}

fn is_score_better(score_1: i32, score_2: i32, game_mode: GameModeId) -> bool {
    match game_mode {
        GameModeId::Sprint | GameModeId::Challenge => score_1 < score_2,
        GameModeId::Stunt => score_1 > score_2,
        _ => panic!("unsupported GameModeId: {}", game_mode),
    }
}

#[cfg(test)]
mod test {
    use super::format_score_as_time;

    #[test]
    fn test_format_score_as_time() {
        assert_eq!(format_score_as_time(17767890), "04:56:07.89");
    }
}
