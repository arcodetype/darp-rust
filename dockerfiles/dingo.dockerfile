FROM golang:alpine

RUN apk add nginx git \
    && go install github.com/air-verse/air@latest \
    && go install github.com/go-delve/delve/cmd/dlv@latest \
    && go install honnef.co/go/tools/cmd/staticcheck@latest

RUN git clone https://github.com/MadAppGang/dingo.git \
    && cd dingo \
    && go build -o dingo ./cmd/dingo \
    && cp dingo /bin/dingo \
    && cd .. \
    && rm -rf dingo

WORKDIR /app

# docker build -f dingo.dockerfile -t darp-dingo .
# podman build -f dingo.dockerfile -t darp-dingo .