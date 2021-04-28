use anyhow::{Context, Result};
use chrono::{prelude::*, serde::ts_seconds, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use iif::iif;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::{self, Write}};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod settings;

use settings::Settings;

#[derive(Debug, StructOpt)]
struct Options {
    #[cfg(feature = "binary")]
    /// which data file to use. [default: ~/timetracking.bin]
    #[structopt(short, long)]
    data_file: Option<PathBuf>,

    #[cfg(not(feature = "binary"))]
    /// which data file to use. [default: ~/timetracking.json]
    #[structopt(short, long)]
    data_file: Option<PathBuf>,

    /// which config file to use.
    #[structopt(short, long)]
    config_file: Option<String>,

    #[structopt(subcommand)]
    command: Option<Command>,
}

#[derive(Default, Debug, StructOpt)]
struct FilterData {
    /// show all entries after this point in time [defaults to current day 00:00:00]
    /// allowed formats are: "%Y-%m-%d %H:%M:%S", "%Y-%m-%d", "%H:%M:%S"
    #[structopt(short, long)]
    from: Option<String>,

    /// show all entries before this point in time [defaults to start day 23:59:59]
    /// allowed formats are: "%Y-%m-%d %H:%M:%S", "%Y-%m-%d", "%H:%M:%S"
    #[structopt(short, long)]
    to: Option<String>,

    /// filter entries. possible filter values: "week", "all" or part of the description
    filter: Option<String>,
}

#[derive(Debug, StructOpt)]
enum Command {
    // keep this at the top, otherwise rust analyzer will underline the whole struct until this
    // point as it thinks there is a problem, because it doesn't understand that this variant is
    // disabled via attribute.
    #[cfg(not(feature = "binary"))]
    /// export data to file
    Export {
        /// where to write the output file
        path: PathBuf,
    },

    /// show info from the latest entry. Returns the exit code 0, if the time tracking is currently
    /// active and -1 if not.
    Status,

    /// starts an interactive cleanup session
    Cleanup,

    /// start time tracking
    Start {
        /// a description for the event
        description: Option<String>,

        /// the time at which the event happend.
        /// format: "HH:MM:SS" or "YY-mm-dd HH:MM:SS" [defaults to current time]
        #[structopt(short, long)]
        at: Option<String>,
    },

    /// stop time tracking
    Stop {
        /// a description for the event
        description: Option<String>,

        /// the time at which the event happend.
        /// format: "HH:MM:SS" or "YY-mm-dd HH:MM:SS" [defaults to current time]
        #[structopt(short, long)]
        at: Option<String>,
    },

    /// continue time tracking with last description
    Continue,

    /// list all entries
    List {
        #[structopt(flatten)]
        filter: FilterData,
    },

    /// show path to data file
    Path,

    /// show work time for given timespan
    Show {
        #[structopt(flatten)]
        filter: FilterData,

        /// show only the time with no additional text
        #[structopt(short, long)]
        plain: bool,

        /// show time until the defined time goals are met.
        #[structopt(short, long)]
        remaining: bool,

        /// include seconds in time calculation
        #[structopt(short)]
        include_seconds: bool,

        /// show only the time with no additional text. [default: "{hh}:{mm}:{ss}"]
        #[structopt(long)]
        format: Option<String>,
    },
    #[cfg(feature = "binary")]
    /// export data to file
    Export {
        /// export in a human readable format. This format is for human reading only and cannot be
        /// imported
        #[structopt(short, long)]
        readable: bool,
        /// pretty print json
        #[structopt(short, long)]
        pretty: bool,
        /// where to write the output file
        path: PathBuf,
    },
    #[cfg(feature = "binary")]
    /// import data from json file
    Import {
        /// which file to import
        path: PathBuf,
    },
}

