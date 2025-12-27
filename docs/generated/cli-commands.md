# CLI Commands Reference

This document is auto-generated from the halvor CLI. For the most up-to-date information, run `halvor --help` or `halvor <command> --help`.

Halvor - CLI tool for managing homelab infrastructure

Usage: halvor [OPTIONS] <COMMAND>

Commands:
  backup     Backup services, config, and database
  restore    Restore services, config, or database
  sync       Sync encrypted data between hal installations
  list       List services or hosts
  install    Install an app on a host
  uninstall  Uninstall a service from a host or halvor itself
  config     Configure halvor settings (environment file location, etc.)
  db         Database operations (migrations, backup, generate)
  update     Update halvor or installed apps
  generate   Generate build artifacts (migrations, FFI bindings)
  init       Initialize K3s cluster (primary control plane node) or prepare a node for joining
  join       Join a node to the K3s cluster
  status     Show status of services (mesh overview by default)
  agent      Manage halvor agent (start, stop, discover, sync)
  help       Print this message or the help of the given subcommand(s)

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
  -h, --help                 Print help
  -V, --version              Print version

## Subcommands


### `halvor backup`

```
Backup services, config, and database

Usage: halvor backup [OPTIONS] [SERVICE]

Arguments:
  [SERVICE]  Service to backup (e.g., portainer, sonarr). If not provided, interactive selection

Options:
      --env                  Backup to env location instead of backup path
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
      --list                 List available backups instead of creating one
      --db                   Backup the database (unencrypted SQLite backup)
      --path <PATH>          Path to save database backup (only used with --db)
  -h, --help                 Print help
```

### `halvor restore`

```
Restore services, config, or database

Usage: halvor restore [OPTIONS] [SERVICE]

Arguments:
  [SERVICE]  Service to restore (e.g., portainer, sonarr). If not provided, interactive selection

Options:
      --env                  Restore from env location instead of backup path
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
      --backup <BACKUP>      Specific backup timestamp to restore (required when service is specified)
  -h, --help                 Print help
```

### `halvor sync`

```
Sync encrypted data between hal installations

Usage: halvor sync [OPTIONS]

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
      --pull                 Pull data from remote instead of pushing
  -h, --help                 Print help
```

### `halvor list`

```
List services or hosts

Usage: halvor list [OPTIONS]

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
      --verbose              Show verbose information
  -h, --help                 Print help
```

### `halvor install`

```
Install an app on a host

Usage: halvor install [OPTIONS] [APP]

Arguments:
  [APP]  App to install (e.g., docker, sonarr, portainer). Use --list to see all

Options:
  -H, --hostname <HOSTNAME>    Hostname to operate on (defaults to localhost if not provided)
      --list                   List all available apps
      --repo <REPO>            Helm repository URL for external charts (e.g., https://pkgs.tailscale.com/helmcharts)
      --repo-name <REPO_NAME>  Helm repository name (defaults to chart name if not provided)
      --name <NAME>            Custom release name for Helm charts (allows multiple instances of the same app, e.g., radarr-4k)
  -h, --help                   Print help
```

### `halvor uninstall`

```
Uninstall a service from a host or halvor itself

Usage: halvor uninstall [OPTIONS] [SERVICE]

Arguments:
  [SERVICE]  Service to uninstall (e.g., portainer, smb, nginx-proxy-manager). If not provided, guided uninstall of halvor

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
  -h, --help                 Print help
```

### `halvor update`

```
Update halvor or installed apps

Usage: halvor update [OPTIONS] [APP]

Arguments:
  [APP]  App to update (e.g., docker, tailscale, portainer). If not provided, updates everything on the system

Options:
      --experimental         Use experimental channel for halvor updates (version less, continuously updated)
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
      --force                Force download and install the latest version (skips version check)
  -h, --help                 Print help
```

### `halvor init`

```
Initialize K3s cluster (primary control plane node) or prepare a node for joining

Usage: halvor init [OPTIONS]

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
      --token <TOKEN>        Token for cluster join (generated if not provided)
  -y, --yes                  Skip confirmation prompts
      --skip-k3s             Skip K3s initialization - only install tools and configure node (useful for nodes that will join an existing cluster)
  -h, --help                 Print help
```

### `halvor join`

```
Join a node to the K3s cluster

Usage: halvor join [OPTIONS] [HOSTNAME]

Arguments:
  [HOSTNAME]  Target hostname to join to the cluster (use -H/--hostname to specify)

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
      --server <SERVER>      First control plane node address (e.g., frigg or 192.168.1.10). If not provided, will try to auto-detect from config
      --token <TOKEN>        Cluster join token (if not provided, will be loaded from K3S_TOKEN env var or fetched from server)
      --control-plane        Join as control plane node (default: false, use --control-plane to join as control plane)
  -h, --help                 Print help
```

### `halvor status`

```
Show status of services (mesh overview by default)

Usage: halvor status [OPTIONS] [COMMAND]

Commands:
  k3s        Show K3s cluster status (nodes, etcd health)
  helm       List Helm releases
  tailscale  Show Tailscale nodes available on the tailnet
  help       Print this message or the help of the given subcommand(s)

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
  -h, --help                 Print help
```

### `halvor configure`

```
error: unrecognized subcommand 'configure'

  tip: a similar subcommand exists: 'config'

Usage: halvor [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### `halvor config`

```
Configure halvor settings (environment file location, etc.)

Usage: halvor config [OPTIONS] [COMMAND]

Commands:
  list          List current configuration
  init          Initialize or update halvor configuration (interactive)
  set-env       Set the environment file path
  stable        Set release channel to stable
  experimental  Set release channel to experimental
  create        Create new configuration
  env           Create example .env file
  set-backup    Set backup location (for current system if no hostname provided)
  commit        Commit host configuration to database (from .env to DB)
  backup        Write host configuration back to .env file (from DB to .env, backs up current .env first)
  delete        Delete host configuration
  ip            Set IP address for hostname
  hostname      Set hostname (typically Tailscale hostname)
  backup-path   Set backup path for hostname
  diff          Show differences between .env and database configurations
  kubeconfig    Get kubeconfig for K3s cluster
  regenerate    Regenerate K3s certificates with Tailscale integration
  help          Print this message or the help of the given subcommand(s)

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
  -v, --verbose              Show verbose output (including passwords)
      --db                   Show database configuration instead of .env
  -h, --help                 Print help
```

### `halvor db`

```
Database operations (migrations, backup, generate)

Usage: halvor db [OPTIONS] <COMMAND>

Commands:
  backup    Backup the SQLite database
  generate  Generate Rust structs from database schema
  migrate   Manage database migrations (defaults to running all pending migrations)
  sync      Sync environment file to database (load env values into DB, delete DB values not in env)
  restore   Restore database from backup
  help      Print this message or the help of the given subcommand(s)

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
  -h, --help                 Print help
```

### `halvor build`

```
error: unrecognized subcommand 'build'

Usage: halvor [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### `halvor dev`

```
error: unrecognized subcommand 'dev'

Usage: halvor [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### `halvor generate`

```
Generate build artifacts (migrations, FFI bindings)

Usage: halvor generate [OPTIONS] <COMMAND>

Commands:
  ffi-bindings  Generate FFI bindings for all platforms
  migrations    Generate migration declarations
  api-clients   Generate API client libraries (TypeScript, Kotlin, Swift)
  all           Generate everything (migrations + FFI bindings + API clients)
  help          Print this message or the help of the given subcommand(s)

Options:
  -H, --hostname <HOSTNAME>  Hostname to operate on (defaults to localhost if not provided)
  -h, --help                 Print help
```
