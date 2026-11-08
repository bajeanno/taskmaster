# Taskmaster

Taskmaster is a Unix process supervisor written in Rust. It runs configured programs as managed processes and provides a command-line client for checking their status, starting and stopping them, and reloading configuration.

## Features

- Run one or more processes for each configured program.
- Automatically start programs at daemon startup or after a configuration reload.
- Restart processes always, only after unexpected exits, or never.
- Configure working directories, environment variables, umask, stop signal, and stdout/stderr output files.
- Reload configuration while the daemon is running. Changes to process count are applied directly; changes to process launch settings restart affected processes.
- Use an interactive `tmcli>` shell or pass a single command as arguments.

## Requirements

Taskmaster uses Unix facilities including Unix domain sockets, process signals, file locking, and `fork`. It uses `/etc/taskmaster.d/` for configuration, `/var/run/taskmaster.d/` for its socket and PID file, and `/var/log/taskmaster.log` for daemonized output. Ensure those directories exist and the account running Taskmaster has the required access. A normal release run daemonizes, debug builds run the daemon in the foreground.

## Build and run

Build the release binary with Rust and Cargo:

```sh
cargo build --release
```

Run it with the permissions and runtime directories described above:

```sh
./target/release/taskmaster
```

On its first run, Taskmaster uses `/etc/taskmaster.d/taskmaster.yaml` as the default config path and creates that YAML file with a small template if it does not exist. It stores the selected default path in `/etc/taskmaster.d/taskmaster.ron`. Create `/etc/taskmaster.d/` before starting Taskmaster, and edit the YAML file to configure programs. Starting without a command opens the interactive shell. If the daemon is not running, the client prompts to start it.

To run a single command, append it and its arguments:

```sh
./target/release/taskmaster status
```

## Configuration

The configuration file is YAML with a top-level `programs` mapping. Each program key is its name. Names may not contain digits or start with `-`. The `cmd` value is a command line string parsed with shell-style quoting.

```yaml
programs:
  web:
    cmd: "./server --port 8080"
    num-procs: 2
    working-dir: /srv/web
    auto-start: true
    auto-start-on-reload: false
    auto-restart: unexpected
    exit-codes: [0]
    start-retries: 3
    start-time: 5
    stop-signal: TERM
    stop-time: 10
    stdout: /var/log/web.out
    stderr: /var/log/web.err
    clear-env: false
    env:
      APP_ENV: production
    umask: "0o022"
```

`cmd` is required. Other fields are optional and default as follows:

| Field                  | Default     | Meaning                                                                                 |
| ---------------------- | ----------- | --------------------------------------------------------------------------------------- |
| `umask`                | `0o022`     | File creation mask; provide an explicit octal value such as `"0o027"`.                  |
| `num-procs`            | `1`         | Number of processes to maintain; must be greater than zero.                             |
| `working-dir`          | `/`         | Working directory for spawned processes.                                                |
| `auto-start`           | `false`     | Start this program when the daemon starts.                                              |
| `auto-start-on-reload` | `false`     | Start this program when configuration is reloaded.                                      |
| `auto-restart`         | `false`     | Restart policy: `false`, `true` (always), or `unexpected` (only on an unexpected exit). |
| `exit-codes`           | `[0]`       | Exit codes treated as expected.                                                         |
| `start-retries`        | `0`         | Retries after an unexpected failure during startup.                                     |
| `start-time`           | `0`         | Seconds allowed for a process to reach its started state.                               |
| `stop-signal`          | `TERM`      | Signal sent before Taskmaster escalates to `SIGKILL`. The `SIG` prefix is optional.     |
| `stop-time`            | `0`         | Seconds to wait after the stop signal before escalation.                                |
| `stdout`, `stderr`     | `/dev/null` | Output destinations. An empty string disables the corresponding file output.            |
| `clear-env`            | `false`     | Clear inherited environment variables before applying `env`.                            |
| `env`                  | `{}`        | Environment variables passed to the process.                                            |

Unknown fields are rejected. Output files are opened while loading the configuration.

## Commands

These commands work in the interactive shell and as arguments to the executable:

| Command                       | Description                                                  |
| ----------------------------- | ------------------------------------------------------------ |
| `status`                      | Show managed program and process status.                     |
| `start <program>`             | Start a program.                                             |
| `restart <program>`           | Restart a program.                                           |
| `stop <program>`              | Stop a program.                                              |
| `stop --all`                  | Stop all managed programs.                                   |
| `reload --default`            | Reload the configured default YAML file.                     |
| `reload --current`            | Reload the file used by the current active configuration.    |
| `reload <path>`               | Load a YAML file for this reload only.                       |
| `reload --new-default <path>` | Set the default path and load that YAML file.                |
| `restore-config-file`         | Write the active configuration back to its source YAML file. |
| `shutdown`                    | Stop all programs and shut down the daemon.                  |

Examples:

```sh
taskmaster status
taskmaster reload --current
taskmaster start web
taskmaster stop --all
taskmaster shutdown
```

The client and daemon communicate through `/var/run/taskmaster.d/taskmaster.sock`; the daemon PID claim is stored at `/var/run/taskmaster.d/taskmaster.pid`.
