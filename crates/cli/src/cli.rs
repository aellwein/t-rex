//! Command line definitions. Commands read like `t-rex <noun> <verb> ...`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::engine::ArgValueCompleter;

use crate::complete;

/// Track your working time from the terminal.
#[derive(Debug, Parser)]
#[command(
    name = "t-rex",
    version,
    about,
    arg_required_else_help = true,
    propagate_version = true
)]
/// Command line arguments of `t-rex`. The global `--server` option, or the `T_REX_SERVER` variable, overrides the saved server address.
pub struct Cli {
    /// Address of the t-rex server. Saved by `login`.
    #[arg(long, global = true, env = "T_REX_SERVER", value_name = "URL")]
    pub server: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
/// The top-level commands. Each one is a noun that groups its verbs, such as `entry list`, or a single action such as `login`.
pub enum Command {
    /// Sign in through the browser and save the token on this computer.
    Login {
        /// Name shown for this token in the token list. Defaults to `user@host`.
        #[arg(long, value_name = "NAME")]
        device: Option<String>,
    },
    /// Revoke the token of this computer and forget it.
    Logout,
    /// Show the signed-in account and the server version.
    Whoami,
    /// Projects you record time on.
    #[command(subcommand)]
    Project(ProjectCommand),
    /// Absence kinds, such as vacation or sick leave.
    #[command(subcommand)]
    Absence(AbsenceCommand),
    /// Time records.
    #[command(subcommand)]
    Entry(EntryCommand),
    /// Start of work that has no time record yet.
    #[command(subcommand)]
    Pending(PendingCommand),
    /// Public holidays and days without working time.
    #[command(subcommand)]
    Holiday(HolidayCommand),
    /// Your settings.
    #[command(subcommand)]
    Settings(SettingsCommand),
    /// Command line tokens of this account.
    #[command(subcommand)]
    Token(TokenCommand),
}

#[derive(Debug, Subcommand)]
/// Project commands: list, add, edit and remove.
pub enum ProjectCommand {
    /// List projects, sorted by name.
    List,
    /// Add a project.
    Add {
        /// Name of the project.
        name: String,
        /// Colour as #rrggbb.
        #[arg(long, value_name = "HEX")]
        color: Option<String>,
        /// Create the project as archived.
        #[arg(long)]
        archived: bool,
    },
    /// Change a project. Only the given fields change.
    Edit {
        /// Name or id of the project.
        project: String,
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        #[arg(long, value_name = "HEX")]
        color: Option<String>,
        /// Hide the project from new entries.
        #[arg(long, conflicts_with = "unarchive")]
        archive: bool,
        /// Show the project for new entries again.
        #[arg(long)]
        unarchive: bool,
    },
    /// Remove a project. Fails while time records still use it.
    #[command(alias = "rm")]
    Remove {
        /// Name or id of the project.
        project: String,
    },
}

#[derive(Debug, Subcommand)]
/// Absence kind commands: list, add, edit and remove.
pub enum AbsenceCommand {
    /// List absence kinds, sorted by name.
    List,
    /// Add an absence kind.
    Add {
        /// Name, for example "Vacation".
        name: String,
        #[arg(long, value_name = "HEX")]
        color: Option<String>,
        /// The absence is not paid.
        #[arg(long)]
        unpaid: bool,
    },
    /// Change an absence kind. Only the given fields change.
    Edit {
        /// Name or id of the absence kind.
        absence: String,
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        #[arg(long, value_name = "HEX")]
        color: Option<String>,
        #[arg(long, conflicts_with = "unpaid")]
        paid: bool,
        #[arg(long)]
        unpaid: bool,
    },
    /// Remove an absence kind. Fails while time records still use it.
    #[command(alias = "rm")]
    Remove {
        /// Name or id of the absence kind.
        absence: String,
    },
}

/// Columns that `entry list` can sort by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EntrySort {
    /// Day, then start time (default).
    Date,
    /// Project or absence name, ignoring case. Ties fall back to date.
    What,
    /// Booked hours. Ties fall back to date.
    Hours,
    /// Record id.
    Id,
}

