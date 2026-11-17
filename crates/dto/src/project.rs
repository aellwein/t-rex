//! Projects: the names that time is booked against, with a colour for the calendar and the reports.

use serde::{Deserialize, Serialize};

use crate::{validate_color, validate_name};

/// A project that time can be booked against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Id of the project, assigned by the server.
    pub id: i64,
    /// Name shown in the calendar and the reports.
    pub name: String,
    /// Colour as `#rrggbb`.
    pub color: String,
    /// Archived projects are hidden from the pickers. Their records stay.
    pub archived: bool,
}

/// Request body for creating or replacing a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectInput {
    /// Name of the project to create or replace. Must not be blank.
    pub name: String,
    #[serde(default = "crate::default_color")]
    /// Colour as `#rrggbb`. Stored in lower case. Defaults to the teal of the calendar.
    pub color: String,
    #[serde(default)]
    /// Whether the project is archived. Defaults to not archived.
    pub archived: bool,
}

impl ProjectInput {
    /// Trims the name and lower-cases the colour so stored values are canonical.
    /// Trims the name and lower-cases the colour so stored values are canonical.
    ///
    /// # Examples
    ///
    /// ```
    /// use dto::ProjectInput;
    ///
    /// let mut input = ProjectInput { name: "  Website ".into(), color: "#0D9488".into(), archived: false };
    /// input.normalize();
    /// assert_eq!(input.name, "Website");
    /// assert_eq!(input.color, "#0d9488");
    /// assert!(input.validate().is_ok());
    ///
    /// input.color = "teal".into();
    /// assert!(input.validate().is_err());
    /// ```
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.color = self.color.trim().to_ascii_lowercase();
    }

    /// Checks the input. Call [`ProjectInput::normalize`] first.
    pub fn validate(&self) -> Result<(), String> {
        validate_name(&self.name)?;
        validate_color(&self.color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, color: &str) -> ProjectInput {
        ProjectInput {
            name: name.into(),
            color: color.into(),
            archived: false,
        }
    }

    #[test]
    fn valid_project_passes_validation() {
        assert!(input("Website", "#0d9488").validate().is_ok());
    }

    #[test]
    fn blank_name_is_rejected() {
        assert!(input("   ", "#0d9488").validate().is_err());
    }

    #[test]
    fn invalid_color_is_rejected() {
        assert!(input("Website", "teal").validate().is_err());
    }

    #[test]
    fn normalize_trims_name_and_lowercases_color() {
        let mut project = input("  Website  ", " #0D9488 ");
        project.normalize();
        assert_eq!(project.name, "Website");
        assert_eq!(project.color, "#0d9488");
        assert!(project.validate().is_ok());
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let parsed: ProjectInput = serde_json::from_str(r#"{"name":"Acme"}"#).unwrap();
        assert_eq!(parsed.color, crate::DEFAULT_COLOR);
        assert!(!parsed.archived);
    }

    #[test]
    fn project_round_trips_through_json() {
        let project = Project {
            id: 7,
            name: "Acme".into(),
            color: "#123456".into(),
            archived: true,
        };
        let json = serde_json::to_string(&project).unwrap();
        assert_eq!(serde_json::from_str::<Project>(&json).unwrap(), project);
    }
}
