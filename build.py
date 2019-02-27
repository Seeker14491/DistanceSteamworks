# This script builds the whole project from a Windows 10 machine to run on Linux x64. WSL needs to be installed.

import shutil
import subprocess

# Set this based on the url the webapp will be hosted at
publicUrl = "/distance-log"

out = "out/linux-x64/"
proxy = out + "distance-steamworks-proxy/"

shutil.rmtree("out", ignore_errors=True)

subprocess.run(["dotnet", "publish", "-c", "Release", "-f", "netcoreapp2.2"], cwd="DistanceSteamworksProxy"
                                                                                  "/DistanceSteamworksProxy",
               check=True)
subprocess.run(["bash", "--login", "-c", "cargo test && cargo build --release"], cwd="distance-log", check=True)
subprocess.run(["bash", "--login", "-c", "cargo build --release"], cwd="manager", check=True)

subprocess.run(["yarn"], cwd="distance-log-frontend", shell=True, check=True)
subprocess.run(["parcel", "build", "--public-url", publicUrl, "www/index.html"], cwd="distance-log-frontend", shell=True, check=True)

shutil.copytree("DistanceSteamworksProxy/DistanceSteamworksProxy/bin/Release/netcoreapp2.1/publish", proxy)
shutil.copy("DistanceSteamworksProxy/Steamworks.NET/OSX-Linux-x64/Steamworks.NET.dll", proxy)
shutil.copy("DistanceSteamworksProxy/Steamworks.NET/OSX-Linux-x64/libsteam_api.so", proxy)
shutil.copy("DistanceSteamworksProxy/DistanceSteamworksProxy/steam_appid.txt", proxy)

shutil.copy("distance-log/target/release/distance-log", out)
shutil.copy("distance-log/official_levels.json", out)

shutil.copy("manager/target/release/manager", out)

shutil.copytree("distance-log-frontend/dist", out + "site")