impl Default for Command {
    fn default() -> Self {
        Self::Show {
            filter: FilterData::default(),
            format: None,
            include_seconds: false,
            plain: false,
            remaining: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TrackingData {
    description: Option<String>,

    #[serde(with = "ts_seconds")]
    time: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum TrackingEvent {
    Start(TrackingData),
    Stop(TrackingData),
}

impl TrackingEvent {
    fn time(&self, include_seconds: bool) -> DateTime<Utc> {
        match self {
            Self::Start(TrackingData { time, .. }) | Self::Stop(TrackingData { time, .. }) => {
                let time = *time;
                if include_seconds {
                    time
                } else {
                    time.with_second(0).expect("could not set seconds to zero")
                }
            }
        }
    }

    fn description(&self) -> Option<String> {
        match self {
            Self::Start(TrackingData { description, .. })
            | Self::Stop(TrackingData { description, .. }) => description.clone(),
        }
    }

    fn is_start(&self) -> bool {
        match self {
            Self::Start(_) => true,
            Self::Stop(_) => false,
        }
    }

    fn is_stop(&self) -> bool {
        match self {
            Self::Start(_) => false,
            Self::Stop(_) => true,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug, Clone, Copy)]
enum DateOrDateTime {
    Date(NaiveDate),
    DateTime(NaiveDateTime),
}

impl From<NaiveDate> for DateOrDateTime {
    fn from(date: NaiveDate) -> Self {
        Self::Date(date)
    }
}

impl From<NaiveDateTime> for DateOrDateTime {
    fn from(date_time: NaiveDateTime) -> Self {
        Self::DateTime(date_time)
    }
}

#[cfg(feature = "binary")]
fn read_data<P: AsRef<Path>>(path: P) -> Result<Vec<TrackingEvent>> {
    let data = std::fs::read(&path)?;
    Ok(bincode::deserialize(&data)?)
}

#[cfg(not(feature = "binary"))]
fn read_data<P: AsRef<Path>>(path: P) -> Result<Vec<TrackingEvent>> {
    read_json_data(path)
}

fn read_json_data<P: AsRef<Path>>(path: P) -> Result<Vec<TrackingEvent>> {
    let data = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&data)?)
}

fn write_with_flush<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(contents.as_ref())?;
    f.flush()?;
    Ok(())
}

#[cfg(feature = "binary")]
fn write_data<P: AsRef<Path>>(path: P, data: &[TrackingEvent]) -> Result<()> {

    let data = bincode::serialize(data).expect("could not serialize data");

    let temp_path = path.as_ref().with_extension("bin.bak");

    match write_with_flush(&temp_path, &data) {
        Ok(_) => {
            Ok(std::fs::rename(temp_path, path.as_ref())?)
        }
        Err(e) => Err(e.into()),
    }
}

fn write_json_data<P: AsRef<Path>>(path: P, data: &[TrackingEvent], pretty: bool) -> Result<()> {
    let data = iif!(
        pretty,
        serde_json::to_string_pretty(data),
        serde_json::to_string(data)
    )
    .expect("could not serialize data");
    Ok(write_with_flush(&path, &data)?)
}

#[cfg(not(feature = "binary"))]
fn write_data<P: AsRef<Path>>(path: P, data: &[TrackingEvent]) -> Result<()> {
    write_json_data(path, data, false)
}

fn start_tracking(
    settings: &Settings,
    data: &mut Vec<TrackingEvent>,
    description: Option<String>,
    at: Option<String>,
) -> Result<()> {
    let (should_add, last_description) = match data.last() {
        None => (true, None),
        Some(event) => (event.is_stop(), event.description()),
    };
    if should_add || at.is_some() {
        data.push(TrackingEvent::Start(TrackingData {
            description,
            time: at.map_or_else(|| Ok(Local::now().into()), |at| parse_date_time(&at))?,
        }));
    } else if settings.auto_insert_stop && at.is_none() {
        match (description, last_description) {
            (Some(description), Some(last_description)) if description == last_description => {
                eprintln!(
                    "Timetracking with the description \"{}\" is already running!",
                    description
                )
            }
            (description, _) => {
                data.push(TrackingEvent::Stop(TrackingData {
                    description: None,
                    time: Local::now().into(),
                }));
                data.push(TrackingEvent::Start(TrackingData {
                    description,
                    time: Local::now().into(),
                }));
            }
        }
    } else {
        eprintln!("Time tracking is already running!");
    }

    Ok(())
}

fn stop_tracking(
    data: &mut Vec<TrackingEvent>,
    description: Option<String>,
    at: Option<String>,
) -> Result<()> {
    let should_add = match data.last() {
        None => true,
        Some(event) => event.is_start(),
    };
    if should_add || at.is_some() {
        data.push(TrackingEvent::Stop(TrackingData {
            description,
            time: at.map_or_else(|| Ok(Local::now().into()), |at| parse_date_time(&at))?,
        }))
    } else {
        eprintln!("Time tracking is already stopped!");
    }

    Ok(())
}

fn continue_tracking(data: &mut Vec<TrackingEvent>) {
    if let Some(TrackingEvent::Stop { .. }) = data.last() {
        if let Some(TrackingEvent::Start(TrackingData { description, .. })) =
            data.iter().rev().find(|t| t.is_start()).cloned()
        {
            data.push(TrackingEvent::Start(TrackingData {
                description,
                time: Local::now().into(),
            }))
        }
    } else {
        eprintln!("Time tracking couldn't be continued, because there are no entries. Use the start command instead!");
    }
}

fn split_duration(duration: Duration) -> (i64, i64, i64) {
    let hours = duration.num_hours();
    let hours_in_minutes = hours * 60;
    let hours_in_seconds = hours_in_minutes * 60;
    let minutes = duration.num_minutes() - hours_in_minutes;
    let minutes_in_seconds = minutes * 60;
    let seconds = duration.num_seconds() - hours_in_seconds - minutes_in_seconds;
    (hours, minutes, seconds)
}

fn filter_events(
    data: &[TrackingEvent],
    from: &Option<String>,
    to: &Option<String>,
    filter: &Option<String>,
) -> Result<Vec<TrackingEvent>> {
    let (filter, from, to) = match filter {
        Some(from) if from == "week" => {
            let now = Local::today();
            let weekday = now.weekday();
            let offset = weekday.num_days_from_monday();
            let (monday_offset, sunday_offset) = (offset, 6 - offset);
            let from = DateOrDateTime::Date(
                (now - Duration::days(i64::from(monday_offset))).naive_local(),
            );
            let to = DateOrDateTime::Date(
                (now + Duration::days(i64::from(sunday_offset))).naive_local(),
            );
            (None, Some(from), Some(to))
        }
        f => {
            let from = from.as_deref().map_or_else(
                || Ok(DateOrDateTime::Date(Local::today().naive_local())),
                parse_date_or_date_time,
            )?;

            let to = to
                .as_deref()
                .map(parse_date_or_date_time)
                .unwrap_or_else(|| {
                    Ok(match from {
                        DateOrDateTime::DateTime(from) => DateOrDateTime::Date(from.date()),
                        from @ DateOrDateTime::Date(..) => from,
                    })
                })?;
            (f.clone(), Some(from), Some(to))
        }
    };
    let data_iterator = data
        .iter()
        .filter(|entry| {
            iif!(
                filter.clone().unwrap_or_default() == "all",
                true,
                match from {
                    None => true,
                    Some(DateOrDateTime::Date(from)) => {
                        entry.time(true).timestamp_millis()
                            >= TimeZone::from_local_date(&Local, &from)
                                .unwrap()
                                .and_time(NaiveTime::from_hms(0, 0, 0))
                                .unwrap()
                                .timestamp_millis()
                    }
                    Some(DateOrDateTime::DateTime(from)) => {
                        entry.time(true).timestamp_millis()
                            >= TimeZone::from_local_datetime(&Local, &from)
                                .unwrap()
                                .timestamp_millis()
                    }
                }
            )
        })
        .filter(|entry| {
            iif!(
                filter.clone().unwrap_or_default() == "all",
                true,
                match to {
                    None => true,
                    Some(DateOrDateTime::Date(to)) => {
                        entry.time(true).timestamp_millis()
                            <= TimeZone::from_local_date(&Local, &to)
                                .unwrap()
                                .and_time(NaiveTime::from_hms(23, 59, 59))
                                .unwrap()
                                .timestamp_millis()
                    }
                    Some(DateOrDateTime::DateTime(to)) => {
                        entry.time(true).timestamp_millis()
                            <= TimeZone::from_local_datetime(&Local, &to)
                                .unwrap()
                                .timestamp_millis()
                    }
                }
            )
        })
        .filter(|entry| match entry {
            TrackingEvent::Start(TrackingData { description, .. })
            | TrackingEvent::Stop(TrackingData { description, .. }) => match (&filter, description)
            {
                (Some(filter), Some(description)) => {
                    filter == "all" || description.contains(filter)
                }
                (Some(filter), None) => filter == "all",
                (None, _) => true,
            },
        })
        .skip_while(|entry| TrackingEvent::is_stop(entry));

    Ok(data_iterator.cloned().collect())
}

fn get_data_as_days(data: &[TrackingEvent]) -> Vec<Vec<TrackingEvent>> {
    if data.is_empty() {
        return vec![];
    }

    let mut current_day = data.first().unwrap().time(true).date();
    let mut result = Vec::new();
    let mut current = Vec::new();
    for d in data {
        let date = d.time(true).date();
        if current_day == date {
            current.push(d.clone());
        } else {
            result.push(current);
            current = Vec::new();
            current.push(d.clone());
            current_day = date;
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    return result;
}

const CHECKED_ADD_DURATION_ERROR: &str = "couldn't add up durations";

fn get_time_from_day(
    settings: &Settings,
    data: &[TrackingEvent],
    include_seconds: bool,
) -> Duration {
    let mut data_iterator = data.iter();
    let mut work_day = Duration::zero();
    let mut first = None;
    let mut last = None;
    loop {
        let start = data_iterator.find(|e| e.is_start());
        let stop = data_iterator.find(|e| e.is_stop());
        match (start, stop) {
            (Some(start), Some(stop)) => {
                if let None = first {
                    first = Some(start.time(include_seconds));
                }
                last = Some(stop.time(include_seconds));
                let duration = stop.time(include_seconds) - start.time(include_seconds);
                work_day = work_day
                    .checked_add(&duration)
                    .expect(CHECKED_ADD_DURATION_ERROR);
            }
            (Some(start), None) => {
                if let None = first {
                    first = Some(start.time(include_seconds));
                }
                let now = if include_seconds {
                    Utc::now()
                } else {
                    Utc::now().with_second(0).unwrap()
                };
                last = Some(now);
                let duration = now - start.time(include_seconds);
                work_day = work_day
                    .checked_add(&duration)
                    .expect(CHECKED_ADD_DURATION_ERROR);
                break;
            }
            (_, _) => break,
        }
    }
    if settings.min_daily_break > 0 {
        let now = Utc::now();
        let total = last.unwrap_or(now) - first.unwrap_or(now);
        let pause = total - work_day;
        let min_break_duration = Duration::minutes(i64::from(settings.min_daily_break));
        if pause > Duration::zero() && pause < min_break_duration {
            let difference = min_break_duration - pause;
            work_day = work_day - difference;
        }
    }
    work_day.max(Duration::zero())
}

fn get_time_from_events(
    settings: &Settings,
    data: &[TrackingEvent],
    include_seconds: bool,
) -> Duration {
    let days = get_data_as_days(data);
    let mut time = Duration::zero();
    for day in days {
        let time_for_day = get_time_from_day(&settings, &day, include_seconds);
        time = time
            .checked_add(&time_for_day)
            .expect(CHECKED_ADD_DURATION_ERROR);
    }
    time
}

fn get_remaining_minutes(settings: &Settings, filter: &str, hours: i64, minutes: i64) -> i64 {
    let total = minutes + (hours * 60);
    let time_goal = if filter == "week" {
        &settings.time_goal.weekly
    } else {
        &settings.time_goal.daily
    };
    let required = i64::from(time_goal.minutes) + (i64::from(time_goal.hours) * 60);
    required - total
}

fn show(
    settings: &Settings,
    data: &[TrackingEvent],
    filter: &FilterData,
    format: Option<String>,
    include_seconds: bool,
    plain: bool,
    remaining: bool,
) -> Result<()> {
    let FilterData { from, to, filter } = filter;
    let filtered_data = filter_events(data, &from, &to, &filter)?;
    let work_time = get_time_from_events(&settings, &filtered_data, include_seconds);
    let (mut hours, mut minutes, mut seconds) = split_duration(work_time);

    let filter = filter.clone().unwrap_or_default();
    if remaining {
        if (filter == "week" || filter.is_empty()) && from.is_none() && to.is_none() {
            seconds = 0;
            let mut remaining_minutes = get_remaining_minutes(&settings, &filter, hours, minutes);

            if filter != "week" {
                let filtered_data_week =
                    filter_events(&data, &None, &None, &Some("week".to_string()))?;
                let week_work_time =
                    get_time_from_events(&settings, &filtered_data_week, include_seconds);
                let (week_hours, week_minutes, _) = split_duration(week_work_time);
                let remaining_minutes_week =
                    get_remaining_minutes(&settings, "week", week_hours, week_minutes);

                let today = Local::today().weekday();
                
                if today == settings.last_day_of_work_week {
                    // on last day in a work week, always show remaining minutes for week
                    remaining_minutes = remaining_minutes_week;
                } else {
                    // on all other days, show whichever is less
                    remaining_minutes = remaining_minutes.min(remaining_minutes_week);
                }
            }

            remaining_minutes = remaining_minutes.max(0);

            hours = remaining_minutes / 60;
            minutes = remaining_minutes - (hours * 60);
        } else {
            eprintln!("Remaining only works when \"from\" and \"to\" are not set and with no filter or filter \"week\"");
            return Ok(());
        }
    }
    let seconds_final = if include_seconds { seconds } else { 0 };
    let format = format.unwrap_or_else(|| "{hh}:{mm}:{ss}".to_string());
    let time = format
        .replace("{hh}", &format!("{:02}", hours))
        .replace("{mm}", &format!("{:02}", minutes))
        .replace("{ss}", &format!("{:02}", seconds_final))
        .replace("{h}", &format!("{}", hours))
        .replace("{m}", &format!("{}", minutes))
        .replace("{s}", &format!("{}", seconds_final));
    if plain {
        println!("{}", time);
    } else if remaining {
        println!("Remaining Work Time: {}", time);
    } else {
        println!("Work Time: {}", time);
    }

    Ok(())
}

fn cleanup(data: &[TrackingEvent]) -> Vec<TrackingEvent> {
    let mut cleaned = Vec::with_capacity(data.len());

    let mut data_iter = data.iter();
    let mut conflicting = Vec::new();

    let mut is_start = None;

    let mut all_conflicting = Vec::new();

    while let Some(e) = data_iter.next() {
        match is_start {
            None => {
                is_start = Some(e.is_start());
                cleaned.push(e);
            }
            Some(true) => {
                if e.is_start() {
                    if conflicting.is_empty() {
                        conflicting.push(cleaned.pop().unwrap());
                    }
                    conflicting.push(e);
                } else {
                    if !conflicting.is_empty() {
                        all_conflicting.push(conflicting);
                        conflicting = Vec::new();
                    }
                    cleaned.push(e);
                    is_start.replace(false);
                }
            }
            Some(false) => {
                if e.is_stop() {
                    if conflicting.is_empty() {
                        conflicting.push(cleaned.pop().unwrap());
                    }
                    conflicting.push(e);
                } else {
                    if !conflicting.is_empty() {
                        all_conflicting.push(conflicting);
                        conflicting = Vec::new();
                    }
                    cleaned.push(e);
                    is_start.replace(true);
                }
            }
        }
    }

    for mut conflicting in all_conflicting {
        let event_type = iif!(conflicting.first().unwrap().is_start(), "start", "stop");
        println!("Repeated {} events found:", event_type);
        for (i, event) in conflicting.iter().enumerate() {
            println!(
                "({}) {}",
                i,
                to_human_readable(
                    &format!("S{}", &event_type[1..]),
                    &event.time(true).with_timezone(&Local),
                    event.description()
                )
            );
        }
        loop {
            println!();
            println!("Please enter the number of the entry to keep (<num>|skip) [default: skip]: ");
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let text = input.trim();
                    if text == "skip" || text.is_empty() {
                        cleaned.append(&mut conflicting);
                        break;
                    } else {
                        let parsed: Result<usize, _> = text.parse();
                        match parsed {
                            Ok(n) => match conflicting.get(n) {
                                Some(value) => {
                                    cleaned.push(value);
                                    break;
                                }
                                None => println!("Please use one of the numbers given above!"),
                            },
                            Err(_) => println!("Could not parse number!"),
                        }
                    }
                }
                Err(_) => println!("Could not read from stdin!"),
            }
        }
    }

    cleaned.iter().map(Clone::clone).cloned().collect()
}

fn status(data: &[TrackingEvent]) {
    if let Some(event) = data.last() {
        let time = event.time(true).with_timezone(&Local);
        let active = event.is_start();
        let text = iif!(active, "Start", "End");
        if let Some(description) = event.description() {
            println!("Active: {}", active);
            println!("Description: {}", description,);
            println!(
                "{} Time: {:02}:{:02}:{:02}",
                text,
                time.hour(),
                time.minute(),
                time.second()
            );
        } else {
            println!("Active: {}", active);
            println!(
                "{} Time: {:02}:{:02}:{:02}",
                text,
                time.hour(),
                time.minute(),
                time.second()
            );
        }
        std::process::exit(iif!(active, 0, -1));
    } else {
        println!("No Events found!");
        std::process::exit(-1);
    }
}

fn to_human_readable<Tz: TimeZone>(
    prefix: &str,
    time: &DateTime<Tz>,
    description: Option<String>,
) -> String {
    let description = description
        .map(|d| format!(" \"{}\"", d))
        .unwrap_or_default();
    format!(
        "{} at {:04}-{:02}-{:02} {:02}:{:02}:{:02}{}",
        prefix,
        time.year(),
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second(),
        description,
    )
}

fn get_human_readable(data: &[TrackingEvent]) -> Vec<String> {
    data.iter()
        .map(|event| match event {
            TrackingEvent::Start(TrackingData { time, description }) => {
                to_human_readable("Start", &time.with_timezone(&Local), description.clone())
            }
            TrackingEvent::Stop(TrackingData { time, description }) => {
                to_human_readable("Stop ", &time.with_timezone(&Local), description.clone())
            }
        })
        .collect::<Vec<_>>()
}

fn export_human_readable(path: String, data: &[TrackingEvent]) {
    let lines = get_human_readable(data);
    std::fs::write(path, lines.join("\n")).expect("could not export file");
}

fn main() -> Result<()> {
    let Options { command, data_file, config_file } = Options::from_args();

    let settings = Settings::new(&config_file)?;

    let path = match data_file {
        Some(path) => path,
        None => shellexpand::full(&settings.data_file)?.parse()?,
    };
    let expanded_path = shellexpand::full(&path.to_string_lossy())
        .expect("could not expand path")
        .to_string();
    let mut data = read_data(&expanded_path).unwrap_or_default();

    let data_changed = match command.unwrap_or_default() {
        Command::Start { description, at } => {
            start_tracking(&settings, &mut data, description, at)?;
            true
        }
        Command::Stop { description, at } => {
            stop_tracking(&mut data, description, at)?;
            true
        }
        Command::Continue => {
            continue_tracking(&mut data);
            true
        }
        Command::List { filter } => {
            let data = filter_events(&data, &filter.from, &filter.to, &filter.filter)?;
            for s in get_human_readable(&data) {
                println!("{}", s);
            }
            false
        }
        Command::Path => {
            println!("{}", expanded_path);
            false
        }
        Command::Show {
            format,
            filter,
            include_seconds,
            plain,
            remaining,
        } => {
            show(
                &settings,
                &data,
                &filter,
                format,
                include_seconds,
                plain,
                remaining,
            )?;
            false
        }
        Command::Status => {
            status(&data);
            false
        }
        Command::Cleanup => {
            data = cleanup(&data);
            true
        }
        #[cfg(not(feature = "binary"))]
        Command::Export { path } => {
            let expanded_path = shellexpand::full(&path.to_string_lossy())
                .expect("could not expand path")
                .to_string();
            export_human_readable(expanded_path, &data);
            false
        }

        #[cfg(feature = "binary")]
        Command::Export {
            path,
            readable,
            pretty,
        } => {
            let expanded_path = shellexpand::full(&path.to_string_lossy())
                .expect("could not expand path")
                .to_string();
            if readable {
                export_human_readable(expanded_path, &data);
            } else {
                write_json_data(expanded_path, &data, pretty).expect("Could not write file");
            }
            false
        }
        #[cfg(feature = "binary")]
        Command::Import { path } => {
            data = read_json_data(path)?;
            true
        }
        #[allow(unreachable_patterns)]
        _ => unimplemented!(),
    };

    if data_changed {
        data.sort_by_key(|e| e.time(true));
        data.dedup();
        write_data(expanded_path, &data).expect("Could not write file!");
    }

    Ok(())
}

fn parse_date_time(s: &str) -> Result<DateTime<Utc>> {
    let from_time = |s: &str| NaiveTime::parse_from_str(s, "%H:%M:%S");
    let from_date_time = |s: &str| Local.datetime_from_str(s, "%Y-%m-%d %H:%M:%S");

    from_time(s)
        .or_else(|_| from_time(&format!("{}:0", s)))
        .or_else(|_| from_time(&format!("{}:0:0", s)))
        .map_err(Into::into)
        .and_then(|time| Local::today().and_time(time).context("invalid time"))
        .or_else(|_| {
            from_date_time(s)
                .or_else(|_| from_date_time(&format!("{}:0", s)))
                .or_else(|_| from_date_time(&format!("{}:0:0", s)))
        })
        .map(|date_time| date_time.with_timezone(&Utc))
        .map_err(Into::into)
}

fn parse_date_or_date_time(s: &str) -> Result<DateOrDateTime> {
    if let Ok(date) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        return Ok(date.into());
    }
    if let Ok(date_time) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
        return Ok(date_time.into());
    }

