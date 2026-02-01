use serde::{Deserialize, Serialize};

use crate::MINUTES_PER_DAY;

/// Display language of the web app. `System` uses the browser's language, English if unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    System,
    En,
    De,
    Ru,
    Fr,
    Es,
    It,
    Pt,
    Nl,
    Pl,
    Zh,
    Ja,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Language::System => "system",
            Language::En => "en",
            Language::De => "de",
            Language::Ru => "ru",
            Language::Fr => "fr",
            Language::Es => "es",
            Language::It => "it",
            Language::Pt => "pt",
            Language::Nl => "nl",
            Language::Pl => "pl",
            Language::Zh => "zh",
            Language::Ja => "ja",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "system" => Some(Language::System),
            "en" => Some(Language::En),
            "de" => Some(Language::De),
            "ru" => Some(Language::Ru),
            "fr" => Some(Language::Fr),
            "es" => Some(Language::Es),
            "it" => Some(Language::It),
            "pt" => Some(Language::Pt),
            "nl" => Some(Language::Nl),
            "pl" => Some(Language::Pl),
            "zh" => Some(Language::Zh),
            "ja" => Some(Language::Ja),
            _ => None,
        }
    }
}

/// Per-user settings. Also the body of `PUT /api/v1/settings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// `system`, or the id of a colour theme, see [`crate::theme`].
    pub theme: String,
    pub language: Language,
    /// Expected working time per day, in minutes.
    pub daily_minutes: u32,
    /// Show the account's email in the header instead of its name. Off by default.
    #[serde(default)]
    pub show_email: bool,
    /// Unpaid break per working day, in minutes. New days start with this value.
    #[serde(default = "default_break_minutes")]
    pub break_minutes: u32,
    /// Overtime (or deficit) from before the recorded time, in minutes. Added to the total.
    #[serde(default)]
    pub carry_over_minutes: i32,
}

fn default_break_minutes() -> u32 {
    45
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: crate::theme::SYSTEM.to_string(),
            language: Language::System,
            daily_minutes: 8 * 60,
            show_email: false,
            break_minutes: default_break_minutes(),
            carry_over_minutes: 0,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !crate::theme::is_known(&self.theme) {
            return Err(format!("unknown theme '{}'", self.theme));
        }
        if self.break_minutes > MINUTES_PER_DAY {
            return Err(format!("break_minutes must be at most {MINUTES_PER_DAY}"));
        }
        if self.daily_minutes > MINUTES_PER_DAY {
            return Err(format!("daily_minutes must be at most {MINUTES_PER_DAY}"));
        }
        Ok(())
    }
}

/// Parses a signed amount of hours into minutes: `12:30`, `-1:45`, `+2,5`, `-1.25`.
pub fn parse_signed_hours(text: &str) -> Option<i32> {
    let text = text.trim();
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let minutes: i64 = match rest.split_once(':') {
        Some((hours, minutes)) => {
            let hours: u32 = hours.parse().ok()?;
            let minutes: u32 = minutes.parse().ok()?;
            if minutes >= 60 {
                return None;
            }
            i64::from(hours) * 60 + i64::from(minutes)
        }
        None => {
            let value: f64 = rest.replace(',', ".").parse().ok()?;
            if !value.is_finite() || value < 0.0 {
                return None;
            }
            (value * 60.0).round() as i64
        }
    };
    let signed = if negative { -minutes } else { minutes };
    i32::try_from(signed).ok()
}

/// Formats signed minutes as hours, e.g. `+12:30` or `-1:45`. Zero has no sign.
pub fn format_signed_hours(minutes: i32) -> String {
    let sign = match minutes.cmp(&0) {
        std::cmp::Ordering::Less => "-",
        std::cmp::Ordering::Equal => "",
        std::cmp::Ordering::Greater => "+",
    };
    let abs = minutes.unsigned_abs();
    format!("{sign}{}:{:02}", abs / 60, abs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_system_and_are_eight_hours() {
        let settings = Settings::default();
        assert_eq!(settings.theme, "system");
        assert_eq!(settings.language, Language::System);
        assert_eq!(settings.daily_minutes, 480);
        assert!(!settings.show_email, "email stays hidden unless chosen");
        assert_eq!(settings.break_minutes, 45);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn more_than_a_day_is_rejected() {
        let settings = Settings {
            daily_minutes: MINUTES_PER_DAY + 1,
            ..Settings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn enums_serialize_as_lowercase_strings() {
        let settings = Settings {
            theme: "catppuccin-mocha".into(),
            language: Language::De,
            daily_minutes: 420,
            show_email: true,
            break_minutes: 30,
            carry_over_minutes: -90,
        };
        let json = serde_json::to_value(settings).unwrap();
        assert_eq!(json["theme"], "catppuccin-mocha");
        assert_eq!(json["language"], "de");
        assert_eq!(json["daily_minutes"], 420);
    }

    #[test]
    fn string_conversions_round_trip() {
        let all = [
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
        ];
        for language in all {
            assert_eq!(Language::parse(language.as_str()), Some(language));
        }
        assert_eq!(Language::parse("xx"), None);
    }

    #[test]
    fn signed_hours_parse_in_several_notations() {
        assert_eq!(parse_signed_hours("12:30"), Some(750));
        assert_eq!(parse_signed_hours("-1:45"), Some(-105));
        assert_eq!(parse_signed_hours("+2,5"), Some(150));
        assert_eq!(parse_signed_hours("-1.25"), Some(-75));
        assert_eq!(parse_signed_hours("0"), Some(0));
        assert_eq!(parse_signed_hours("abc"), None);
        assert_eq!(parse_signed_hours("1:75"), None);
        assert_eq!(parse_signed_hours(""), None);
    }

    #[test]
    fn signed_hours_format_with_sign() {
        assert_eq!(format_signed_hours(750), "+12:30");
        assert_eq!(format_signed_hours(-105), "-1:45");
        assert_eq!(format_signed_hours(0), "0:00");
        assert_eq!(format_signed_hours(5), "+0:05");
    }
}
