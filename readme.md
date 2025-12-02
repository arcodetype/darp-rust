# darp

`darp` (<span style="color:#d670d6">d</span>irectories <span style="color:#d670d6">a</span>uto-<span style="color:#d670d6">r</span>everse <span style="color:#d670d6">p</span>roxied) is a CLI that automatically reverse-proxies local project folders into nice `.test` domains (e.g. `hello-world.projects.test`) using Docker or Podman, nginx, and dnsmasq.

## Build

### Mac

```
cargo build --release
sudo rm /usr/local/bin/darp
sudo cp ./target/release/darp /usr/local/bin/darp 
```

## Tutorial

This tutorial takes you through running a simple Go API with darp.

> Note: The examples use Docker, but everything works with Podman by substituting `docker` → `podman`.

### Step One: Set Up a Projects Domain

Initialize darp and configure a folder to be reverse-proxied:

```sh
mkdir ~/projects/
mkdir ~/projects/hello-world/
darp config set engine docker
darp install
darp config add domain ~/projects
darp deploy
darp urls
```

> Note: If you switch between Docker and Podman, run `darp install` again to ensure all of the necessary configuration is installed.

> Note: After creating any new projects inside one of your domains, run `darp deploy` so darp can register a URL for it.

### Step Two: Create a linux image with nginx installed so that it's "`darp serve` compatible"

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

> Note: You can find compatibile starter [dockerfiles](./dockerfiles/) in the `./dockerfiles/` directory of this project.

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
darp config set env serve_command go 'air'
darp serve -e go darp-go
```

Now test the endpoint from another terminal:
```sh
curl http://hello-world.projects.test
```

> Note: If your endpoint is unresponsive, try `darp config set urls_in_hosts True`. It's possible that the `darp-masq` image is not resolving correctly. After turning on, a computer password will be required on future runs of the `darp deploy` command.

Try editing files inside the hello-world project directory in your editor:

- Docker: Air automatically reloads on file changes.
- Podman: You must edit `.air.toml` with
`poll = false` → `poll = true` so changes are detected.

### Step Five: Add default settings

While you have the flexibility to provide an image or an environment, depending on the complexity of your setup, you may not want to have to keep track of that from day-to-day.

```sh
darp config set env default-container-image darp-go
darp config set dom default-environment projects go
```

Now, you can simply run `darp shell` or `darp serve` when in the directory of your choosing and darp will choose the correct settings.

> Note: When default settings are used and there's a conflict, `command line arguments` override `services` which override `environments`.

## Example config.json

The following is a robust example showing the convenience afforded by setting up your `~/darp/config.json`. Some of the settings are configured just for demonstrative purposes.

```sh
> tree Projects

Projects
├── admin
├── analytics
├── billing-service
├── dashboard
├── gateway
└── user-service
```

```sh
> darp urls

arco
    http://admin.arco.test (50100)
    http://analytics.arco.test (50101)
    http://billing-service.arco.test (50102)
    http://dashboard.arco.test (50103)
    http://gateway.arco.test (50104)
    http://user-service.arco.test (50105)
```

The following darp commands simplify down to this docker equivalent based on the `config.json`.

<table>
<tr>
<td>darp command</td><td>equivalent docker command</td>
</tr>
<tr>
<td>

```sh
# cd ~/Projects/admin
darp shell
```

</td>
<td>

```sh
docker run --rm -it -p 50100:8000 -p 8082:8082 \
    -v /Users/arco/.darp/hosts_container:/etc/hosts \
    -v /Users/arco/.darp/vhost_container.conf:/etc/nginx/http.d/vhost_container.conf \
    -v /Users/arco/Projects/admin:/app \
    -v /Users/arco/Projects/admin/deploy/conf/php.ini:/etc/php83/php.ini \
    -v /Users/arco/.composer/auth.json:/root/.composer/auth.json \
    -v /Users/arco/.gitconfig:/root/.gitconfig \
    -v /Users/arco/.ssh/:/root/.ssh/ \
    git.arco.com:4567/php/master:php:83fpm bash
