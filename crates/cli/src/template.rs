//! Output of `entry list`: the table and the `--format` templates.
//!
//! A template is text with `{{.Field}}` placeholders, as in `docker --format`. Columns are
//! separated by a tab, written as `\t` or as a real tab. `\n` ends a line. A template that starts
//! with `table ` prints a header and aligns the columns. The fields are ID, Date, Start, End,
//! Hours, Break, What and Note.

use anyhow::{Result, bail};

use crate::output;

/// The fields a template can use, in the order they are listed in the help.
pub const FIELDS: [&str; 8] = [
    "ID", "Date", "Start", "End", "Hours", "Break", "What", "Note",
];

/// One record as the templates see it. Times are already formatted as `hh:mm`.
pub struct EntryRow {
    pub id: i64,
    pub date: String,
    pub start: String,
    pub end: String,
    pub hours: String,
    pub break_time: String,
    pub what: String,
    pub note: String,
}

impl EntryRow {
    fn field(&self, name: &str) -> String {
        match name {
            "ID" => self.id.to_string(),
            "Date" => self.date.clone(),
            "Start" => self.start.clone(),
            "End" => self.end.clone(),
            "Hours" => self.hours.clone(),
            "Break" => self.break_time.clone(),
            "What" => self.what.clone(),
            "Note" => self.note.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Part {
    Text(String),
    Field(&'static str),
}

/// A parsed template: one list of parts per column.
#[derive(Debug)]
pub struct Template {
    table: bool,
    columns: Vec<Vec<Part>>,
}

impl Template {
    /// Parses a template. Unknown fields and unclosed placeholders are errors.
    pub fn parse(text: &str) -> Result<Self> {
        let (table, body) = match text.strip_prefix("table ") {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let columns = body
            .replace("\\t", "\t")
            .split('\t')
            .map(parse_parts)
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { table, columns })
    }

    /// Prints the records: a table when the template starts with `table `, else one line each.
    pub fn print(&self, rows: &[EntryRow]) {
        let cells: Vec<Vec<String>> = rows.iter().map(|row| self.cells(row)).collect();
        if self.table {
            let headers: Vec<String> = self.columns.iter().map(|column| header(column)).collect();
            let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
            output::table(&header_refs, &cells);
        } else {
            for row in &cells {
                println!("{}", row.join("\t"));
            }
        }
    }

    fn cells(&self, row: &EntryRow) -> Vec<String> {
        self.columns
            .iter()
            .map(|column| {
                column
                    .iter()
                    .map(|part| match part {
                        Part::Text(text) => text.clone(),
                        Part::Field(name) => row.field(name),
                    })
                    .collect()
            })
            .collect()
    }
}

/// The header of a table column: the names of its fields, in capitals.
fn header(column: &[Part]) -> String {
    column
        .iter()
        .filter_map(|part| match part {
            Part::Field(name) => Some(name.to_uppercase()),
            Part::Text(_) => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_parts(text: &str) -> Result<Vec<Part>> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        push_text(&mut parts, &rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            bail!("unclosed placeholder in template \"{text}\"");
        };
        let placeholder = after[..close].trim();
        let Some(name) = placeholder.strip_prefix('.') else {
            bail!("placeholders look like {{{{.Date}}}}, found \"{placeholder}\"");
        };
        let Some(field) = FIELDS.iter().find(|field| **field == name) else {
            bail!("unknown field \"{name}\"; use one of {}", FIELDS.join(", "));
        };
        parts.push(Part::Field(field));
        rest = &after[close + 2..];
    }
    push_text(&mut parts, rest);
    Ok(parts)
}

fn push_text(parts: &mut Vec<Part>, text: &str) {
    let text = text.replace("\\n", "\n");
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
}

/// The records as a JSON array, for `--format json`.
pub fn to_json(rows: &[EntryRow]) -> serde_json::Value {
    serde_json::Value::Array(
        rows.iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.id,
                    "date": row.date,
                    "start": row.start,
                    "end": row.end,
                    "hours": row.hours,
                    "break": row.break_time,
                    "what": row.what,
                    "note": row.note,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> EntryRow {
        EntryRow {
            id: 7,
            date: "2026-11-02".into(),
            start: "09:00".into(),
            end: "17:30".into(),
            hours: "07:30".into(),
            break_time: "00:45".into(),
            what: "Website".into(),
            note: "release".into(),
        }
    }

    #[test]
    fn placeholders_and_columns_are_filled_in() {
        let template = Template::parse("{{.ID}}: {{.Hours}}\\t{{.What}}").unwrap();
        assert_eq!(template.cells(&row()), vec!["7: 07:30", "Website"]);
    }

    #[test]
    fn a_real_tab_also_separates_columns() {
        let template = Template::parse("{{.Date}}\t{{.Note}}").unwrap();
        assert_eq!(template.cells(&row()), vec!["2026-11-02", "release"]);
    }

    #[test]
    fn table_headers_come_from_the_fields() {
        let template = Template::parse("table {{.Date}}\\t{{.Start}} {{.End}}").unwrap();
        let headers: Vec<String> = template.columns.iter().map(|c| header(c)).collect();
        assert_eq!(headers, vec!["DATE", "START END"]);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let message = format!("{:#}", Template::parse("{{.Project}}").unwrap_err());
        assert!(message.contains("unknown field \"Project\""), "{message}");
    }

    #[test]
    fn unclosed_placeholders_are_rejected() {
        assert!(Template::parse("{{.Date").is_err());
    }

    #[test]
    fn newline_escape_ends_a_line() {
        let template = Template::parse("{{.ID}}\\n{{.Note}}").unwrap();
        assert_eq!(template.cells(&row()), vec!["7\nrelease"]);
    }

    #[test]
    fn json_keeps_the_same_names() {
        let value = to_json(&[row()]);
        assert_eq!(value[0]["hours"], "07:30");
        assert_eq!(value[0]["break"], "00:45");
        assert_eq!(value[0]["id"], 7);
    }
}
