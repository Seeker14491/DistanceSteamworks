#[macro_use]
extern crate log;

use failure::{Error, ResultExt};
use reqwest::Client;
use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs,
    process::{self, Child, Command, Stdio},
    thread,
    time::Duration,
};

const STEAM_RESTART_FREQUENCY_HOURS: i32 = 2;

fn main() {
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

    if let Err(e) = run() {
        print_error(e);

        if let Some(url) = discord_webhook_url {
            send_problem_notification(&url).expect("Couldn't send problem notification");
        }

        process::exit(-1);
    }
}

fn run() -> Result<(), Error> {
    'outer: loop {
        start_steam()?;
        sleep_secs(60);
        let mut proxy = start_proxy(env::args_os().skip(1))?;
        sleep_secs(5);
        const PROXY_POLL_FREQUENCY_SECS: i32 = 5;
        for i in 0..((60 / PROXY_POLL_FREQUENCY_SECS) * 60 * STEAM_RESTART_FREQUENCY_HOURS) {
            if let Ok(Some(exit_status)) = proxy.try_wait() {
                warn!("DistanceSteamworksProxy exited: {:?}", exit_status);

                if i == 0 {
                    break 'outer Err(failure::err_msg(
                        "Giving up on restarting DistanceSteamworksProxy",
                    ));
                }

                kill_steam()?;
                sleep_secs(5);
                continue 'outer;
            }
            sleep_secs(PROXY_POLL_FREQUENCY_SECS as u64);
        }

        proxy.kill().context("Couldn't kill proxy process")?;
        kill_steam()?;
        sleep_secs(5);
    }
}

fn print_error<E: Into<Error>>(e: E) {
    let e = e.into();
    error!("error: {}", e);
    for err in e.iter_causes() {
        error!(" caused by: {}", err);
    }
}

fn sleep_secs(secs: u64) {
    thread::sleep(Duration::from_secs(secs));
}

fn start_steam() -> Result<(), Error> {
    info!("Starting Steam");
    Command::new("steam")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env("DISPLAY", ":0")
        .spawn()
        .context("couldn't spawn Steam process")?;

    Ok(())
}

fn kill_steam() -> Result<(), Error> {
    info!("Killing Steam");
    Command::new("pkill")
        .arg("steam")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Couldn't spawn pkill process to kill Steam process")?
        .wait()
        .context("An error occured while waiting on pkill to finish killing Steam")?;

    Ok(())
}

fn start_proxy<I, S>(proxy_args: I) -> Result<Child, Error>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let dll_path = fs::canonicalize("./distance-steamworks-proxy")
        .unwrap()
        .into_os_string();
    let mut args = vec!["DistanceSteamworksProxy.dll".to_owned().into()];
    args.extend(proxy_args.into_iter().map(Into::into));

    info!("Starting DistanceSteamworksProxy");
    debug!("dotnet args: {:?}", args);
    let child = Command::new("dotnet")
        .args(args)
        .current_dir(dll_path)
        .spawn()
        .context("Error launching DistanceSteamworksProxy through dotnet")?;

    Ok(child)
}

fn send_problem_notification(discord_webhook_url: &str) -> Result<(), Error> {
    let mut params = HashMap::new();
    params.insert("content", "DistanceSteamworksProxy exited");

    Client::new()
        .post(discord_webhook_url)
        .json(&params)
        .send()
        .context("Error sending crash notification")?;

    Ok(())
}
