# DistanceSteamworks

> Software for querying and processing Distance's Steamworks data.

This repo contains three pieces of software: `manager`, `distance-log`, and `distance-log-frontend`.

distance-log queries the Steam leaderboards and output a log of new world records in JSON. You can access this generated changelog here: https://seekr.pw/distance-log/changelist.json.

"manager" is a wrapper around distance-log that periodically runs it.

distance-log-frontend is a webapp that displays the changelog that distance-log outputs. You can see it running at https://seekr.pw/distance-log/.

## Building

A Dockerfile is provided to easily build all sub-projects for Linux. You just need Docker installed, then run the `build_with_docker.sh` script. The build output will be placed in the `out` directory.

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

The manager application also has built-in support for pinging [healthchecks.io](https://healthchecks.io/) for monitoring. To use this functionality, set the `HEALTHCHECKS_URL` environment variable to your ping endpoint, which should be of the form `https://hc-ping.com/{uuid}`, before running the manager application.

The manager does not take any arguments; modify the consts in `src/main.rs` if you want to configure the intervals. Note that the manager starts Steam on its own; Steam should not be running already when you run the manager (Though you should have run Steam before and logged in so it won't prompt for your password again).

```
./manager
```
