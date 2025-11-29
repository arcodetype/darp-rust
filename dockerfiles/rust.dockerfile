FROM rust:alpine

RUN apk add nginx build-base

WORKDIR /app

# docker build -f rust.dockerfile -t darp-rust .
# podman build -f rust.dockerfile -t darp-rust .