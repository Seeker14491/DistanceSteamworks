#!/bin/bash

docker build -t distance-steamworks-builder .
docker run --rm -v "$(pwd)":/src distance-steamworks-builder
