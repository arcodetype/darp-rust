# Configuration Guide

darp stores its configuration in `~/.darp/config.json`. You can edit this file directly or use `darp config` commands.

## Settings Resolution

When you run `darp serve`, `darp shell` or `darp exec`, settings are resolved from most specific to least specific:

```
Service > Group > Domain > Environment
```

The first level that defines a setting wins. For example, if a service defines `serve_command`, the group/domain/environment values are ignored.

**For scalar settings** (serve_command, shell_command, image_repository, platform, default_container_image): the most specific value wins.

**For collection settings** (volumes, host_portmappings, variables): the most specific level that defines the collection wins entirely. Collections are not merged across levels.

## Environment Resolution

The environment is determined by:

1. The `-e` flag on the command line
2. The service's `default_environment`
3. The group's `default_environment`
4. The domain's `default_environment`

## Config Structure

```json
{
  "pre_config": [
    {
      "location": "{home}/team-repo/config.json",
      "repo_location": "{home}/team-repo"
    }
  ],
  "engine": "docker",
  "podman_machine": null,
  "urls_in_hosts": true,
  "domains": {
    "my-projects": {
      "location": "{home}/projects",
      "default_environment": "go",
      "groups": {
        ".": {},
        "laravel": {
          "default_environment": "lara:13",
          "services": {
            "admin": {
              "serve_command": "php artisan serve --host 0.0.0.0",
              "host_portmappings": { "8082": "8082" },
              "urls": ["local.admin.example.org"]
            }
          }
        }
      },
      "volumes": [...],
      "variables": {...},
      "host_portmappings": {...},
      "serve_command": "...",
      "shell_command": "...",
      "image_repository": "...",
      "platform": "...",
      "default_container_image": "..."
    }
  },
  "environments": {
    "go": {
      "serve_command": "air",
      "shell_command": "bash",
      "image_repository": "my-registry/go",
      "default_container_image": "1.25",
      "platform": "linux/amd64",
      "volumes": [...],
      "variables": {...},
      "host_portmappings": {...}
    }
  }
}
```

## In-container `/etc/hosts`

darp bind-mounts a managed hosts file over `/etc/hosts` inside every `darp shell` / `darp serve` container. That file includes standard loopback entries, a line for the container engine's host-gateway (`host.docker.internal` or `host.containers.internal` resolved to the platform-correct IP), and one `0.0.0.0 <service>.<domain>.test` line per service for intra-service reachability. The gateway IP is probed once by `darp install` and cached at `~/.darp/container_host_ip`; `darp deploy` re-probes automatically if the cache is missing or was written for a different engine.

## Tokens

These tokens are expanded at runtime:

| Token | Expands to |
|---|---|
| `{home}` | Your home directory (e.g. `/Users/you`) |
| `{pwd}` | The current project directory (volumes only) |

## Available Settings

These settings can be configured at the **environment**, **domain**, **group**, or **service** level:

| Setting | Description |
|---|---|
| `serve_command` | Command run by `darp serve` |
| `shell_command` | Shell used by `darp shell` (default: `sh`) |
| `image_repository` | Docker registry prefix (image becomes `repo:tag`) |
| `default_container_image` | Image used when none is passed on the CLI |
| `platform` | Container platform (e.g. `linux/amd64`) |
| `host_portmappings` | Map of `host_port: container_port` to expose |
| `variables` | Map of `name: value` environment variables |
| `volumes` | List of `{ container, host }` mount paths |

Additionally:

