# Team Collaboration

darp supports sharing configuration across teams using **pre_config** -- parent config files that get merged into your local config. This replaces manual copy-pasting of settings.

## How It Works

1. Your team maintains a shared config in a git repo
2. Each team member points their `~/.darp/config.json` at the shared config(s)
3. darp merges the shared configs with your local overrides
4. Run `darp config pull` to update from the team repos

## Setup

### Team side: create a shared config repo

```sh
mkdir ~/team-darp-config && cd ~/team-darp-config
git init
```

Create `config.json`:

```json
{
  "engine": "docker",
  "domains": {
    "my-org": {
      "location": "{home}/my-org",
      "groups": {
        "go": {
          "default_environment": "go"
        },
        "laravel": {
          "default_environment": "lara:13",
          "services": {
            "admin": {
              "serve_command": "php artisan serve --host 0.0.0.0"
            }
          }
        }
      }
    }
  },
  "environments": {
    "go": {
      "image_repository": "registry.example.com/go",
      "default_container_image": "1.25",
      "serve_command": "air"
    },
    "lara:13": {
      "image_repository": "registry.example.com/php",
      "default_container_image": "8.4",
      "serve_command": "php artisan serve --host 0.0.0.0"
    }
  }
}
```

Push it to your shared git host.

### Developer side: link the team config

```sh
git clone git@your-host:team/darp-config.git ~/team-darp-config
darp config add pre-config '{home}/team-darp-config/config.json' -r '{home}/team-darp-config'
```

That's it. darp will now merge the team config with your local settings.

## Multiple Teams

You can have multiple pre_config entries for different teams:

```sh
darp config add pre-config '{home}/team-a/config.json' -r '{home}/team-a'
darp config add pre-config '{home}/team-b/config.json' -r '{home}/team-b'
```

**Constraint:** Pre-configs cannot define the same domain name. If two pre_configs both define a domain called `"my-org"`, darp will error on load. This prevents silent conflicts.

Environments *can* overlap across pre_configs -- later entries in the array take priority.

## Pulling Updates

When the team pushes config changes, pull them all at once:

```sh
darp config pull
```

This runs `git pull` in each pre_config's `repo_location`. Entries without a `repo_location` are skipped.

## Merge Behavior

Pre-configs are merged in array order, then your local config overlays on top.

**Default: field-level merge**
- Objects/maps merge by key (both sides' keys are present, local wins per key)
- Arrays (e.g. volumes) concatenate
- Scalars: local wins if present

**Override with `*` prefix**

To completely replace a value instead of merging, prefix the JSON key with `*`:

```json
{
  "environments": {
    "go": {
      "*volumes": [
        { "container": "/root/.ssh", "host": "{home}/.ssh" }
      ]
    }
  }
}
```

This replaces the team's volumes entirely instead of appending to them.

The `*` prefix works at any nesting level:
- `"*volumes"` -- replace the volumes array
- `"*go"` -- replace the entire `go` environment
- `"*environments"` -- replace all environments

## Local Overrides

Your `~/.darp/config.json` always has the highest priority. Common overrides:

**Change a domain's location** (e.g. your repo is in a different folder):

```json
{
  "domains": {
    "my-org": {
      "location": "{home}/custom-path/my-org"
    }
  }
}
```

**Add personal volumes** (merged with team volumes by default):

```json
{
  "domains": {
    "my-org": {
      "volumes": [
        { "container": "/root/.claude/", "host": "{home}/.claude/" }
      ]
    }
  }
}
```

**Add a personal environment**:

```json
{
  "environments": {
    "local-rust": {
      "default_container_image": "local-rust:latest"
    }
  }
}
```

## Managing Pre-configs

```sh
darp config add pre-config <location> [-r <repo_location>]
darp config rm pre-config <location>
darp config pull
```

The resulting `pre_config` field in your config.json:

```json
{
  "pre_config": [
    {
      "location": "{home}/team-darp-config/config.json",
      "repo_location": "{home}/team-darp-config"
    },
    {
      "location": "{home}/other-team/config.json"
    }
  ]
}
```
