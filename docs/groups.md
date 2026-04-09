# Groups

Groups organize services within a domain by tech stack or team. They're an optional layer between domains and services.

## Why Groups?

Without groups, all services in a domain share the same defaults. With groups, you can set different defaults per stack:

```
~/my-org/                           <- domain
  ~/my-org/laravel/                 <- group "laravel" (default_environment: lara:13)
    admin/                          <- admin.my-org.test
    api-gateway/                    <- api-gateway.my-org.test
  ~/my-org/go/                      <- group "go" (default_environment: go)
    queue-monitor/                  <- queue-monitor.my-org.test
    auth-service/                   <- auth-service.my-org.test
```

Groups **don't affect URLs**. Both `admin` and `queue-monitor` are under `my-org.test`.

## The `.` Group

The `.` group represents services directly in the domain folder (not in a subfolder):

```
~/my-org/                           <- domain
  standalone-tool/                  <- "." group -> standalone-tool.my-org.test
  ~/my-org/go/                      <- group "go"
    my-api/                         <- go group -> my-api.my-org.test
```

When you don't use groups at all, darp auto-migrates your services into a `.` group behind the scenes.

## Resolution Chain

Settings cascade through four levels:

```
Service > Group > Domain > Environment
```

For example, if the `go` group sets `default_environment: go`, all services in that group use the `go` environment unless they override it themselves.

## Setting Up Groups

### Via CLI

```sh
# Create a group
darp config add grp group my-org go

# Set group defaults
darp config set grp default-environment my-org go go
darp config set grp default-environment my-org laravel 'lara:13'

# Add group-level volumes
darp config add grp volume my-org laravel /root/.composer/auth.json '{home}/.composer/auth.json'

# Add a service to a group
darp config set svc serve-command -g laravel my-org admin 'php artisan serve --host 0.0.0.0'
```

### Via JSON

```json
{
  "domains": {
    "my-org": {
      "location": "{home}/my-org",
      "groups": {
        ".": {},
        "laravel": {
          "default_environment": "lara:13",
          "volumes": [
            { "container": "/root/.composer/auth.json", "host": "{home}/.composer/auth.json" }
          ],
          "services": {
            "admin": {
              "serve_command": "php artisan serve --host 0.0.0.0"
            }
          }
        },
        "go": {
          "default_environment": "go"
        }
      }
    }
  }
}
```

## How darp Detects Your Group

When you run `darp serve` or `darp shell`, darp looks at your current directory:

1. **Parent is a domain folder?** You're in the `.` group.
   - `~/my-org/standalone-tool/` -> domain=my-org, group=`.`, service=standalone-tool

2. **Grandparent is a domain folder?** Parent folder name is your group.
   - `~/my-org/go/auth-service/` -> domain=my-org, group=`go`, service=auth-service

## Deploy Behavior

When `darp deploy` scans a domain:

- The `.` group scans the domain folder directly, **skipping** subdirectories that are named groups
- Named groups scan their own subdirectory

This means `~/my-org/go/` won't be registered as a service -- it's recognized as a group directory.
