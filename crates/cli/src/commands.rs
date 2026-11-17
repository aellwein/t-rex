//! Runs a parsed command against the server.

use std::{env, fs, process};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Datelike, Local, NaiveDate};
use dto::{
    AbsenceKind, AbsenceKindInput, BlockError, DurationUnit, Holiday, HolidayImport,
    HolidayImportResult, HolidayImportText, HolidayInput, Language, PendingStart, Project,
    ProjectInput, Settings, TimeBlock, TimeEntry, TimeEntryInput, TokenInfo, builtin_themes,
    duration_minutes, format_clock, parse_clock, parse_signed_hours, resolve_block,
};
use reqwest::Method;

use crate::{
    cli::{
        AbsenceCommand, Cli, Command, EntryCommand, EntryFields, EntrySort, HolidayCommand,
        PendingCommand, ProjectCommand, SettingKey, SettingsCommand, TokenCommand,
    },
    client::Client,
    config::{self, Config},
    login, output,
    template::{self, EntryRow, Template},
};

/// Runs the command. Errors are shown to the user as they are.
pub fn run(cli: Cli) -> Result<()> {
    let saved = config::load()?;
    let server = cli.server.clone().or_else(|| saved.server.clone());
    match cli.command {
        Command::Login { device } => sign_in(server, device),
        Command::Logout => sign_out(saved),
        command => {
            let token = saved
                .token
                .clone()
                .context("not signed in: run `t-rex login` first")?;
            let server = server.context("no server address: run `t-rex login --server <URL>`")?;
            let client = Client::new(&server, Some(token))?;
            dispatch(&client, command)
        }
    }
}

/// Runs a command that needs a server connection. `login` and `logout` are handled before this, because they
/// work without a saved token.
fn dispatch(client: &Client, command: Command) -> Result<()> {
    match command {
        Command::Whoami => whoami(client),
        Command::Project(command) => project(client, command),
        Command::Absence(command) => absence(client, command),
        Command::Entry(command) => entry(client, command),
        Command::Pending(command) => pending(client, command),
        Command::Holiday(command) => holiday(client, command),
        Command::Settings(command) => settings(client, command),
        Command::Token(command) => token(client, command),
        Command::Login { .. } | Command::Logout => {
            unreachable!("handled before the client is created")
        }
    }
}

// ---- account -------------------------------------------------------------------------------

/// Signs in through the browser and saves the token. See [`crate::login`] for the steps.
fn sign_in(server: Option<String>, device: Option<String>) -> Result<()> {
    let server = server.context("no server address: pass --server <URL> once, for example `t-rex login --server https://t-rex.example.com`")?;
    let device = device.unwrap_or_else(default_device);
    let created = login::sign_in(&server, &device)?;
    config::save(&Config {
        server: Some(server),
        token: Some(created.token),
        token_id: Some(created.id),
    })?;
    println!(
        "Signed in. Token \"{}\" saved to {}.",
        created.name,
        config::path()?.display()
    );
    Ok(())
}

/// Revokes the token on the server, if it is still known there, and forgets it locally. A failed revoke is a warning, not an error.
fn sign_out(saved: Config) -> Result<()> {
    if let (Some(server), Some(token), Some(id)) = (&saved.server, &saved.token, saved.token_id) {
        let client = Client::new(server, Some(token.clone()))?;
        if let Err(err) = client.delete(&format!("/api/v1/tokens/{id}")) {
            eprintln!("warning: could not revoke the token on the server: {err:#}");
        }
    }
    config::save(&Config {
        server: saved.server,
        token: None,
        token_id: None,
    })?;
    println!("Signed out. This computer no longer has a token.");
    Ok(())
}

/// The name of a new token: `<user>@<host>`, so the token list shows which computer it belongs to.
fn default_device() -> String {
    let user = env::var("USER").or_else(|_| env::var("USERNAME")).ok();
    device_name(user, host_name())
}

/// The name of this computer's token: `user@host`, with the parts that are known.
fn device_name(user: Option<String>, host: Option<String>) -> String {
    let user = user.map(|u| u.trim().to_string()).filter(|u| !u.is_empty());
    let host = host.map(|h| h.trim().to_string()).filter(|h| !h.is_empty());
    format!(
        "{}@{}",
        user.unwrap_or_else(|| "user".to_string()),
        host.unwrap_or_else(|| "computer".to_string())
    )
}

/// The host name, from the environment or the `hostname` command.
fn host_name() -> Option<String> {
    env::var("HOSTNAME").ok().or_else(|| {
        let output = process::Command::new("hostname").output().ok()?;
        String::from_utf8(output.stdout).ok()
    })
}

#[cfg(test)]
mod device_tests {
    use super::device_name;

    #[test]
    fn device_is_user_at_host() {
        assert_eq!(
            device_name(Some("alex".into()), Some("laptop\n".into())),
            "alex@laptop"
        );
    }

    #[test]
    fn missing_parts_fall_back() {
        assert_eq!(device_name(None, Some("laptop".into())), "user@laptop");
        assert_eq!(device_name(Some(" ".into()), None), "user@computer");
    }
}

