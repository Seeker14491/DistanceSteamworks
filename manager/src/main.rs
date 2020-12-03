#![warn(
    rust_2018_idioms,
    deprecated_in_future,
    macro_use_extern_crate,
    missing_debug_implementations,
    unused_qualifications
)]

use anyhow::{format_err, Context, Error};
use futures::pin_mut;
use log::{error, info, warn};
use std::{
    collections::HashMap,
    env,
    fmt::Display,
    process,
    process::{ExitStatus, Stdio},
    time::{Duration, Instant},
};
use tokio::{
    process::{Child, Command},
    time,
};

const UPDATE_PERIOD: Duration = Duration::from_secs(5 * 60);
const MAX_UPDATE_DURATION: Duration = Duration::from_secs(60 * 60);
const STEAM_RESTART_PERIOD: Duration = Duration::from_secs(3 * 3600);
const PROBLEM_NOTIFICATION_THRESHOLD: Duration = Duration::from_secs(4 * 3600);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    color_backtrace::install();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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

    let result = run(discord_webhook_url.as_deref()).await;

    if let Err(e) = result {
        if let Some(url) = discord_webhook_url {
            discord_send_problem_notification(&url, &format!("error: {}", e))
                .await
                .expect("Couldn't send problem notification");
        }

        print_error(e);

        process::exit(-1);
    }
}

fn print_error<E: Into<Error>>(e: E) {
    let e = e.into();
    error!("error: {}", e);
    while let Some(e) = e.source() {
        error!(" caused by: {}", e);
    }
}

async fn run(discord_webhook_url: Option<&str>) -> Result<(), Error> {
    let mut last_successful_update = Instant::now();
    let mut consecutive_update_failures = 0;

    loop {
        let mut steam = start_steam()?;
        sleep_secs(60).await;

        let steam_start_time = Instant::now();
        while steam_start_time.elapsed() < STEAM_RESTART_PERIOD {
            let update_start_time = Instant::now();
            let f = run_distance_log();
            pin_mut!(f);
            match time::timeout(MAX_UPDATE_DURATION, f).await {
                Ok(_) => {
                    last_successful_update = Instant::now();

                    time::sleep(
                        UPDATE_PERIOD
                            .checked_sub(update_start_time.elapsed())
                            .unwrap_or_else(Duration::default),
                    )
                    .await;

                    consecutive_update_failures = 0;
                }
                Err(_) => {
                    print_error(format_err!("distance-log ran for too long"));

                    if let Some(discord_webhook_url) = discord_webhook_url {
                        if last_successful_update.elapsed() > PROBLEM_NOTIFICATION_THRESHOLD {
                            discord_send_problem_notification(
                                discord_webhook_url,
                                "Time since last successful update has exceeded threshold",
                            )
                            .await?;
                        }
                    }

                    if consecutive_update_failures != 0 {
                        let i = consecutive_update_failures % 3;
                        if i == 0 {
                            break;
                        } else {
                            sleep_secs(5 * 60).await;
                        }
                    }

                    consecutive_update_failures += 1;
                }
            }
        }

        shutdown_steam().await?;
        steam.wait().await?;
    }
}

async fn sleep_secs(secs: u64) {
    time::sleep(Duration::from_secs(secs)).await;
}

fn start_steam() -> Result<Child, Error> {
    info!("Starting Steam");
    let child = Command::new("steam")
        .arg("-no-browser")
        .env("DISPLAY", ":0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Couldn't spawn Steam process")?;

    Ok(child)
}

async fn shutdown_steam() -> Result<ExitStatus, Error> {
    info!("Shutting down Steam");
    Command::new("steam")
        .arg("-shutdown")
        .status()
        .await
        .context("Error shutting down Steam")
}

async fn run_distance_log() -> Result<ExitStatus, Error> {
    info!("Starting distance-log");
    let mut child = Command::new("./distance-log")
        .spawn()
        .context("Couldn't spawn the distance-log process")?;

    Ok(child.wait().await?)
}

async fn discord_send_problem_notification(
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
        .await
        .context("Error sending crash notification")?;

    Ok(())
}
