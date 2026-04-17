# Configuration Guide

darp stores its configuration in `~/.darp/config.json`. You can edit this file directly or use `darp config` commands.

## Settings Resolution

When you run `darp serve` or `darp shell`, settings are resolved from most specific to least specific:

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
              "host_portmappings": { "8082": "8082" }
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

## Viewing Resolved Config

To see what settings would apply at your current directory:

```sh
darp config show              # uses default environment
darp config show -e staging   # override environment
```

This outputs the fully resolved JSON after applying the Service > Group > Domain > Environment chain.