| Setting | Where | Description |
|---|---|---|
| `default_environment` | Domain, Group, Service | Fallback environment when `-e` isn't passed |
| `location` | Domain | Filesystem path to the domain folder |
| `urls` | Service | Extra hostnames that reach this service alongside `{service}.{domain}.test` |
| `nginx` | Global, Domain, Group, Service | Map of nginx directives, e.g. `client_max_body_size`, `proxy_read_timeout`. See [nginx Settings](#nginx-settings) |
| `dns_port` | Global | Host port darp's DNS server listens on, when something else holds 53. See [DNS Port](#dns-port) |

## URL Aliases

A service normally answers on one hostname, `{service}.{domain}.test`, derived
from its folder name. `urls` adds more:

```json
"portal-website": {
  "urls": ["local.comagine.org", "local.zoo.org"]
}
```

`darp deploy` then writes an nginx `server` block for each alias, all proxying to
the same upstream port as the canonical URL, and a `hosts_container` entry so the
name works between containers. `darp urls` lists aliases indented under their
service.

### Making a non-`.test` alias resolve

Aliases do not have to end in `.test`, but darp only resolves the name for you in
some cases:

| Alias | Resolves on the host because |
|---|---|
| `tenant-a.portal.test` | darp's dnsmasq rule `address=/.test/127.0.0.1` wildcards every `.test` name. Nothing else needed. |
| `local.zoo.org` | Only with `urls_in_hosts: true`, which lets darp manage an entry in your system hosts file — or with another DNS source you run that resolves the name to `127.0.0.1`. |

Configuring an alias does **not** implicitly enable or modify hosts-file
synchronization; `urls_in_hosts` stays whatever you set it to. When darp detects
a non-`.test` alias and `urls_in_hosts` is off, `darp deploy` prints a warning
naming each affected service and alias, and points at:

```sh
darp config set urls-in-hosts true
```

The warning is informational — deployment continues, because darp cannot tell
whether you already resolve those names some other way.

WSL users who need the alias to work in **Windows** applications (a browser on
the Windows side, for instance) may also need:

```sh
darp config set wsl true
```

which syncs the Windows hosts file alongside the Linux one. darp does not warn
when `wsl` is false, since resolution inside WSL alone is a legitimate setup.

This exists for apps that behave differently per hostname — a multi-tenant
front-end that picks its tenant from `window.location.hostname`, for instance.
Because every alias reaches the same container on the same port, the app sees
the original hostname in the `Host` header and can branch on it.

`urls` is service-level only and applies at deploy time. It does not cascade
through group/domain/environment and has no `*urls` override form.

### Hostname rules

An alias value is a **hostname, not a URL**. `darp deploy` validates every alias
before it writes anything, and rejects the whole deploy — leaving the previous
deployment untouched — if any value is malformed. Every invalid alias across
every service is reported in one error.

Not allowed:

- Schemes (`http://`, `https://`), ports, paths, query strings, fragments, and
  user information.
- Whitespace, control characters, and characters that would alter nginx or
  hosts-file syntax (`;`, `{`, `}`, quotes, backslash).
- Wildcards such as `*.zoo.org`.
- IPv4 and IPv6 address literals — `urls` names alternate hostnames, not
  alternate listener addresses.
- Labels that are empty, longer than 63 characters, or that begin or end with a
  hyphen; hostnames longer than 253 characters.
- Raw non-ASCII names. Internationalized hostnames must be supplied in
  ASCII/Punycode form (`xn--…`).

Normalization applied to accepted values:

- Surrounding whitespace is trimmed.
- One optional trailing DNS dot is accepted and removed (`local.zoo.org.` →
  `local.zoo.org`).
- ASCII letters are lowercased, since DNS is case-insensitive.

The normalized value is what lands in the nginx `server_name`, the hosts files,
`portmap.json`, and `darp urls` output.

Valid:

```json
"urls": ["local.zoo.org", "tenant-a.portal.test"]
```

### One hostname, one service

Every HTTP/WebSocket hostname must identify exactly one service. nginx routes by
`server_name`: if two server blocks declare the same name it ignores the second
and sends all traffic for that hostname to whichever service was registered
first — and registration follows directory-scan order, which is not stable, so
the winner can change from one deploy to the next.

`darp deploy` therefore refuses to deploy when a hostname is claimed twice, and
reports every conflict at once:

- An alias cannot duplicate another HTTP/WebSocket service's alias or canonical
  URL.
- Two canonical URLs cannot collide either. A canonical URL is
  `{folder}.{domain}.test` with no group component, so two identically-named
  folders in different groups of the same domain claim the same hostname.

Comparison is case-insensitive, and a terminal DNS dot does not make a hostname
unique — `LOCAL.ZOO.ORG`, `local.zoo.org.`, and `local.zoo.org` are one name.

Repeated aliases *within* one service are not a conflict: they resolve to the
same upstream, so they are deduplicated to a single hosts entry and a single
server block. An alias equal to the service's own canonical URL is dropped for
the same reason. This is what makes `pre_config` merging safe — array values
concatenate, so a personal config re-listing a team alias is harmless.

TCP services are exempt. They get no nginx server block and are distinguished by
their assigned ports, so two TCP services — or a TCP service and an
HTTP/WebSocket service — may share a hostname. Syntax validation and
same-service deduplication still apply to their aliases.

Invalid:

```json
"urls": [
  "https://local.zoo.org",
  "local.zoo.org:8080",
  "local.zoo.org/path"
]
```

## nginx Settings

darp generates `~/.darp/nginx.conf` and `~/.darp/vhost_container.conf` on every
`darp deploy`, so editing them by hand doesn't last. Set nginx directives in an
`nginx` map instead:

```json
{
  "nginx": { "client_max_body_size": "100m" },
  "domains": {
    "my-domain": {
      "location": "{home}/projects",
      "nginx": { "proxy_read_timeout": "300s" },
      "groups": {
        ".": {
          "services": {
            "uploader": { "nginx": { "client_max_body_size": 0 } },
            "events": { "nginx": { "proxy_buffering": "off" } }
          }
        }
      }
    }
  }
}
```

```sh
darp config set nginx client_max_body_size 100m
darp config set dom nginx my-domain proxy_read_timeout 300s
darp config set svc nginx my-domain . uploader client_max_body_size 0
darp config rm svc nginx my-domain . uploader client_max_body_size
darp deploy
```

### Where directives go

- **Global** directives go in the `http` block of `nginx.conf` and apply to every
  service. If darp already sets that directive (for example
  `client_max_body_size 1m` or `server_tokens off`), yours replaces darp's value.
- **Domain, group, and service** directives go inside the `location /` block of
  that service's vhosts, for its canonical hostname and every `urls` alias. They
  override the global value. Maps merge down the chain, with service beating group
  beating domain. `*nginx` replaces the inherited map, and `"*nginx": null` clears
  it. Environments don't apply, because vhosts are written at deploy time.

### Values

- Each entry is one directive: `"proxy_read_timeout": "300s"` becomes
  `proxy_read_timeout 300s;`. Values can contain spaces
  (`"large_client_header_buffers": "4 16k"`) but not `;`, `{`, `}`, `#`, or line
  breaks.
- Values are strings. Bare JSON numbers are also accepted
  (`"client_max_body_size": 0`).
- Size directives (`client_max_body_size`, `client_body_buffer_size`,
  `proxy_buffer_size`, `proxy_busy_buffers_size`) need a `k`, `m`, or `g` unit,
  or `0`. nginx reads a bare number as bytes, so `50` would mean 50 bytes, not 50
  megabytes.
- darp manages `listen`, `server_name`, `location`, `proxy_pass`, and `include`
  itself, so they can't be set. Block directives (`server`, `map`, `upstream`,
  `if`, ...) can't be expressed as one line.
