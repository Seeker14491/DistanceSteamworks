#!/bin/bash

cd distance-log &&\
cargo test && cargo build --release &&\

cd ../distance-log-frontend &&\
yarn parcel build --public-url /distance-log www/index.html &&\

cd .. &&\
rm -rf out/ &&\
mkdir -p out/ &&\
cp -r distance-log-frontend/dist out/site/ &&\
cp distance-log/target/release/distance-log out/ &&\
cp distance-log/steam_appid.txt out/ &&\
cp libsteam_api.so out/ &&\
cp manager/target/release/manager out/