#[derive(Debug, Subcommand)]
/// Time record commands: list, add, edit and remove.
pub enum EntryCommand {
    /// List time records of a period, by default the current month, sorted by date.
    ///
    /// Shows the first 10 records. Use --offset to see the next ones, or --all for every record.
    List {
        /// First day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        from: Option<String>,
        /// Last day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        to: Option<String>,
        /// Column to sort by.
        #[arg(long, value_enum, value_name = "COLUMN", default_value_t = EntrySort::Date)]
        sort: EntrySort,
        /// Sort in descending order.
        #[arg(long)]
        reverse: bool,
        /// Records per page.
        #[arg(long, value_name = "N", default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..))]
        limit: u32,
        /// Records to skip, to see the next page.
        #[arg(long, value_name = "N", default_value_t = 0)]
        offset: u32,
        /// Show every record, without paging.
        #[arg(long, conflicts_with_all = ["limit", "offset"])]
        all: bool,
        /// Output format. `table` (default), `json`, or a template such as
        /// "{{.Date}} {{.Hours}} {{.What}}". Start a template with "table " to print a header and
        /// aligned columns, and separate columns with \t. Fields: ID, Date, Start, End, Hours,
        /// Break, What, Note.
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// Add a time record. Give two of start, end and duration; the third is worked out.
    Add(EntryFields),
    /// Change a time record. Only the given fields change.
    Edit {
        /// Id of the record, see `entry list`.
        id: i64,
        #[command(flatten)]
        fields: EntryFields,
    },
    /// Remove a time record.
    #[command(alias = "rm")]
    Remove {
        /// Id of the record, see `entry list`.
        id: i64,
    },
}

/// The fields of a time record, shared by `add` and `edit`.
#[derive(Debug, Args, Default)]
pub struct EntryFields {
    /// Day of the record, YYYY-MM-DD. Defaults to today.
    #[arg(long, value_name = "DATE")]
    pub date: Option<String>,
    /// Start time, HH:MM. Defaults to the pending start of the day.
    #[arg(long, value_name = "HH:MM")]
    pub start: Option<String>,
    /// End time, HH:MM.
    #[arg(long, value_name = "HH:MM")]
    pub end: Option<String>,
    /// Work time: 7.5, 7h, 450m. A bare number is hours.
    #[arg(long, value_name = "TIME")]
    pub duration: Option<String>,
    /// Unpaid break within the record: 45, 45m, 1h. Defaults to the setting.
    #[arg(long = "break", value_name = "TIME")]
    pub break_time: Option<String>,
    /// Project, by name or id.
    #[arg(
        long,
        value_name = "PROJECT",
        conflicts_with = "absence",
        add = ArgValueCompleter::new(complete::complete_project)
    )]
    pub project: Option<String>,
    /// Absence kind, by name or id.
    #[arg(long, value_name = "ABSENCE", add = ArgValueCompleter::new(complete::complete_absence))]
    pub absence: Option<String>,
    /// Free text.
    #[arg(long, value_name = "TEXT")]
    pub note: Option<String>,
}

#[derive(Debug, Subcommand)]
/// Commands for the pending start: show it, start it, or clear it.
pub enum PendingCommand {
    /// Show the pending start.
    Show,
    /// Remember that work started now, or at the given time.
    Start {
        /// Start time, HH:MM. Defaults to now.
        #[arg(long, value_name = "HH:MM")]
        time: Option<String>,
    },
    /// Forget the pending start.
    Clear,
}

#[derive(Debug, Subcommand)]
/// Holiday commands: list, add, edit, remove, and import from a calendar address or file.
pub enum HolidayCommand {
    /// List holidays of a year, by default this year.
    List {
        #[arg(long, value_name = "YEAR")]
        year: Option<i32>,
    },
    /// Add a holiday.
    Add {
        /// Day, YYYY-MM-DD.
        date: String,
        /// Name, for example "Christmas".
        name: String,
    },
    /// Change a holiday. Only the given fields change.
    Edit {
        /// Id of the holiday, see `holiday list`.
        id: i64,
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
    },
    /// Remove a holiday.
    #[command(alias = "rm")]
    Remove {
        /// Id of the holiday, see `holiday list`.
        id: i64,
    },
    /// Import the all-day events of an iCalendar address.
    Import {
        /// Address of the .ics file, http:// or https://.
        url: String,
    },
    /// Import the all-day events of a local .ics file.
    ImportFile {
        /// Path to the .ics file.
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Subcommand)]
/// Settings commands: show all settings, or set one.
pub enum SettingsCommand {
    /// Show all settings.
    Show,
    /// Change one setting.
    Set {
        /// The setting to change.
        key: SettingKey,
        /// The new value. Hours for daily-hours and carry-over, minutes or 1h for break.
        value: String,
    },
}

/// The settings that `settings set` can change.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SettingKey {
    /// Working time per day, for example 7.5 or 7h30.
    DailyHours,
    /// Default unpaid break, for example 45 or 45m.
    Break,
    /// Interface language, for example de or en.
    Language,
    /// Colour theme id, for example nord-dark.
    Theme,
    /// Show the email in the header: true or false.
    ShowEmail,
    /// Overtime carried over, signed hours such as -1:30 or +12:00.
    CarryOver,
}

#[derive(Debug, Subcommand)]
/// Token commands: list the tokens of the account, or remove one.
pub enum TokenCommand {
    /// List the tokens of this account. The secret is never shown again after login.
    List,
    /// Revoke a token, for example one from a lost computer.
    #[command(alias = "rm")]
    Remove {
        /// Id of the token, see `token list`.
        id: i64,
    },
}