```

</td>
</tr>
<tr>
<td>

```sh
# cd ~/Projects/admin
darp serve
```

</td>
<td>

```sh
docker run --rm -it -p 50100:8000 -p 8082:8082 \
    -v /Users/arco/.darp/hosts_container:/etc/hosts \
    -v /Users/arco/.darp/vhost_container.conf:/etc/nginx/http.d/vhost_container.conf \
    -v /Users/arco/Projects/admin:/app \
    -v /Users/arco/Projects/admin/deploy/conf/php.ini:/etc/php83/php.ini \
    -v /Users/arco/.composer/auth.json:/root/.composer/auth.json \
    -v /Users/arco/.gitconfig:/root/.gitconfig \
    -v /Users/arco/.ssh/:/root/.ssh/ \
    git.arco.com:4567/php/master:php:83fpm "/usr/bin/php artisan serve --host 0.0.0.0 & /usr/bin/npm run hot"
```

</td>
</tr>
</tr>
<tr>
<td>

```sh
# cd ~/Projects/billing-service
darp shell
```

</td>
<td>

```sh
docker run --rm -it -p 50102:8000 \
    --platform linux/amd64 \
    -v /Users/arco/.darp/hosts_container:/etc/hosts \
    -v /Users/arco/.darp/vhost_container.conf:/etc/nginx/http.d/vhost_container.conf \
    -v /Users/arco/Projects/admin:/app \
    -v /Users/arco/Projects/admin/deploy/conf/php.ini:/etc/php83/php.ini \
    -v /Users/arco/.composer/auth.json:/root/.composer/auth.json \
    -v /Users/arco/.gitconfig:/root/.gitconfig \
    -v /Users/arco/.ssh/:/root/.ssh/ \
    git.arco.com:4567/php/master:php:83 bash
```

</td>
</tr>
<tr>
<td>

```sh
# cd ~/Projects/billing-service
darp serve
```

</td>
<td>

```sh
docker run --rm -it -p 50102:8000 -p 8082:8082 \
    --platform linux/amd64 \
    -v /Users/arco/.darp/hosts_container:/etc/hosts \
    -v /Users/arco/.darp/vhost_container.conf:/etc/nginx/http.d/vhost_container.conf \
    -v /Users/arco/Projects/admin:/app \
    -v /Users/arco/Projects/admin/deploy/conf/php.ini:/etc/php83/php.ini \
    -v /Users/arco/.composer/auth.json:/root/.composer/auth.json \
    -v /Users/arco/.gitconfig:/root/.gitconfig \
    -v /Users/arco/.ssh/:/root/.ssh/ \
    git.arco.com:4567/php/master:php:83 "php artisan serve --host 0.0.0.0"
```

</td>
</tr>
</table>

And finally, the `config.json`

```json
{
  "engine": "docker",
  "domains": {
    "/Users/arco/Projects": {
      "name": "arco",
      "services": {
        "admin": {
          "serve_command": "/usr/bin/php artisan serve --host 0.0.0.0 & /usr/bin/npm run hot",
          "host_portmappings": {
            "8082": "8082"
          },
          "default_container_image": "php:83fpm"
        },
        "dashboard": {
          "serve_command": "/usr/bin/php artisan serve --host 0.0.0.0 & /usr/bin/npm run hot",
          "host_portmappings": {
            "8081": "8081"
          },
          "default_container_image": "php:83fpm"
        }
      },
      "default_environment": "lara:11"
    }
  },
  "environments": {
    "lara:11": {
      "volumes": [
        {
          "container": "/root/.composer/auth.json",
          "host": "{home}/.composer/auth.json"
        },
        {
          "container": "/etc/php83/php.ini",
          "host": "{pwd}/deploy/conf/php.ini"
        },
        {
          "container": "/root/.gitconfig",
          "host": "{home}/.gitconfig"
        },
        {
          "container": "/root/.ssh",
          "host": "{home}/.ssh"
        }
      ],
      "serve_command": "php artisan serve --host 0.0.0.0",
      "shell_command": "bash",
      "image_repository": "git.arco.com:4567/php/master",
      "default_container_image": "php:83",
      "platform": "linux/amd64"
    },
    "laravue:11": {
      "volumes": [
        {
          "container": "/root/.composer/auth.json",
          "host": "{home}/.composer/auth.json"
        },
        {
          "container": "/etc/php83/php.ini",
          "host": "{pwd}/deploy/conf/php.ini"
        },
        {
          "container": "/root/.gitconfig",
          "host": "{home}/.gitconfig"
        },
        {
          "container": "/root/.ssh",
          "host": "{home}/.ssh"
        }
      ],
      "image_repository": "git.arco.com:4567/php/master",
    }
  }
}
```
