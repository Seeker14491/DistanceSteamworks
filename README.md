# DistanceSteamworks

> Software for querying and processing Distance's Steamworks data.

This repo contains three pieces of software: `manager`, `distance-log`, and `distance-log-frontend`.

distance-log queries the Steam leaderboards and output a log of new world records in JSON. You can access this generated changelog here: https://seekr.pw/distance-log/changelist.json.

"manager" is a wrapper around distance-log that periodically runs it.

distance-log-frontend is a webapp that displays the changelog that distance-log outputs. You can see it running at https://seekr.pw/distance-log/.

## Building

### Prerequisites

- Rust, to build distance-log and "manager"
- Yarn and Parcel, to build distance-log-frontend
- Python, if you will make use of the "build.py" build script

The code is cross-platform and should build and run on Windows, Mac, and Linux.

A build script "build.py" is provided that builds everything and copies things into the right places, but it'll only work on Windows 10 with WSL installed, and it just builds for Linux x64. You will need Rust installed inside WSL. If your setup is different you will need to tweak this file, or just build what you need individually. This script builds everything into the `out\linux-x64` directory.

## Running

### distance-log

#### Prerequisites:

- `steam_appid.txt` next to the executable, copied from the `distance-log` directory.
- Depending on your platform, a Steam API `.dll`, `.so`, or `.dylib` file, also next to the executable. You can find these [here](https://github.com/Seeker14491/steamworks-rs/tree/master/steamworks-sys/steamworks_sdk/redistributable_bin).
- Steam, logged into an account that owns Distance

The executable takes no arguments, so run it like this:

```
./distance-log
```

The program will create or update `changelist.json`, which is the log of new world records, then exit. It only writes records obtained since it last ran, so the first time it runs it will not generate any entries. It also writes `query_results.json`, which is used in the creation of the changelist.

### manager

#### Prerequisites:

- The distance-log executable must be in the same directory as the manager executable
- All prerequisites of distance-log
- Make sure the manager and distance-log executables have execute permissions.

The manager application is a wrapper around distance-log. It runs continuously, executing the distance-log binary at a regular interval. It also starts and restarts Steam at a regular interval.

The manager application also has built-in support for sending a Discord message if something goes wrong, through a webhook. To set this up, see [this link](https://support.discordapp.com/hc/en-us/articles/228383668-Intro-to-Webhooks) to set up a webhook, and then set a `DISCORD_WEBHOOK_URL` environment variable with the value of the webhook URL before running the manager application.

It currently does not take any arguments; modify the consts in `src/main.rs` if you want to configure the intervals. Note that the manager starts Steam on its own; Steam should not be running already when you run the manager (Though you should have run Steam before and logged in so it won't prompt for your password again).

```
./manager
```