/// Prints the signed-in account and the server version.
fn whoami(client: &Client) -> Result<()> {
    let me: dto::Me = client.get("/api/v1/me")?;
    let version: dto::VersionInfo = client.get("/api/v1/version")?;
    let who = me
        .name
        .or(me.email)
        .unwrap_or_else(|| "unknown account".into());
    println!("Signed in as {who}");
    println!("Server version {}", version.version);
    Ok(())
}

// ---- projects and absence kinds ------------------------------------------------------------

/// Runs a project command. Projects are referred to by id or by name.
fn project(client: &Client, command: ProjectCommand) -> Result<()> {
    match command {
        ProjectCommand::List => {
            let projects: Vec<Project> = client.get_all("/api/v1/projects")?;
            let rows: Vec<Vec<String>> = projects
                .iter()
                .map(|p| {
                    vec![
                        p.id.to_string(),
                        p.name.clone(),
                        p.color.clone(),
                        status(p.archived),
                    ]
                })
                .collect();
            output::table(&["ID", "NAME", "COLOR", "STATUS"], &rows);
        }
        ProjectCommand::Add {
            name,
            color,
            archived,
        } => {
            let input = ProjectInput {
                name,
                color: color.unwrap_or_else(|| dto::DEFAULT_COLOR.to_string()),
                archived,
            };
            let created: Project = client.send(Method::POST, "/api/v1/projects", &input)?;
            println!("Added project {} ({}).", created.name, created.id);
        }
        ProjectCommand::Edit {
            project,
            name,
            color,
            archive,
            unarchive,
        } => {
            let id = project_id(client, &project)?;
            let current: Project = client.get(&format!("/api/v1/projects/{id}"))?;
            let input = ProjectInput {
                name: name.unwrap_or(current.name),
                color: color.unwrap_or(current.color),
                archived: if archive {
                    true
                } else if unarchive {
                    false
                } else {
                    current.archived
                },
            };
            let updated: Project =
                client.send(Method::PUT, &format!("/api/v1/projects/{id}"), &input)?;
            println!("Saved project {} ({}).", updated.name, updated.id);
        }
        ProjectCommand::Remove { project } => {
            let id = project_id(client, &project)?;
            client.delete(&format!("/api/v1/projects/{id}"))?;
            println!("Removed project {id}.");
        }
    }
    Ok(())
}

/// Runs an absence kind command. Kinds are referred to by id or by name.
fn absence(client: &Client, command: AbsenceCommand) -> Result<()> {
    match command {
        AbsenceCommand::List => {
            let kinds: Vec<AbsenceKind> = client.get_all("/api/v1/absence-kinds")?;
            let rows: Vec<Vec<String>> = kinds
                .iter()
                .map(|k| {
                    vec![
                        k.id.to_string(),
                        k.name.clone(),
                        k.color.clone(),
                        if k.paid { "paid" } else { "unpaid" }.into(),
                    ]
                })
                .collect();
            output::table(&["ID", "NAME", "COLOR", "PAY"], &rows);
        }
        AbsenceCommand::Add {
            name,
            color,
            unpaid,
        } => {
            let input = AbsenceKindInput {
                name,
                color: color.unwrap_or_else(|| dto::DEFAULT_COLOR.to_string()),
                paid: !unpaid,
            };
            let created: AbsenceKind =
                client.send(Method::POST, "/api/v1/absence-kinds", &input)?;
            println!("Added absence kind {} ({}).", created.name, created.id);
        }
        AbsenceCommand::Edit {
            absence,
            name,
            color,
            paid,
            unpaid,
        } => {
            let id = absence_id(client, &absence)?;
            let current: AbsenceKind = client.get(&format!("/api/v1/absence-kinds/{id}"))?;
            let input = AbsenceKindInput {
                name: name.unwrap_or(current.name),
                color: color.unwrap_or(current.color),
                paid: if paid {
                    true
                } else if unpaid {
                    false
                } else {
                    current.paid
                },
            };
            let updated: AbsenceKind =
                client.send(Method::PUT, &format!("/api/v1/absence-kinds/{id}"), &input)?;
            println!("Saved absence kind {} ({}).", updated.name, updated.id);
        }
        AbsenceCommand::Remove { absence } => {
            let id = absence_id(client, &absence)?;
            client.delete(&format!("/api/v1/absence-kinds/{id}"))?;
            println!("Removed absence kind {id}.");
        }
    }
    Ok(())
}

/// The word shown in project and absence lists for the archived flag.
fn status(archived: bool) -> String {
    if archived { "archived" } else { "active" }.into()
}

/// Finds a project by id or by name (case-insensitive).
fn project_id(client: &Client, reference: &str) -> Result<i64> {
    let projects: Vec<Project> = client.get_all("/api/v1/projects")?;
    find_id(
        reference,
        projects.iter().map(|p| (p.id, p.name.as_str())),
        "project",
    )
}

