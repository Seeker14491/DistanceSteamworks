# This script builds the whole project from a Windows 10 machine to run on Linux x64. WSL needs to be installed.

import shutil
import subprocess

# Set this based on the url the webapp will be hosted at
publicUrl = "/distance-log"

out = "out/linux-x64/"

shutil.rmtree("out", ignore_errors=True)

subprocess.run(["bash", "--login", "-c", "cargo test --all-features && cargo build --release"], cwd="distance-log", check=True)
subprocess.run(["bash", "--login", "-c", "cargo build --release"], cwd="manager", check=True)

subprocess.run(["yarn"], cwd="distance-log-frontend", shell=True, check=True)
subprocess.run(["parcel", "build", "--public-url", publicUrl, "www/index.html"], cwd="distance-log-frontend", shell=True, check=True)

shutil.copytree("distance-log-frontend/dist", out + "site")

shutil.copy("distance-log/target/release/distance-log", out)
shutil.copy("distance-log/steam_appid.txt", out)

shutil.copy("libsteam_api.so", out)

shutil.copy("manager/target/release/manager", out)
