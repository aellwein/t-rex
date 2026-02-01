use serde::{Deserialize, Serialize};

use crate::{validate_color, validate_name};

/// A kind of time off, such as vacation or sick leave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceKind {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub paid: bool,
}

/// Request body for creating or replacing an absence kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceKindInput {
    pub name: String,
    #[serde(default = "crate::default_color")]
    pub color: String,
    #[serde(default = "default_paid")]
    pub paid: bool,
}

fn default_paid() -> bool {
    true
}

impl AbsenceKindInput {
    /// Trims the name and lower-cases the colour so stored values are canonical.
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