/// Finds an absence kind by id or by name (case-insensitive).
fn absence_id(client: &Client, reference: &str) -> Result<i64> {
    let kinds: Vec<AbsenceKind> = client.get_all("/api/v1/absence-kinds")?;
    find_id(
        reference,
        kinds.iter().map(|k| (k.id, k.name.as_str())),
        "absence kind",
    )
}

/// Formats an RFC 3339 timestamp as local date and time. Falls back to the text if it does not parse.
fn local_time(text: &str) -> String {
    DateTime::parse_from_rfc3339(text)
        .map(|time| {
            time.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| text.to_string())
}

/// A number that matches an id wins. Otherwise the name is matched in any case; an ambiguous or unknown name is an error.
fn find_id<'a>(
    reference: &str,
    items: impl Iterator<Item = (i64, &'a str)>,
    kind: &str,
) -> Result<i64> {
    let items: Vec<(i64, &str)> = items.collect();
    if let Ok(id) = reference.trim().parse::<i64>()
        && items.iter().any(|(item, _)| *item == id)
    {
        return Ok(id);
    }
    let wanted = reference.trim().to_lowercase();
    let matches: Vec<i64> = items
        .iter()
        .filter(|(_, name)| name.to_lowercase() == wanted)
        .map(|(id, _)| *id)
        .collect();
    match matches.as_slice() {
        [id] => Ok(*id),
        [] => bail!("no {kind} named or numbered \"{reference}\""),
        _ => bail!("more than one {kind} is called \"{reference}\"; use its id"),
    }
}

// ---- time records --------------------------------------------------------------------------

/// Runs a time record command.
fn entry(client: &Client, command: EntryCommand) -> Result<()> {
    match command {
        EntryCommand::List {
            from,
            to,
            sort,
            reverse,
            limit,
            offset,
            all,
            format,
        } => list_entries(
            client,
            ListOptions {
                from,
                to,
                sort,
                reverse,
                page: (!all).then_some((limit as usize, offset as usize)),
                format,
            },
        ),
        EntryCommand::Add(fields) => add_entry(client, fields),
        EntryCommand::Edit { id, fields } => edit_entry(client, id, fields),
        EntryCommand::Remove { id } => {
            client.delete(&format!("/api/v1/time-entries/{id}"))?;
            println!("Removed time record {id}.");
            Ok(())
        }
    }
}

/// Orders entries by the chosen column, then by date, start and id so the order is stable.
fn sort_entries(entries: &mut [(TimeEntry, String)], sort: EntrySort, reverse: bool) {
    let by_day = |e: &TimeEntry| (e.date, e.start_minute, e.id);
    entries.sort_by(|(a, a_what), (b, b_what)| {
        let order = match sort {
            EntrySort::Date => by_day(a).cmp(&by_day(b)),
            EntrySort::What => a_what
                .to_lowercase()
                .cmp(&b_what.to_lowercase())
                .then_with(|| by_day(a).cmp(&by_day(b))),
            EntrySort::Hours => a
                .minutes
                .cmp(&b.minutes)
                .then_with(|| by_day(a).cmp(&by_day(b))),
            EntrySort::Id => a.id.cmp(&b.id),
        };
        if reverse { order.reverse() } else { order }
    });
}

/// What `entry list` shows: the period, the order, the page and the output format.
struct ListOptions {
    from: Option<String>,
    to: Option<String>,
    sort: EntrySort,
    reverse: bool,
    /// `(limit, offset)` of the page, or `None` for every record.
    page: Option<(usize, usize)>,
    format: Option<String>,
}

/// The default `entry list` output: one aligned table with the common columns.
const DEFAULT_FORMAT: &str =
    "table {{.ID}}\t{{.Date}}\t{{.Start}}\t{{.End}}\t{{.Hours}}\t{{.Break}}\t{{.What}}\t{{.Note}}";

/// One time record as the output needs it: times as `hh:mm`, the project or absence by name.
fn entry_row(entry: &TimeEntry, what: &str) -> EntryRow {
    let end = entry
        .start_minute
        .map(|start| format_clock((start + entry.minutes + entry.break_minutes) % 1440))
        .unwrap_or_else(|| "-".into());
    EntryRow {
        id: entry.id,
        date: entry.date.to_string(),
        start: entry
            .start_minute
            .map(format_clock)
            .unwrap_or_else(|| "-".into()),
        end,
        hours: format_clock(entry.minutes),
        break_time: format_clock(entry.break_minutes),
        what: what.to_string(),
        note: entry.note.clone(),
    }
}

/// The `[first, last)` range of the sorted records that a page shows. Without a page, all of them.
fn page_bounds(total: usize, page: Option<(usize, usize)>) -> (usize, usize) {
    match page {
        None => (0, total),
        Some((limit, offset)) => {
            let first = offset.min(total);
            (first, first.saturating_add(limit).min(total))
        }
    }
}

/// Fetches the records of the period, sorts them, cuts the page and prints them. Records are fetched page by page from the server, so the whole period is always sorted correctly before it is cut.
fn list_entries(client: &Client, options: ListOptions) -> Result<()> {
    // Check the format before the server is asked for anything.
    let json = options.format.as_deref() == Some("json");
    let template = match (&options.format, json) {
        (_, true) => None,
        (Some(text), false) => Some(Template::parse(text)?),
        (None, false) => Some(Template::parse(DEFAULT_FORMAT)?),
    };

    let today = Local::now().date_naive();
    let from = match options.from {
        Some(text) => parse_date(&text)?,
        None => today.with_day(1).context("invalid date")?,
    };
    let to = match options.to {
        Some(text) => parse_date(&text)?,
        None => last_day_of_month(from),
    };
    let entries: Vec<TimeEntry> =
        client.get_all(&format!("/api/v1/time-entries?from={from}&to={to}"))?;
    let projects: Vec<Project> = client.get_all("/api/v1/projects")?;
    let kinds: Vec<AbsenceKind> = client.get_all("/api/v1/absence-kinds")?;

    let mut named: Vec<(TimeEntry, String)> = entries
        .into_iter()
        .map(|e| {
            let what = match (e.project_id, e.absence_kind_id) {
                (Some(id), _) => projects.iter().find(|p| p.id == id).map(|p| p.name.clone()),
                (None, Some(id)) => kinds.iter().find(|k| k.id == id).map(|k| k.name.clone()),
                _ => None,
            };
            let what = what.unwrap_or_else(|| "?".into());
            (e, what)
        })
        .collect();
    sort_entries(&mut named, options.sort, options.reverse);

    // The page is cut from the sorted list, so `--sort hours --limit 10` gives the ten longest.
    let total = named.len();
    let (first, last) = page_bounds(total, options.page);
    let rows: Vec<EntryRow> = named[first..last]
        .iter()
        .map(|(entry, what)| entry_row(entry, what))
        .collect();

    match template {
        None => println!(
            "{}",
            serde_json::to_string_pretty(&template::to_json(&rows))?
        ),
        Some(template) => template.print(&rows),
    }
    // The paging hints go to stderr, so piped output stays clean.
    if options.page.is_some() && rows.is_empty() && total > 0 {
        eprintln!("No records at offset {first}; {total} in total.");
    } else if options.page.is_some() && last < total {
        eprintln!(
            "Showing {}-{} of {total}. Next page: --offset {last}, or --all.",
            first + 1,
            last
        );
    }
    Ok(())
}

/// Adds a time record. The start defaults to the pending start of the day, if there is one.
fn add_entry(client: &Client, fields: EntryFields) -> Result<()> {
    let date = match &fields.date {
        Some(text) => parse_date(text)?,
        None => Local::now().date_naive(),
    };
    let settings: Settings = client.get("/api/v1/settings")?;
    let pending: Option<PendingStart> = client.get("/api/v1/pending-start")?;
    let pending_start = pending.filter(|p| p.date == date).map(|p| p.start_minute);

    let block = entry_block(&fields, None, settings.break_minutes, pending_start)?;
    let (project_id, absence_kind_id) = entry_refs(client, &fields, None)?;
    let input = TimeEntryInput {
        date,
        start_minute: block.start_minute,
        minutes: block.minutes,
        break_minutes: block.break_minutes,
        project_id,
        absence_kind_id,
        note: fields.note.unwrap_or_default(),
    };
    let created: TimeEntry = client.send(Method::POST, "/api/v1/time-entries", &input)?;
    println!("Added time record {} for {}.", created.id, created.date);
    Ok(())
}

/// Changes a time record. Only the fields that are given change; the rest keep the saved values.
fn edit_entry(client: &Client, id: i64, fields: EntryFields) -> Result<()> {
    let existing: TimeEntry = client.get(&format!("/api/v1/time-entries/{id}"))?;
    let date = match &fields.date {
        Some(text) => parse_date(text)?,
        None => existing.date,
    };
    let block = entry_block(&fields, Some(&existing), 0, None)?;
    let (project_id, absence_kind_id) = entry_refs(client, &fields, Some(&existing))?;
    let input = TimeEntryInput {
        date,
        start_minute: block.start_minute,
        minutes: block.minutes,
        break_minutes: block.break_minutes,
        project_id,
        absence_kind_id,
        note: fields.note.unwrap_or(existing.note),
    };
    let updated: TimeEntry =
        client.send(Method::PUT, &format!("/api/v1/time-entries/{id}"), &input)?;
    println!("Saved time record {} for {}.", updated.id, updated.date);
    Ok(())
}

/// Works out start, work time and break. Two of start, end and duration are enough; for an edit,
/// anything not given keeps its saved value.
fn entry_block(
    fields: &EntryFields,
    existing: Option<&TimeEntry>,
    default_break: u32,
    pending_start: Option<u32>,
) -> Result<TimeBlock> {
    let start = match (&fields.start, existing, pending_start) {
        (Some(start), _, _) => start.clone(),
        (None, Some(record), _) => match record.start_minute {
            Some(minute) => format_clock(minute),
            None => bail!("this record has no start time; give --start"),
        },
        (None, None, Some(minute)) => format_clock(minute),
        (None, None, None) => {
            bail!("give --start, or start the day first with `t-rex pending start`")
        }
    };
    let break_minutes = match (&fields.break_time, existing) {
        (Some(text), _) => parse_break(text)?,
        (None, Some(record)) => record.break_minutes,
        (None, None) => default_break,
    };
    let (amount, unit) = match (&fields.duration, &fields.end, existing) {
        (Some(text), _, _) => parse_duration(text),
        (None, Some(_), _) => (String::new(), DurationUnit::Hours),
        (None, None, Some(record)) => (record.minutes.to_string(), DurationUnit::Minutes),
        (None, None, None) => (String::new(), DurationUnit::Hours),
    };
    let end = fields.end.clone().unwrap_or_default();
    resolve_block(&start, &end, &amount, unit, break_minutes).map_err(block_error)
}

/// Project or absence kind of a record. For an edit, a given reference replaces the saved one.
fn entry_refs(
    client: &Client,
    fields: &EntryFields,
    existing: Option<&TimeEntry>,
) -> Result<(Option<i64>, Option<i64>)> {
    match (&fields.project, &fields.absence) {
        (Some(project), _) => Ok((Some(project_id(client, project)?), None)),
        (None, Some(absence)) => Ok((None, Some(absence_id(client, absence)?))),
        (None, None) => match existing {
            Some(record) => Ok((record.project_id, record.absence_kind_id)),
            None => bail!("give --project or --absence"),
        },
    }
}

/// `7.5`, `7h` (hours) or `450m` (minutes) as the amount and its unit. Validated later.
fn parse_duration(text: &str) -> (String, DurationUnit) {
    let text = text.trim();
    if let Some(number) = text.strip_suffix('m') {
        (number.to_string(), DurationUnit::Minutes)
    } else if let Some(number) = text.strip_suffix('h') {
        (number.to_string(), DurationUnit::Hours)
    } else {
        (text.to_string(), DurationUnit::Hours)
    }
}

/// A break as minutes: `45` or `45m` for minutes, `1h` for hours.
fn parse_break(text: &str) -> Result<u32> {
    let text = text.trim();
    let (amount, unit) = match text.strip_suffix('h') {
        Some(hours) => (hours, DurationUnit::Hours),
        None => (
            text.strip_suffix('m').unwrap_or(text),
            DurationUnit::Minutes,
        ),
    };
    match duration_minutes(amount, unit) {
        Some(minutes) if minutes <= 1440 => Ok(minutes),
        _ => bail!("the break \"{text}\" is not a valid duration"),
    }
}

/// The message for a time record that cannot be built, in the words of the command line.
fn block_error(err: BlockError) -> anyhow::Error {
    anyhow!(match err {
        BlockError::StartMissing => "give --start",
        BlockError::StartInvalid => "the start must be a time like 09:30",
        BlockError::EndInvalid => "the end must be a time like 17:00",
        BlockError::DurationMissing => "give --end or --duration",
        BlockError::DurationInvalid => "the duration must be like 7.5, 7h or 450m",
        BlockError::EndNotAfterStart => "the end must be after the start",
        BlockError::DurationMismatch => "start, end and duration do not agree; drop one of them",
        BlockError::BreakTooLong => "the break is longer than the record",
        BlockError::PastMidnight => "the record must end on the same day",
    })
}

// ---- pending start -------------------------------------------------------------------------

/// Runs a pending start command: show, start or clear.
fn pending(client: &Client, command: PendingCommand) -> Result<()> {
    match command {
        PendingCommand::Show => {
            let pending: Option<PendingStart> = client.get("/api/v1/pending-start")?;
            match pending {
                Some(p) => println!("Started at {} on {}.", format_clock(p.start_minute), p.date),
                None => println!("No pending start."),
            }
        }
        PendingCommand::Start { time } => {
            let start_minute = match time {
                Some(text) => parse_clock(&text).context("the time must be HH:MM")?,
                None => {
                    let now = Local::now();
                    now.hour_minute()
                }
            };
            let pending = PendingStart {
                date: Local::now().date_naive(),
                start_minute,
            };
            let saved: PendingStart =
                client.send(Method::PUT, "/api/v1/pending-start", &pending)?;
            println!(
                "Started at {} on {}.",
                format_clock(saved.start_minute),
                saved.date
            );
        }
        PendingCommand::Clear => {
            client.delete("/api/v1/pending-start")?;
            println!("Cleared the pending start.");
        }
    }
    Ok(())
}

/// A time of day as minutes after midnight, so that clock times and durations use the same format.
trait HourMinute {
    /// Minutes after midnight of this time, in local time.
    fn hour_minute(&self) -> u32;
}

impl HourMinute for chrono::DateTime<Local> {
    fn hour_minute(&self) -> u32 {
        use chrono::Timelike;
        self.hour() * 60 + self.minute()
    }
}

// ---- holidays ------------------------------------------------------------------------------

/// Runs a holiday command.
fn holiday(client: &Client, command: HolidayCommand) -> Result<()> {
    match command {
        HolidayCommand::List { year } => {
            let year = year.unwrap_or_else(|| Local::now().year());
            let holidays: Vec<Holiday> = client.get_all("/api/v1/holidays")?;
            let rows: Vec<Vec<String>> = holidays
                .iter()
                .filter(|h| h.date.year() == year)
                .map(|h| vec![h.id.to_string(), h.date.to_string(), h.name.clone()])
                .collect();
            output::table(&["ID", "DATE", "NAME"], &rows);
        }
        HolidayCommand::Add { date, name } => {
            let input = HolidayInput {
                date: parse_date(&date)?,
                name,
            };
            let created: Holiday = client.send(Method::POST, "/api/v1/holidays", &input)?;
            println!(
                "Added holiday {} on {} ({}).",
                created.name, created.date, created.id
            );
        }
        HolidayCommand::Edit { id, date, name } => {
            let current: Holiday = client.get(&format!("/api/v1/holidays/{id}"))?;
            let input = HolidayInput {
                date: match date {
                    Some(text) => parse_date(&text)?,
                    None => current.date,
                },
                name: name.unwrap_or(current.name),
            };
            let updated: Holiday =
                client.send(Method::PUT, &format!("/api/v1/holidays/{id}"), &input)?;
            println!(
                "Saved holiday {} on {} ({}).",
                updated.name, updated.date, updated.id
            );
        }
        HolidayCommand::Remove { id } => {
            client.delete(&format!("/api/v1/holidays/{id}"))?;
            println!("Removed holiday {id}.");
        }
        HolidayCommand::Import { url } => {
            let result: HolidayImportResult = client.send(
                Method::POST,
                "/api/v1/holidays/import",
                &HolidayImport { url },
            )?;
            print_import(result);
        }
        HolidayCommand::ImportFile { path } => {
            let text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let result: HolidayImportResult = client.send(
                Method::POST,
                "/api/v1/holidays/import-text",
                &HolidayImportText { text },
            )?;
            print_import(result);
        }
    }
    Ok(())
}

/// Prints how many holidays an import added and how many days already had one.
fn print_import(result: HolidayImportResult) {
    println!(
        "Added {} holiday(s). {} day(s) already had one and were kept.",
        result.added, result.existing
    );
}

// ---- settings and tokens -------------------------------------------------------------------

/// Shows the settings, or saves one setting after checking it locally.
fn settings(client: &Client, command: SettingsCommand) -> Result<()> {
    match command {
        SettingsCommand::Show => {
            let s: Settings = client.get("/api/v1/settings")?;
            println!("daily-hours  {}", format_clock(s.daily_minutes));
            println!("break        {}", format_clock(s.break_minutes));
            println!("language     {}", s.language.as_str());
            println!("theme        {}", s.theme);
            println!("show-email   {}", s.show_email);
            println!(
                "carry-over   {}",
                dto::format_signed_hours(s.carry_over_minutes)
            );
        }
        SettingsCommand::Set { key, value } => {
            let mut s: Settings = client.get("/api/v1/settings")?;
            apply_setting(&mut s, key, &value)?;
            let saved: Settings = client.send(Method::PUT, "/api/v1/settings", &s)?;
            println!("Saved. {key:?} is now {}.", value_of(&saved, key));
        }
    }
    Ok(())
}

/// Parses `value` for the given setting and writes it into `s`. Errors describe the expected form.
fn apply_setting(s: &mut Settings, key: SettingKey, value: &str) -> Result<()> {
    match key {
        SettingKey::DailyHours => {
            let (amount, unit) = parse_duration(value);
            s.daily_minutes = duration_minutes(&amount, unit)
                .filter(|minutes| *minutes <= 1440)
                .context("daily-hours must be between 0 and 24, like 7.5 or 7h30")?;
        }
        SettingKey::Break => s.break_minutes = parse_break(value)?,
        SettingKey::Language => {
            s.language = Language::parse(value).with_context(|| {
                format!(
                    "unknown language \"{value}\"; use one of: {}",
                    language_codes()
                )
            })?;
        }
        SettingKey::Theme => {
            let known =
                value == dto::theme::SYSTEM || builtin_themes().iter().any(|t| t.id == value);
            if !known {
                bail!("unknown theme \"{value}\"; see the theme list in the web app");
            }
            s.theme = value.to_string();
        }
        SettingKey::ShowEmail => {
            s.show_email = match value.to_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => true,
                "false" | "no" | "off" | "0" => false,
                _ => bail!("show-email must be true or false"),
            };
        }
        SettingKey::CarryOver => {
            s.carry_over_minutes = parse_signed_hours(value)
                .context("carry-over must be signed hours, like -1:30 or +12:00")?;
        }
    }
    Ok(())
}