- A map holds one value per directive name, so directives that nginx allows more
  than once (such as several `add_header` lines) can only be set once.

### Deploy check

Before replacing anything, `darp deploy` runs `nginx -t` on the new configuration
in a throwaway `nginx:alpine` container. An unknown directive, a bad value, or a
directive that isn't allowed where darp puts it stops the deploy with nginx's
error message, and the running deployment is left untouched.

### Common settings

| Problem | Directive |
|---|---|
| Uploads fail with `413 Request Entity Too Large` | `client_max_body_size` (e.g. `100m`, or `0` for no limit) |
| `504 Gateway Timeout` while paused at a debugger breakpoint, or HMR websockets reconnecting every minute | `proxy_read_timeout` / `proxy_send_timeout` (e.g. `3600s`) |
| Server-Sent Events or streamed responses arrive all at once | `proxy_buffering off` |
| `502` with "upstream sent too big header" in the proxy logs | `proxy_buffer_size` (e.g. `16k`) |
| `400 Request Header Or Cookie Too Large` | `large_client_header_buffers` (e.g. `4 16k`, global) |

## DNS Port

`.test` names resolve through `darp-masq`, a dnsmasq container that darp publishes
on host port 53, and `/etc/resolver/test`, which sends `.test` lookups to
`127.0.0.1`. When something else needs port 53 on your machine — a VPN client's DNS
proxy, for example — move darp-masq to another port:

```sh
darp config set dns-port 5300
darp deploy
```

`darp deploy` then publishes darp-masq on `127.0.0.1:5300` only (dnsmasq still
listens on 53 inside the container), replacing a darp-masq published anywhere else.
On macOS it also adds `port 5300` to `/etc/resolver/test`, which needs sudo, keeping
any other options in the file. `darp install` writes the resolver file from scratch
with the same port. Both settle on the configured port on every run, so a reinstall
or redeploy keeps it. Only macOS reads that `port` line (resolver(5)); Linux never
reads `/etc/resolver/test`, so neither command writes it there and no host lookup
reaches a moved darp-masq — use `darp config set urls-in-hosts true` instead.

`darp config rm dns-port` and another `darp deploy` put both back on port 53.
Unset, or set to `53`, darp-masq is published on port 53 on every interface.
Deploy then leaves a darp-masq already on 53 alone and replaces one published on
any other port, and on macOS it rewrites `/etc/resolver/test` (sudo) only when the
file has a `port` line, removing it. That holds for a darp-masq or `port` line set
up by hand, too, so set `dns-port` before deploying on a machine that moved
darp-masq itself.

A running darp-masq is only removed once its replacement has started. If the new
port is already taken, deploy prints a warning and carries on, leaving the running
darp-masq and the resolver file as they were. `darp doctor` warns when either one
doesn't match the setting. If darp-masq wasn't running and can't start, or the
resolver file can't be written, deploy also warns and carries on.

## Viewing Resolved Config

To see what settings would apply at your current directory:

```sh
darp config show              # uses default environment
darp config show -e staging   # override environment
```

This outputs the fully resolved JSON after applying the Service > Group > Domain > Environment chain.
