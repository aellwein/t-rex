//! Absence kinds: the kinds of time off that a time record can book, such as vacation or sick leave.
//! Paid absences count as actual working time in the reports; unpaid ones do not.

use serde::{Deserialize, Serialize};

use crate::{validate_color, validate_name};

/// A kind of time off, such as vacation or sick leave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceKind {
    /// Id of the kind, assigned by the server.
    pub id: i64,
    /// Name shown in the calendar and the reports, for example "Vacation".
    pub name: String,
    /// Colour as `#rrggbb`, used for the marker of the kind.
    pub color: String,
    /// Whether the time counts as working time in the reports.
    pub paid: bool,
}

/// Request body for creating or replacing an absence kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceKindInput {
    /// Name of the kind to create or replace. Must not be blank.
    pub name: String,
    #[serde(default = "crate::default_color")]
    /// Colour as `#rrggbb`. Stored in lower case. Defaults to the teal of the calendar.
    pub color: String,
    #[serde(default = "default_paid")]
    /// Whether the time counts as working time. Defaults to paid.
    pub paid: bool,
}

/// Absence kinds are paid unless they are marked as unpaid.
fn default_paid() -> bool {
    true
}

impl AbsenceKindInput {
    /// Trims the name and lower-cases the colour so stored values are canonical.
    /// Trims the name and lower-cases the colour so stored values are canonical.
    ///
    /// # Examples
    ///
    /// ```
    /// use dto::AbsenceKindInput;
    ///
    /// let mut input = AbsenceKindInput { name: "  Vacation ".into(), color: "#F59E0B".into(), paid: true };
    /// input.normalize();
    /// assert_eq!(input.name, "Vacation");
    /// assert_eq!(input.color, "#f59e0b");
    /// assert!(input.validate().is_ok());
    /// ```
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.color = self.color.trim().to_ascii_lowercase();
    }

    /// Checks the input. Call [`AbsenceKindInput::normalize`] first.
    pub fn validate(&self) -> Result<(), String> {
        validate_name(&self.name)?;
        validate_color(&self.color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paid_defaults_to_true_when_missing() {
        let parsed: AbsenceKindInput = serde_json::from_str(r#"{"name":"Vacation"}"#).unwrap();
        assert!(parsed.paid);
        assert_eq!(parsed.color, crate::DEFAULT_COLOR);
    }

    #[test]
    fn blank_name_is_rejected() {
        let input = AbsenceKindInput {
            name: " ".into(),
            color: "#0d9488".into(),
            paid: true,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn normalize_then_validate_accepts_padded_input() {
        let mut input = AbsenceKindInput {
            name: " Sick leave ".into(),
            color: " #AABBCC ".into(),
            paid: false,
        };
        input.normalize();
        assert_eq!(input.name, "Sick leave");
        assert_eq!(input.color, "#aabbcc");
        assert!(input.validate().is_ok());
    }
}
