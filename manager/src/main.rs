#![feature(async_await)]
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

use async_timer::TimerProvider;
use failure::{Error, ResultExt};
use futures::{channel::oneshot, prelude::*};
use log::{error, info, warn};
use pin_utils::pin_mut;
use std::{
    collections::HashMap,
    env,
    fmt::Display,
    process::{self, Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

const UPDATE_PERIOD: Duration = Duration::from_secs(5 * 60);
const MAX_UPDATE_DURATION: Duration = Duration::from_secs(30 * 60);
const STEAM_RESTART_PERIOD: Duration = Duration::from_secs(2 * 3600);
const PROBLEM_NOTIFICATION_THRESHOLD: Duration = Duration::from_secs(4 * 3600);

#[runtime::main]
async fn main() {
    color_backtrace::install();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = cli_args::get();

    let discord_webhook_url = match env::var("DISCORD_WEBHOOK_URL") {
        Ok(x) => Some(x),
        Err(e) => {
            match e {
                env::VarError::NotPresent => {
                    warn!("Environment variable DISCORD_WEBHOOK_URL is not set")
                }
                env::VarError::NotUnicode(_) => {
                    warn!("Invalid DISCORD_WEBHOOK_URL");
                }
            }

            None
        }
    };

    if let Err(e) = run(args, discord_webhook_url.as_ref().map(String::as_str)).await {
        if let Some(url) = discord_webhook_url {
            discord_send_problem_notification(&url, &format!("error: {}", e))
                .expect("Couldn't send problem notification");
        }

        print_error(e);

        process::exit(-1);
    }
}

fn print_error<E: Into<Error>>(e: E) {
    let e = e.into();
    error!("error: {}", e);
    for err in e.iter_causes() {
        error!(" caused by: {}", err);
    }
}

async fn run(_args: cli_args::Opt, discord_webhook_url: Option<&str>) -> Result<(), Error> {
    let timer = TimerProvider::new();
    let mut last_successful_update = Instant::now();
    let mut consecutive_update_failures = 0;

    loop {
        let mut steam = start_steam()?;
        sleep_secs(&timer, 60).await?;

        let steam_start_time = Instant::now();
        while steam_start_time.elapsed() < STEAM_RESTART_PERIOD {
            let update_start_time = Instant::now();
            let f = run_distance_log();
            pin_mut!(f);
            match timer.timeout_future_from_now(f, MAX_UPDATE_DURATION).await {
                Ok(_) => {
                    last_successful_update = Instant::now();

                    timer
                        .delay_from_now(
                            UPDATE_PERIOD
                                .checked_sub(update_start_time.elapsed())
                                .unwrap_or(Duration::default()),
                        )
                        .await?;

                    consecutive_update_failures = 0;
                }
                Err(timeout_error) => {
                    if timeout_error.is_elapsed() {
                        print_error(failure::err_msg("distance-log ran for too long"));
                    } else {
                        print_error(timeout_error.into_inner().unwrap())
                    }

                    if let Some(discord_webhook_url) = discord_webhook_url {
                        if last_successful_update.elapsed() > PROBLEM_NOTIFICATION_THRESHOLD {
                            discord_send_problem_notification(
                                discord_webhook_url,
                                "Time since last successful update has exceeded threshold",
                            )?;
                        }
                    }

                    if consecutive_update_failures != 0 {
                        let i = consecutive_update_failures % 3;
                        if i == 0 {
                            break;
                        } else {
                            sleep_secs(&timer, 5 * 60).await?;
                        }
                    }

                    consecutive_update_failures += 1;
                }
            }
        }

        steam.kill()?;
    }
}

async fn sleep_secs(timer_provider: &TimerProvider, secs: u64) -> Result<(), async_timer::Error> {
    timer_provider
        .delay_from_now(Duration::from_secs(secs))
        .await
}

fn start_steam() -> Result<Child, Error> {
    info!("Starting Steam");
    let child = Command::new("steam")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env("DISPLAY", ":0")
        .spawn()
        .context("Couldn't spawn Steam process")?;

    Ok(child)
}

async fn run_distance_log() -> Result<ExitStatus, Error> {
    info!("Starting distance-log");
    let (tx, rx) = oneshot::channel::<Result<ExitStatus, Error>>();
    thread::spawn(|| {
        let mut child = match Command::new("./distance-log")
            .spawn()
            .context("Couldn't spawn the distance-log process")
        {
            Ok(x) => x,
            Err(e) => {
                tx.send(Err(e.into())).ok();
                return;
            }
        };

        loop {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    tx.send(Ok(exit_status)).ok();
                    break;
                }
                Ok(None) => {}
                Err(e) => {
                    tx.send(Err(e.into())).ok();
                    break;
                }
            }
        }
    });

    rx.map(|res| res.unwrap()).await
}

fn discord_send_problem_notification(
    discord_webhook_url: &str,
    error: impl Display,
) -> Result<(), Error> {
    let mut params = HashMap::new();
    params.insert(
        "content",
        format!("[Distance Steamworks Manager] error: {}", error),
    );

    reqwest::Client::new()
        .post(discord_webhook_url)
        .json(&params)
        .send()
        .context("Error sending crash notification")?;

    Ok(())
}
