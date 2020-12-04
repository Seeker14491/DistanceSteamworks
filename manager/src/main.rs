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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    color_backtrace::install();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let healthchecks_url = match env::var("HEALTHCHECKS_URL") {
        Ok(x) => Some(x),
        Err(e) => {
            match e {
                env::VarError::NotPresent => {
                    warn!("Environment variable HEALTHCHECKS_URL is not set")
                }
                env::VarError::NotUnicode(_) => {
                    warn!("Invalid HEALTHCHECKS_URL environment variable");
                }
            }

            None
        }
    };

    let result = run(healthchecks_url.as_deref()).await;

    if let Err(e) = result {
        if let Some(url) = healthchecks_url {
            healthchecks_send_fail_signal(&url, &format!("error: {}", e))
                .await
                .expect("Couldn't send healthchecks fail signal");
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

async fn run(healthchecks_url: Option<&str>) -> Result<(), Error> {
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
                    if let Some(url) = healthchecks_url {
                        healthchecks_send_ping(url).await.ok();
                    }

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

async fn healthchecks_send_ping(healthchecks_url: &str) -> Result<(), Error> {
    surf::get(healthchecks_url).send().await.map_err(|e| format_err!("Error sending fail signal: {}", e))?;

    Ok(())
}

async fn healthchecks_send_fail_signal(
    healthchecks_url: &str,
    error: impl Display,
) -> Result<(), Error> {
    surf::post(format!("{}/fail", healthchecks_url)).body(format!("[Distance Steamworks Manager] error: {}", error)).send().await
        .map_err(|e| format_err!("Error sending fail signal: {}", e))?;

    Ok(())
}