/// The value of one setting as the client shows it.
fn value_of(s: &Settings, key: SettingKey) -> String {
    match key {
        SettingKey::DailyHours => format_clock(s.daily_minutes),
        SettingKey::Break => format_clock(s.break_minutes),
        SettingKey::Language => s.language.as_str().to_string(),
        SettingKey::Theme => s.theme.clone(),
        SettingKey::ShowEmail => s.show_email.to_string(),
        SettingKey::CarryOver => dto::format_signed_hours(s.carry_over_minutes),
    }
}

/// The language codes the settings accept, separated by commas, for the help text.
fn language_codes() -> String {
    [
        Language::System,
        Language::En,
        Language::De,
        Language::Ru,
        Language::Fr,
        Language::Es,
        Language::It,
        Language::Pt,
        Language::Nl,
        Language::Pl,
        Language::Zh,
        Language::Ja,
    ]
    .iter()
    .map(|language| language.as_str())
    .collect::<Vec<_>>()
    .join(", ")
}

/// Runs a token command. Listing shows id, name, creation and expiry; the secret is never shown again.
fn token(client: &Client, command: TokenCommand) -> Result<()> {
    match command {
        TokenCommand::List => {
            let tokens: Vec<TokenInfo> = client.get_all("/api/v1/tokens")?;
            let rows: Vec<Vec<String>> = tokens
                .iter()
                .map(|t| {
                    vec![
                        t.id.to_string(),
                        t.name.clone(),
                        local_time(&t.created_at),
                        local_time(&t.expires_at),
                    ]
                })
                .collect();
            output::table(&["ID", "NAME", "CREATED", "EXPIRES"], &rows);
        }
        TokenCommand::Remove { id } => {
            client.delete(&format!("/api/v1/tokens/{id}"))?;
            let mut saved = config::load()?;
            if saved.token_id == Some(id) {
                saved.token = None;
                saved.token_id = None;
                config::save(&saved)?;
                println!("Removed token {id}. This computer is signed out too.");
            } else {
                println!("Removed token {id}.");
            }
        }
    }
    Ok(())
}

