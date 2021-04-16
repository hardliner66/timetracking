# Timetracking

Simple command line time tracking application I wrote to keep track of how many hours I already spent working in a week.

[![Crates.io](https://img.shields.io/crates/v/timetracking)](https://crates.io/crates/timetracking)

## Install
```
cargo install timetracking
```

## Commandline
```
USAGE:
    tt [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --data-file <data-file>    which data file to use. [default: ~/timetracking.bin]

SUBCOMMANDS:
    continue    continue time tracking with last description
    export      export data to file
    help        Prints this message or the help of the given subcommand(s)
    import      import data from json file
    list        list all entries
    path        show path to data file
    show        show work time for given timespan
    start       start time tracking
    status      show info from the latest entry. Returns the exit code 0, if the time tracking is currently active
                and -1 if not
    stop        stop time tracking
```

## Example Usage
Start tracking:
`tt start`

Stop tracking:
`tt stop`

Show work time of the current day:
`tt show`

Show work time of the current week:
`tt show week`

List all entries for the current day:
`tt list`

Export to json:
`tt export backup.json`

Import from json:
`tt import backup.json`

## Config

`tt` supports global config (`~/.config/timetracking/config.toml`), project config (`timetracking.project.toml`) and local config (`.timetracking.toml`).

The following settings are supported:
```toml
# the file where to save the events
data_file = "~/timetracking.bin"

# if true, calling start when already running inserts a stop event and a start event.
auto_insert_stop = false

# if true, tt will recursively search parent dirs for project settings
enable_project_settings = true

# minimum amount of minutes of break time per day
min_daily_break = 0

# set the daily time goal
[time_goal.daily]
# work hours to reach in a work day (0-24)
hours = 8

# work minutes to reach in a work day (0-59)
minutes = 0

# set the weekly time goal
[time_goal.weekly]
# work hours to reach in a work week (0-168)
hours = 40

# work minutes to reach in a work week (0-59)
minutes = 0
```

The order in which config files are read is:
- global
- project
- local

Configs override earlier loaded configs.

Project configs are special and will be search recursively upwards, starting from the current directory. So if your in /a/b/c the search order will be:
- /a/b/c/timetracking.project.toml
- /a/b/timetracking.project.toml
- /a/timetracking.project.toml
- /timetracking.project.toml

Project configs can be disabled in the global config file.

## Starship

You can use the following snippet to show how much you worked today,
while the time tracking is running.

Just add it to your starship config (e.g.: ~/.config/starship.toml)
```yml
[custom.worktime]
command = """ tt show --format "{h}h {mm}m" """
when = "tt status"
shell = "sh"
```

This is how it looks like:

![Starship Prompt](https://user-images.githubusercontent.com/2937272/114703152-38f71600-9d25-11eb-8fee-564d2efe2c8e.png)
