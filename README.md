# Timetracking

Simple command line time tracking application.

[![Crates.io](https://img.shields.io/crates/v/timetracking)](https://crates.io/crates/timetracking)

## Install
```
cargo install timetracking
```

## Commandline
```
USAGE:
    tt [data-file] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <data-file>    which data file to use. [default: ~/timetracking.bin]

SUBCOMMANDS:
    continue    continue time tracking with last description
    export      export data to file
    help        Prints this message or the help of the given subcommand(s)
    import      import data from json file
    list        list all entries
    path        show path to data file
    show        show work time for given timespan
    start       start time tracking
    status      show info from the latest entry
    stop        stop time tracking
```

## Starship

You can use the following snippet to show how much you worked today,
while the time tracking is running.

Just add it to your starship config (e.g.: ~/.config/starship.toml)
```yaml
[custom.worktime]
command = "tt show"
when = "tt status"
shell = "sh"
```
