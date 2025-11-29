FROM golang:alpine

RUN apk add nginx \
    && go install github.com/air-verse/air@latest \
    && go install github.com/go-delve/delve/cmd/dlv@latest \
    && go install honnef.co/go/tools/cmd/staticcheck@latest

WORKDIR /app

# docker build -f rust.dockerfile -t darp-go .
# podman build -f rust.dockerfile -t darp-go .