    parse_date_time(s).map(|date_time| date_time.with_timezone(&Local).naive_local().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_time() {
        assert_eq!(
            Local::now().date().and_hms(0, 0, 15).with_timezone(&Utc),
            parse_date_time("00:00:15").unwrap()
        );
        assert_eq!(
            Local::now().date().and_hms(0, 15, 0).with_timezone(&Utc),
            parse_date_time("00:15").unwrap()
        );
        assert_eq!(
            Local::now().date().and_hms(15, 0, 0).with_timezone(&Utc),
            parse_date_time("15").unwrap()
        );

        assert_eq!(
            Local.ymd(2021, 4, 1).and_hms(0, 0, 15).with_timezone(&Utc),
            parse_date_time("2021-04-01 00:00:15").unwrap()
        );
        assert_eq!(
            Local.ymd(2021, 4, 1).and_hms(0, 15, 0).with_timezone(&Utc),
            parse_date_time("2021-04-01 00:15").unwrap()
        );
        assert_eq!(
            Local.ymd(2021, 4, 1).and_hms(15, 0, 0).with_timezone(&Utc),
            parse_date_time("2021-04-01 15").unwrap()
        );
    }

    #[test]
    fn test_parse_date_or_date_time() {
        assert_eq!(
            DateOrDateTime::Date(NaiveDate::from_ymd(2020, 4, 1)),
            parse_date_or_date_time("2020-04-01").unwrap()
        );
        assert_eq!(
            DateOrDateTime::DateTime(NaiveDate::from_ymd(2020, 4, 1).and_hms(12, 15, 20)),
            parse_date_or_date_time("2020-04-01 12:15:20").unwrap()
        );
        assert_eq!(
            DateOrDateTime::DateTime(NaiveDate::from_ymd(2020, 4, 1).and_hms(12, 0, 0)),
            parse_date_or_date_time("2020-04-01 12").unwrap()
        );
    }
}
