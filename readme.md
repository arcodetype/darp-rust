# darp

`darp` (<span style="color:#d670d6">d</span>irectories <span style="color:#d670d6">a</span>uto-r<span style="color:#d670d6">r</span>everse <span style="color:#d670d6">p</span>roxied) is a CLI that automatically reverse-proxies local project folders into nice `.test` domains (e.g. `hello-world.projects.test`) using Docker or Podman, nginx, and dnsmasq.

This is the Rust Port of this application.

## Build

### Mac

```
carbo build --release
sudo cp ./target/release/darp /usr/local/bin/darp 
```

## Tutorial

This tutorial takes you through running a simple Go API with darp.

> Note: The examples use Docker, but everything works with Podman by substituting `docker` → `podman`.

> Note: If you switch between Docker and Podman, you must run `darp deploy` again so the reverse-proxy configuration can be refreshed.

### Step One: Set Up a Projects Domain

Initialize darp and configure a folder to be reverse-proxied:

```sh
darp install
darp config set engine docker
darp config add domain ~/projects
darp mkdir ~/projects/hello-world
darp deploy
darp urls
```

> Note: After creating any new project inside `~/projects`, run `darp deploy` so darp can register it.

### Step Two: Create Your First "darp compatible" Image

#### Requirements

A darp-compatible image must:
- Be based on Alpine
- Have nginx installed

```sh
cd ~/projects/hello-world
echo "FROM golang:1.25-alpine3.22\n\nRUN apk add nginx && go install github.com/air-verse/air@latest\n\nWORKDIR /app" > Dockerfile
docker build -t darp-go .
```

Resulting Dockerfile:

```dockerfile
FROM golang:1.25-alpine3.22

RUN apk add nginx && go install github.com/air-verse/air@latest

WORKDIR /app
```

### Step Three: Shell Into Your Project (Changes Persist Locally)

Start a darp shell session using your image:
```sh
darp shell darp-go
```

Inside the container:
```sh
echo 'package main;import("net/http");func main(){http.HandleFunc("/",func(w http.ResponseWriter,r *http.Request){w.Write([]byte("We are darping! Edit Me\n"))});http.ListenAndServe(":8000",nil)}' > main.go

go mod init arcodetype.test
go mod tidy
air init
```


### Step Four: Serve Your Project

Different tech stacks will use different serve commands. For this example, we'll use Air:

> Note: Updating values via `darp config add`, `darp config set`, and `darp config rm` automatically updates `~/.darp/config.json`. Advanced users may edit this file manually, but using darp commands is recommended.

#### Requirements for `darp serve`

- Your API must listen on `port 8000` inside the container.

Configure and run:
```sh
darp set env serve_command go 'air'
darp serve -e go darp-go
```

Now test the endpoint from another terminal:
```sh
curl http://hello-world.projects.test
```

> Note: If your endpoint is unresponsive, try `darp config set urls_in_hosts True`. It's possible that the `darp-masq` image is not resolving correctly. After turning on, a computer password will be required on future runs of the `darp deploy` command.

Try editing files inside the hello-world project directory in your editor:

- Docker: Air automatically reloads on file changes.
- Podman: You must edit .air.toml and change
poll = false → poll = true so changes are detected.

