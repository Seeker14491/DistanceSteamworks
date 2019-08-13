#![feature(async_await)]
#![recursion_limit = "128"]
#![warn(
    rust_2018_idioms,
    deprecated_in_future,
    macro_use_extern_crate,
    missing_debug_implementations,
    unused_labels,
    unused_qualifications,
    clippy::cast_possible_truncation
)]

mod cli_args;
mod domain;
mod official_levels;
mod persistence;
mod steamworks;

use crate::{
    cli_args::Opt,
    domain::{ChangelistEntry, LevelInfo},
    persistence::{impls::file_json::FileJson, LoadError, Persistence},
    steamworks::Steamworks,
};
use chrono::Utc;
use distance_util::LeaderboardGameMode;
use failure::{Error, Fail};
use futures::prelude::*;
use if_chain::if_chain;
use indicatif::ProgressBar;
use log::{error, info, warn};
use std::{collections::BTreeMap, process};

const QUERY_RESULTS_FILENAME: &str = "query_results.json";
const CHANGELIST_FILENAME: &str = "changelist.json";

#[runtime::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = cli_args::get();

    if let Err(e) = run(args).await {
        print_error(e);
        process::exit(-1);
    }
}

async fn run(_args: Opt) -> Result<(), Error> {
    let steamworks = Steamworks::new()?;
    let persistence = FileJson::new(QUERY_RESULTS_FILENAME, CHANGELIST_FILENAME);

    info!("Starting update procedure");
    update(&steamworks, &persistence).await?;
    info!("Finished update procedure");

    Ok(())
}

fn print_error<E: Into<Error>>(e: E) {
    let e = e.into();
    error!("error: {}", e);
    for err in e.iter_causes() {
        error!(" caused by: {}", err);
    }
}

async fn update(steamworks: &Steamworks, persistence: impl Persistence) -> Result<(), Error> {
    let old_level_infos = match persistence.load_query_results() {
        Ok(x) => {
            info!("Loaded previous query results");
            Some(x)
        }
        Err(e) => {
            if let LoadError::DoesNotExist = e {
                warn!("No previous query results found");
                None
            } else {
                return Err(e.context("Error loading query results").into());
            }
        }
    };

    let mut changelist = match persistence.load_changelist() {
        Ok(x) => {
            info!("Loaded changelist");
            x
        }
        Err(e) => {
            if let LoadError::DoesNotExist = e {
                warn!("No existing changelist found");
                Vec::new()
            } else {
                return Err(e.context("Error loading changelist").into());
            }
        }
    };

    let spinner = ProgressBar::new_spinner();
    let mut new_level_infos = get_level_infos(&steamworks)
        .inspect(|res| {
            if let Ok(level_info) = res {
                spinner.set_message(&format!("Fetched level {}", &level_info.name));
            }
        })
        .try_collect::<Vec<_>>()
        .await?;
    spinner.finish_with_message("Finished fetching level information.");
    if let Some(ref x) = old_level_infos {
        add_missing_entries_from(&mut new_level_infos, x.clone());
    }

    if let Some(old_level_infos) = old_level_infos {
        info!("Computing changelist");
        update_changelist(&mut changelist, &mut new_level_infos, old_level_infos);

        info!("Saving changelist");
        persistence.save_changelist(&changelist)?;
    }

    info!("Saving level info");
    persistence.save_query_results(&new_level_infos)?;
    Ok(())
}

fn get_level_infos(steamworks: &Steamworks) -> impl Stream<Item = Result<LevelInfo, Error>> + '_ {
    stream::iter(get_official_levels(steamworks))
        .buffer_unordered(usize::max_value())
        .chain(
            get_workshop_levels(steamworks)
                .buffer_unordered(usize::max_value())
                .filter_map(|x| future::ready(x.transpose())),
        )
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

fn get_official_levels(
    steamworks: &Steamworks,
) -> impl Iterator<Item = impl Future<Output = Result<LevelInfo, Error>> + '_> + '_ {
    official_levels::iter().map(move |(level_name, mode)| {
        let leaderboard_name = distance_util::create_leaderboard_name_string(
            level_name, mode, None,
        )
        .unwrap_or_else(|| {
            panic!(
                "Couldn't create a leaderboard name string for the official level '{}'",
                level_name
            )
        });

        async move {
            let leaderboard_response = steamworks
                .get_leaderboard_range(leaderboard_name.clone(), 1, 2)
                .await?;

            Ok(LevelInfo {
                name: level_name.to_owned(),
                mode,
                leaderboard_name,
                workshop_response: None,
                leaderboard_response,
                timestamp: Utc::now(),
            })
        }
    })
}

fn get_workshop_levels(
    steamworks: &Steamworks,
) -> impl Stream<Item = impl Future<Output = Result<Option<LevelInfo>, Error>> + '_> + '_ {
    let workshop_levels = steamworks.get_all_workshop_sprint_challenge_stunt_levels();
    let level_infos = workshop_levels
        .map(|fut| {
            fut.map_ok(|workshop_response| {
                let x = [
                    LeaderboardGameMode::Sprint,
                    LeaderboardGameMode::Challenge,
                    LeaderboardGameMode::Stunt,
                ]
                .iter()
                .filter_map(move |mode| {
                    if workshop_response.tags.iter().any(|x| x == mode.name()) {
                        let leaderboard_name = distance_util::create_leaderboard_name_string(
                            remove_bytes_extension(&workshop_response.file_name),
                            *mode,
                            Some(workshop_response.steam_id_owner),
                        );
                        leaderboard_name.map(|leaderboard_name| {
                            Ok((workshop_response.clone(), *mode, leaderboard_name))
                        })
                    } else {
                        None
                    }
                });

                stream::iter(x)
            })
            .try_flatten_stream()
        })
        .flatten();

    level_infos.map(move |x| {
        future::ready(x)
            .and_then(move |(workshop_response, mode, leaderboard_name)| {
                async move {
                    Ok(steamworks
                        .get_leaderboard_range(leaderboard_name.clone(), 1, 2)
                        .await
                        .ok()
                        .map(|leaderboard_response| {
                            (
                                workshop_response,
                                mode,
                                leaderboard_name,
                                leaderboard_response,
                            )
                        }))
                }
            })
            .map_ok(|opt| {
                opt.map(
                    |(workshop_response, mode, leaderboard_name, leaderboard_response)| LevelInfo {
                        name: workshop_response.title.clone(),
                        mode,
                        leaderboard_name,
                        workshop_response: Some(workshop_response),
                        leaderboard_response,
                        timestamp: Utc::now(),
                    },
                )
            })
    })
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
                        Some(distance_util::format_score(previous_first_entry.score, *mode)),
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
            record_new: distance_util::format_score(first_entry.score, *mode),
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

fn is_score_better(score_1: i32, score_2: i32, game_mode: LeaderboardGameMode) -> bool {
    match game_mode {
        LeaderboardGameMode::Sprint | LeaderboardGameMode::Challenge => score_1 < score_2,
        LeaderboardGameMode::Stunt => score_1 > score_2,
    }
}

fn remove_bytes_extension(level: &str) -> &str {
    let pattern = ".bytes";
    assert!(level.ends_with(pattern));
    &level[0..(level.len() - pattern.len())]
}

#[allow(dead_code)]
fn dbg_type<T>(_: ()) -> T {
    panic!();
}

#[test]
fn test_remove_bytes_extension() {
    assert_eq!(remove_bytes_extension("some_level.bytes"), "some_level");
}
