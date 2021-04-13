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
    tt [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --data-file <data-file>    which data file to use

SUBCOMMANDS:
    continue    continue time tracking with last description
    export      export the file as json
    help        Prints this message or the help of the given subcommand(s)
    import      
    list        list all entries
    path        show path to data file
    show        show work time for given timespan
    start       start time tracking
    status      show info from the latest entry
    stop        stop time tracking
```
