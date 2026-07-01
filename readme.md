# darp

[![CI](https://github.com/arcodetype/darp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/arcodetype/darp-rust/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/darp.svg)](https://crates.io/crates/darp)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/darp.svg)](LICENSE-MIT)

**darp** (<b>d</b>irectories <b>a</b>uto-<b>r</b>everse <b>p</b>roxied) turns local project folders into `.test` domains automatically. Point darp at a folder, and every subdirectory gets its own URL (e.g. `hello-world.projects.test`) backed by Docker/Podman, nginx, and dnsmasq.

No YAML files. No port juggling. Just `cd` into a project and run `darp serve`.

darp also assigns each service a stable debug port, so editor/debugger configs
can track projects without fighting your local dev ports.

## Install

### From crates.io

```sh
cargo install darp
```

### Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/arcodetype/darp-rust/releases), extract, and place `darp` somewhere in your `$PATH`:

```sh
# macOS (Apple Silicon)
curl -sL https://github.com/arcodetype/darp-rust/releases/latest/download/darp-aarch64-apple-darwin.tar.gz | tar xz
sudo mv darp-*/darp /usr/local/bin/

# macOS (Intel)
curl -sL https://github.com/arcodetype/darp-rust/releases/latest/download/darp-x86_64-apple-darwin.tar.gz | tar xz
sudo mv darp-*/darp /usr/local/bin/

# Linux (x86_64)
curl -sL https://github.com/arcodetype/darp-rust/releases/latest/download/darp-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv darp-*/darp /usr/local/bin/
```

### From source

```sh
git clone https://github.com/arcodetype/darp-rust.git
cd darp-rust
cargo build --release
sudo cp ./target/release/darp /usr/local/bin/darp
```

## Quick Start

This gets you from zero to a running Go API in under five minutes.

### 1. Set up a domain

```sh
mkdir -p ~/projects/hello-world

darp config set engine docker
darp install
darp config set dom serve-command -l ~/projects projects 'air'
darp deploy
```

### 2. Build a container image

Your image needs **nginx** installed so darp can reverse-proxy to it. See [dockerfiles/](./dockerfiles/) for starters.

```sh
cd ~/projects/hello-world
cat > Dockerfile <<'EOF'
FROM golang:1.25-alpine3.22
RUN apk add nginx && go install github.com/air-verse/air@latest
WORKDIR /app
EOF
docker build -t darp-go .
```

### 3. Shell in and create a project

```sh
darp shell darp-go
```

Inside the container:

```sh
echo 'package main
import "net/http"
func main() {
    http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
        w.Write([]byte("We are darping!\n"))
    })
    http.ListenAndServe(":8000", nil)
}' > main.go
go mod init hello && go mod tidy && air init
exit
```

### 4. Serve it

```sh
darp config set env serve-command go 'air'
darp config set env default-container-image go darp-go
darp config set dom default-environment projects go
darp serve
```

From another terminal:

```sh
curl http://hello-world.projects.test
# We are darping!
```

That's it. Any edits you make in `~/projects/hello-world/` are live-reloaded.

> **Tip:** Run `darp deploy` after adding new project folders so darp registers URLs for them.
> `darp deploy` also assigns each service a stable debug port. Run `darp urls`
> to see both the app URL and its debug port.

## How It Works

```
~/projects/                     <- domain "projects"
  hello-world/                  <- hello-world.projects.test
  billing-api/                  <- billing-api.projects.test
```

When you run `darp serve` or `darp shell` from a project folder, darp:

1. Detects which domain and service you're in based on your current directory
2. Resolves settings from Service > Group > Domain > Environment (most specific wins)
3. Reads the deploy-generated port map, including the reverse-proxy port and per-service debug port
4. Builds a container command with the right ports, volumes, env vars, and token-expanded commands
5. Reverse-proxies port 8000 through nginx to a `.test` URL

## Key Concepts

| Concept | What it is |
|---|---|
| **Domain** | A folder containing projects. Each subdirectory becomes a `.test` URL. |
| **Group** | An optional subfolder within a domain for organizing by tech stack (e.g. `go/`, `laravel/`). The `.` group means "directly in the domain folder." Groups don't affect URLs. |
| **Service** | A project folder. Settings here override everything else. |
| **Environment** | A reusable profile (image, volumes, commands) shared across services. |
| **Pre-config** | A parent config file (e.g. from a team repo) that gets merged into yours. |

Settings cascade: **Service > Group > Domain > Environment**. The most specific level wins.

## Debug Ports

`darp deploy` assigns every service a stable, unique debug port and writes it to
the local port map. By default these start at **13000**, avoiding crowded dev
ports like 9000, 9003, 9090, 9092, 9200, and 9229. Existing valid assignments are
reused on future deploys, so `.vscode/launch.json` or other debugger settings
do not churn.

See the assigned ports with:

```sh
darp urls
# http://hello-world.projects.test (50100)  [debug: 13000]
```

You can reference runtime tokens in `serve_command`, `variables`, and
`host_portmappings`:

| Token | Expands to |
|---|---|
| `{debug_port}` | The service's assigned debug port |
| `{proxy_port}` | The service's reverse-proxy port |
| `{service}` | The current service name |
| `{group}` | The current group name |
| `{domain}` | The current domain name |

For example:

```sh
darp config set env serve-command go 'dlv debug --headless --listen=:{debug_port}'
```

Or publish a debugger port and customize the assignment range directly in
`~/.darp/config.json`:

```json
{
  "debug_port_base": 14000,
  "environments": {
    "go": {
      "serve_command": "dlv debug --headless --listen=:{debug_port}",
      "host_portmappings": {
        "{debug_port}": "{debug_port}"
      }
    }
  }
}
```

## Documentation

- [Configuration Guide](docs/configuration.md) -- settings, resolution, and config.json structure
- [Team Collaboration](docs/team-collaboration.md) -- sharing configs with pre_config and `darp config pull`
- [Command Reference](docs/commands.md) -- every darp command with examples
- [Groups](docs/groups.md) -- organizing multi-stack projects under one domain

## Common Commands

```sh
darp install                    # Set up system integration (nginx, dnsmasq, completions)
darp deploy                     # Register URLs and assign stable debug ports
darp serve                      # Run the serve_command in a container
darp shell                      # Open an interactive shell in a container
darp urls                       # List all registered URLs and debug ports
darp config show                # Show resolved settings for current directory
darp config show -e go          # Show what settings would apply with a specific environment
darp config pull                # Git pull all pre_config repos
```

## Requirements

- Rust toolchain (for building)
- Docker or Podman
- macOS or Linux

## Notes

- Your API must listen on **port 8000** inside the container.
- Container images need **nginx** installed for the reverse proxy to work.
- If `.test` domains aren't resolving, try `darp config set urls-in-hosts true` and re-run `darp deploy`.
- Run `darp install` again if you switch between Docker and Podman.
