# Command Reference

## Core Commands

### `darp install`

Sets up system integration: DNS resolver, nginx config, dnsmasq, and shell completions (bash/zsh/fish).

```sh
darp install
```

Run this again if you switch between Docker and Podman.

### `darp uninstall`

Removes system integration and stops darp containers.

```sh
darp uninstall
```

### `darp deploy`

Scans all domain folders, registers URLs, and restarts the reverse proxy. Run this after adding new project folders.

```sh
darp deploy
```

### `darp serve`

Starts a container running the configured `serve_command`. Your API must listen on port 8000.

```sh
darp serve                         # uses default environment and image
darp serve -e go                   # specify environment
darp serve my-image                # specify container image
darp serve -e go my-image          # specify both
darp serve --dry-run               # print the docker command without running it
```

### `darp shell`

Opens an interactive shell in a container.

```sh
darp shell                         # uses defaults
darp shell my-image                # specify image
darp shell -e go                   # specify environment
darp shell --dry-run               # print without running
```

### `darp urls`

Lists all registered URLs and their ports, grouped by service group. Services under named groups appear under a cyan group header; services in the default group (`.`) are listed without a header.

```sh
darp urls
```

### `darp doctor`

Checks your system health and darp configuration. Verifies:

- Darp root directory and core files (config.json, nginx.conf, dnsmasq config)
- Container engine installation and status
- DNS resolver configuration (/etc/resolver/test)
- Infrastructure containers (reverse proxy, dnsmasq)
- Shell completions
- Domain configurations and filesystem state
- Deploy artifacts (portmap, vhost config)
- Environment definitions

Run this when something isn't working to quickly identify what's misconfigured.

```sh
darp doctor
```

### `darp check-image`

Validates that a container image is compatible with darp. Spins up a quick diagnostic container and checks for:

- `sh` — required for darp to run commands inside the container
- `nginx` — enables in-container reverse proxy for `.test` domains
- Your configured `serve_command` binary (e.g. `air`, `php`, `npm`)
- Your configured `shell_command` binary

If run from inside a service directory, the image and commands are resolved automatically from your configuration. Otherwise, pass an image name directly.

```sh
darp check-image                   # resolve image from current directory context
darp check-image golang:1.25       # check a specific image
darp check-image -e go my-image    # check with a specific environment
```

## Configuration Commands

### `darp config show`

Shows the resolved configuration for your current directory after applying the full resolution chain (Service > Group > Domain > Environment) and all pre_config merges.

```sh
darp config show                   # uses default environment
darp config show -e staging        # show with a specific environment
```

### `darp config pull`

Runs `git pull` in each pre_config repo that has a `repo_location`.

```sh
darp config pull
```

### `darp config set`

Set scalar values on various config levels.

```sh
# Global
darp config set engine docker
darp config set podman-machine my-machine
darp config set urls-in-hosts true

# Environment level
darp config set env serve-command go 'air'
darp config set env shell-command go 'bash'
darp config set env image-repository go 'registry.example.com/go'
darp config set env default-container-image go '1.25'
darp config set env platform go 'linux/amd64'

# Domain level (use -l to create the domain if it doesn't exist)
darp config set dom default-environment my-domain go
darp config set dom serve-command -l ~/projects my-domain 'npm start'
darp config set dom image-repository my-domain 'registry.example.com/node'
# Also: shell-command, platform, default-container-image

# Group level
darp config set grp default-environment my-domain laravel 'lara:13'
darp config set grp serve-command my-domain go 'air'
# Also: shell-command, image-repository, platform, default-container-image

# Service level (use -g for non-default group)
darp config set svc serve-command my-domain my-service 'npm start'
darp config set svc serve-command -g laravel my-domain admin 'php artisan serve'
# Also: shell-command, image-repository, platform, default-container-image
```

### `darp config add`

Add entries to collections or create new items.

```sh
# Pre-config
darp config add pre-config '{home}/team/config.json' -r '{home}/team-repo'

# Group (create an empty group)
darp config add grp group my-domain laravel

# Port mappings (use -l to create the domain if it doesn't exist)
darp config add env portmap go 2345 2345
darp config add dom portmap my-domain 8080 8080
darp config add dom portmap -l ~/projects my-domain 8080 8080
darp config add grp portmap my-domain laravel 9000 9000
darp config add svc portmap my-domain my-service 3000 3000
darp config add svc portmap -g laravel my-domain admin 8082 8082

# Variables
darp config add env variable go NODE_ENV development
darp config add svc variable my-domain my-service API_KEY abc123

# Volumes
darp config add env volume go /root/.ssh '{home}/.ssh'
darp config add dom volume my-domain /root/.gitconfig '{home}/.gitconfig'
```

### `darp config rm`

Remove entries.

```sh
# Pre-config
darp config rm pre-config '{home}/team/config.json'

# Domain
darp config rm domain my-projects

# Group
darp config rm group my-domain laravel

# Service
darp config rm service my-domain laravel admin

# Scalar settings
darp config rm env serve-command go
darp config rm dom default-environment my-domain
darp config rm grp default-environment my-domain laravel
darp config rm svc serve-command my-domain my-service
darp config rm svc serve-command -g laravel my-domain admin

# Collection entries
darp config rm env portmap go 2345
darp config rm svc variable my-domain my-service API_KEY
darp config rm env volume go /root/.ssh '{home}/.ssh'

# Also: podman-machine
darp config rm podman-machine
```