// ---- date helpers --------------------------------------------------------------------------

/// Parses a `YYYY-MM-DD` date, with an error that shows the expected form.
fn parse_date(text: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d")
        .with_context(|| format!("\"{text}\" is not a date; use YYYY-MM-DD"))
}

/// The last day of the month that `day` is in.
fn last_day_of_month(day: NaiveDate) -> NaiveDate {
    let (year, month) = if day.month() == 12 {
        (day.year() + 1, 1)
    } else {
        (day.year(), day.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|first| first.pred_opt())
        .unwrap_or(day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> EntryFields {
        EntryFields::default()
    }

    fn saved_record() -> TimeEntry {
        TimeEntry {
            id: 7,
            date: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            start_minute: Some(9 * 60),
            minutes: 450,
            break_minutes: 45,
            project_id: Some(1),
            absence_kind_id: None,
            note: "work".into(),
        }
    }

    #[test]
    fn durations_are_hours_unless_marked_as_minutes() {
        assert_eq!(parse_duration("7.5"), ("7.5".into(), DurationUnit::Hours));
        assert_eq!(parse_duration("7h"), ("7".into(), DurationUnit::Hours));
        assert_eq!(
            parse_duration("450m"),
            ("450".into(), DurationUnit::Minutes)
        );
    }

    #[test]
    fn a_break_is_minutes_by_default() {
        assert_eq!(parse_break("45").unwrap(), 45);
        assert_eq!(parse_break("1h").unwrap(), 60);
        assert!(parse_break("nonsense").is_err());
    }

    #[test]
    fn add_derives_the_duration_from_start_and_end() {
        let mut f = fields();
        f.start = Some("09:00".into());
        f.end = Some("17:00".into());
        let block = entry_block(&f, None, 45, None).unwrap();
        // 17:00 - 09:00 = 8 h, of which 45 min is break, so 7 h 15 min of work.
        assert_eq!(
            (block.start_minute, block.minutes, block.break_minutes),
            (540, 435, 45)
        );
    }

    #[test]
    fn add_uses_the_pending_start_when_no_start_is_given() {
        let mut f = fields();
        f.duration = Some("8".into());
        let block = entry_block(&f, None, 30, Some(8 * 60)).unwrap();
        assert_eq!(
            (block.start_minute, block.minutes, block.break_minutes),
            (480, 480, 30)
        );
    }

    #[test]
    fn add_without_a_start_explains_how_to_get_one() {
        let mut f = fields();
        f.duration = Some("8".into());
        let message = format!("{:#}", entry_block(&f, None, 0, None).unwrap_err());
        assert!(message.contains("pending start"), "{message}");
    }

    #[test]
    fn edit_keeps_the_saved_values_it_is_not_given() {
        let record = saved_record();
        let mut f = fields();
        f.end = Some("18:00".into());
        let block = entry_block(&f, Some(&record), 0, None).unwrap();
        assert_eq!(block.start_minute, 9 * 60);
        assert_eq!(block.break_minutes, 45);
        // 18:00 - 09:00 - 45 min break = 8 h 15 min of work
        assert_eq!(block.minutes, 495);
    }

    #[test]
    fn find_id_accepts_an_id_or_a_unique_name() {
        let items = [(1, "Website"), (2, "Internal")];
        assert_eq!(find_id("2", items.iter().copied(), "project").unwrap(), 2);
        assert_eq!(
            find_id("website", items.iter().copied(), "project").unwrap(),
            1
        );
        assert!(find_id("Missing", items.iter().copied(), "project").is_err());
    }

    #[test]
    fn find_id_refuses_an_ambiguous_name() {
        let items = [(1, "Website"), (2, "website")];
        let message = format!(
            "{:#}",
            find_id("WEBSITE", items.iter().copied(), "project").unwrap_err()
        );
        assert!(message.contains("more than one"), "{message}");
    }

    #[test]
    fn last_day_handles_february_and_december() {
        let feb = NaiveDate::from_ymd_opt(2028, 2, 1).unwrap();
        assert_eq!(
            last_day_of_month(feb),
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
        );
        let dec = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        assert_eq!(
            last_day_of_month(dec),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
        );
    }

    #[test]
    fn settings_are_validated_before_they_are_sent() {
        let mut s = Settings::default();
        apply_setting(&mut s, SettingKey::DailyHours, "7.5").unwrap();
        assert_eq!(s.daily_minutes, 450);
        assert!(apply_setting(&mut s, SettingKey::Language, "xx").is_err());
        assert!(apply_setting(&mut s, SettingKey::Theme, "no-such-theme").is_err());
        apply_setting(&mut s, SettingKey::CarryOver, "-1:30").unwrap();
        assert_eq!(s.carry_over_minutes, -90);
    }
}

#[cfg(test)]
mod sort_tests {
    use super::*;
    use chrono::NaiveDate;

    fn entry(id: i64, day: u32, start: Option<u32>, minutes: u32) -> TimeEntry {
        TimeEntry {
            id,
            date: NaiveDate::from_ymd_opt(2026, 11, day).unwrap(),
            start_minute: start,
            minutes,
            break_minutes: 0,
            project_id: None,
            absence_kind_id: None,
            note: String::new(),
        }
    }

    #[test]
    fn a_page_is_cut_from_the_sorted_list() {
        assert_eq!(page_bounds(25, Some((10, 0))), (0, 10));
        assert_eq!(page_bounds(25, Some((10, 10))), (10, 20));
        assert_eq!(page_bounds(25, Some((10, 20))), (20, 25));
        assert_eq!(page_bounds(25, Some((10, 30))), (25, 25));
        assert_eq!(page_bounds(25, None), (0, 25));
    }

    #[test]
    fn an_entry_row_shows_end_and_times_as_hours_and_minutes() {
        let mut e = entry(3, 2, Some(9 * 60), 450);
        e.break_minutes = 45;
        let row = entry_row(&e, "Website");
        assert_eq!(row.start, "09:00");
        assert_eq!(row.end, "17:15");
        assert_eq!(row.hours, "07:30");
        assert_eq!(row.break_time, "00:45");
        assert_eq!(row.what, "Website");
    }

    #[test]
    fn a_record_without_start_has_no_end() {
        let row = entry_row(&entry(3, 2, None, 60), "Web");
        assert_eq!(row.start, "-");
        assert_eq!(row.end, "-");
    }

    fn ids(entries: &[(TimeEntry, String)]) -> Vec<i64> {
        entries.iter().map(|(e, _)| e.id).collect()
    }

    #[test]
    fn date_is_the_default_order_with_start_as_tie_breaker() {
        let mut rows = vec![
            (entry(3, 5, Some(600), 60), "B".to_string()),
            (entry(1, 2, Some(540), 60), "A".to_string()),
            (entry(2, 2, Some(480), 60), "A".to_string()),
        ];
        sort_entries(&mut rows, EntrySort::Date, false);
        assert_eq!(ids(&rows), vec![2, 1, 3]);
    }

    #[test]
    fn reverse_flips_the_order() {
        let mut rows = vec![
            (entry(1, 2, Some(540), 60), "A".to_string()),
            (entry(2, 5, Some(480), 60), "A".to_string()),
        ];
        sort_entries(&mut rows, EntrySort::Date, true);
        assert_eq!(ids(&rows), vec![2, 1]);
    }

    #[test]
    fn what_sorts_by_name_ignoring_case_then_date() {
        let mut rows = vec![
            (entry(1, 3, None, 60), "vacation".to_string()),
            (entry(2, 9, None, 60), "Alpha".to_string()),
            (entry(3, 1, None, 60), "alpha".to_string()),
        ];
        sort_entries(&mut rows, EntrySort::What, false);
        assert_eq!(ids(&rows), vec![3, 2, 1]);
    }

    #[test]
    fn hours_sort_by_booked_minutes() {
        let mut rows = vec![
            (entry(1, 1, None, 480), "A".to_string()),
            (entry(2, 2, None, 240), "A".to_string()),
        ];
        sort_entries(&mut rows, EntrySort::Hours, false);
        assert_eq!(ids(&rows), vec![2, 1]);
    }

    #[test]
    fn id_sorts_by_record_id() {
        let mut rows = vec![
            (entry(7, 1, None, 60), "A".to_string()),
            (entry(4, 9, None, 60), "A".to_string()),
        ];
        sort_entries(&mut rows, EntrySort::Id, false);
        assert_eq!(ids(&rows), vec![4, 7]);
    }
}
