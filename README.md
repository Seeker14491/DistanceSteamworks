# DistanceSteamworks

> Software for querying and processing Distance's Steamworks data.

This repo currently contains three pieces of software: `DistanceSteamworksProxy`, `distance-log`, and `manager`.

DistanceSteamworksProxy communicates with Steam servers through the [Steamworks API](https://partner.steamgames.com/doc/sdk/api), and exposes its own API via [JSON-RPC](https://www.jsonrpc.org/specification). It is implemented in C#, and makes use of the [Steamworks.NET](https://steamworks.github.io/) library, which is also used by Distance.

"manager" is a wrapper around DistanceSteamworksProxy that periodically restarts Steam and DistanceSteamworksProxy to keep things running smoothly. It is implemented in Rust.

distance-log consumes the API exposed by DistanceSteamworksProxy, and uses it to query the Steam leaderboards and render an HTML page showing a log of new world records. It is implemented in Rust. You can see its output at https://seekr.pw/distance-log/.

## Building

### Prerequisites

- .NET Core SDK to build DistanceSteamworksProxy
- Rust nightly to build distance-log and "manager"
- Python, if you will make use of the "build.py" build script


Most code is cross-platform and should build and run on Windows, Mac, and Linux. An exception is the "manager" program, which is only written for running on Linux.

A build script "build.py" is provided that builds everything and copies things into the right places, but it'll only work on Windows 10 with WSL installed, and it just builds for Linux x64. If your setup is different you will need to tweak this file. This script builds everything into the `out\linux-x64` directory.

## Running

### DistanceSteamworksProxy

#### Prerequisites:

- .NET Core Runtime
- Steam, logged into an account that owns Distance

Instead of running it directly, you should run it through the manager application, substituting `8000` to whatever port number you want it to expose the JSON-RPC api on:

```
./manager 8000
```

The manager application has built-in support for sending a Discord message if something goes wrong, through a webhook. To set this up, see [this link](https://support.discordapp.com/hc/en-us/articles/228383668-Intro-to-Webhooks) to set up a webhook, and then set a `DISCORD_WEBHOOK_URL` environment variable with the value of the webhook URL before running the manager application.

If you do want to run the bare DistanceSteamworksProxy, you can run it like this from inside the folder with the build `DistanceSteamworksProxy.dll`:

```
dotnet DistanceSteamworksProxy.dll 8000
```

### distance-log

#### Prerequisites:

- a running DistanceSteamworksProxy process

You might run it like this:

```
./distance-log -p 8000 -d "10 min"
```

To see available flags and options, you can pass the `-h` flag:

```
./distance-log -h
```

Once run, the program will update `site/index.html` regularly.

## Contributing

Suggestions and PRs are welcome.