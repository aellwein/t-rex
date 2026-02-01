//! Colour themes of the web app. A theme is a set of colour values for the palette tokens; the web app
//! sets them as CSS variables, so a theme imported later has the same shape as a built-in one.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

mod popular;

/// The setting that follows the browser's colour scheme: Nord Light or Nord Dark.
pub const SYSTEM: &str = "system";

/// The colour tokens a theme sets. Each one is the Tailwind variable `--color-<token>`.
pub const TOKENS: &[&str] = &[
    "neutral-100",
    "neutral-200",
    "neutral-300",
    "neutral-400",
    "neutral-500",
    "neutral-600",
    "neutral-700",
    "neutral-800",
    "black",
    "white",
    "teal-50",
    "teal-400",
    "teal-500",
    "teal-600",
    "teal-700",
    "teal-900",
    "red-50",
    "red-400",
    "red-500",
    "red-600",
    "red-950",
];

/// A colour theme: its id, its name, whether it is a dark one, and a `#rrggbb` value per token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    /// Dark themes also set the `dark` class, which switches the `dark:` style variants.
    pub dark: bool,
    pub colors: BTreeMap<String, String>,
}

impl ThemeInfo {
    /// Checks that every token has a valid colour and that there are no unknown tokens.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err("a theme needs an id and a name".into());
        }
        for token in TOKENS {
            match self.colors.get(*token) {
                None => return Err(format!("theme '{}' has no colour for '{token}'", self.id)),
                Some(value) if !is_hex_colour(value) => {
                    return Err(format!("'{value}' is not a #rrggbb colour"));
                }
                Some(_) => {}
            }
        }
        if let Some(unknown) = self
            .colors
            .keys()
            .find(|key| !TOKENS.contains(&key.as_str()))
        {
            return Err(format!("unknown colour token '{unknown}'"));
        }
        Ok(())
    }
}

fn is_hex_colour(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Maps the names stored before theme ids existed to the ids in use now.
fn alias(id: &str) -> &str {
    match id {
        "light" | "Light" => "nord-light",
        "dark" | "Dark" => "nord-dark",
        other => other,
    }
}

/// Whether a setting value is allowed: `system` or the id of a built-in theme.
pub fn is_known(id: &str) -> bool {
    id == SYSTEM || builtin_themes().iter().any(|theme| theme.id == id)
}

/// The theme to show for a setting value. `system` picks Nord Dark or Nord Light by the browser's
/// preference. Unknown values fall back to Nord Light.
pub fn resolve(id: &str, prefers_dark: bool) -> ThemeInfo {
    let themes = builtin_themes();
    let wanted = match alias(id) {
        SYSTEM if prefers_dark => "nord-dark",
        SYSTEM => "nord-light",
        other => other,
    };
    themes
        .iter()
        .find(|theme| theme.id == wanted)
        .cloned()
        .unwrap_or_else(|| themes[0].clone())
}

/// Every built-in theme, in the order the settings list them.
pub fn builtin_themes() -> Vec<ThemeInfo> {
    let mut themes = vec![nord_light(), nord_dark()];
    themes.extend(CATPPUCCIN.iter().map(catppuccin));
    themes.extend(popular::themes());
    themes
}

fn theme(id: &str, name: &str, dark: bool, pairs: &[(&str, &str)]) -> ThemeInfo {
    let owned: Vec<(&str, String)> = pairs
        .iter()
        .map(|(token, value)| (*token, value.to_string()))
        .collect();
    theme_from_pairs(id, name, dark, &owned)
}

fn theme_from_pairs(id: &str, name: &str, dark: bool, pairs: &[(&str, String)]) -> ThemeInfo {
    ThemeInfo {
        id: id.into(),
        name: name.into(),
        dark,
        colors: pairs
            .iter()
            .map(|(token, value)| (token.to_string(), value.clone()))
            .collect(),
    }
}

/// Nord, light flavour: the colours the app used before themes could be chosen.
/// Palette: https://www.nordtheme.com/docs/colors-and-palettes/
fn nord_light() -> ThemeInfo {
    theme(
        "nord-light",
        "Nord Light",
        false,
        &[
            ("neutral-100", "#e5e9f0"),
            ("neutral-200", "#d8dee9"),
            ("neutral-300", "#cdd4e0"),
            ("neutral-400", "#aeb7c7"),
            ("neutral-500", "#6b7487"),
            ("neutral-600", "#4c566a"),
            ("neutral-700", "#3b4252"),
            ("neutral-800", "#2e3440"),
            ("black", "#2e3440"),
            ("white", "#eceff4"),
            ("teal-50", "#e5eef6"),
            ("teal-400", "#88c0d0"),
            ("teal-500", "#5e81ac"),
            ("teal-600", "#4c6a8f"),
            ("teal-700", "#3d5777"),
            ("teal-900", "#2e3f57"),
            ("red-50", "#fbeeee"),
            ("red-400", "#e0838b"),
            ("red-500", "#bf616a"),
            ("red-600", "#a84e57"),
            ("red-950", "#3b2326"),
        ],
    )
}

/// Nord, dark flavour.
fn nord_dark() -> ThemeInfo {
    theme(
        "nord-dark",
        "Nord Dark",
        true,
        &[
            ("neutral-100", "#2e3440"),
            ("neutral-200", "#3b4252"),
            ("neutral-300", "#d8dee9"),
            ("neutral-400", "#7b8497"),
            ("neutral-500", "#616b80"),
            ("neutral-600", "#434c5e"),
            ("neutral-700", "#2e3440"),
            ("neutral-800", "#3b4252"),
            ("black", "#2e3440"),
            ("white", "#eceff4"),
            ("teal-50", "#3b4d63"),
            ("teal-400", "#88c0d0"),
            ("teal-500", "#81a1c1"),
            ("teal-600", "#5e81ac"),
            ("teal-700", "#4c6a8f"),
            ("teal-900", "#3b4d63"),
            ("red-50", "#3b2326"),
            ("red-400", "#e0838b"),
            ("red-500", "#bf616a"),
            ("red-600", "#d08070"),
            ("red-950", "#3b2326"),
        ],
    )
}

/// The colours of one Catppuccin flavour, from the official palette
/// (https://github.com/catppuccin/rust, palette.json, version 2.8.0).
struct Flavor {
    id: &'static str,
    name: &'static str,
    dark: bool,
    base: &'static str,
    mantle: &'static str,
    crust: &'static str,
    surface0: &'static str,
    surface1: &'static str,
    overlay0: &'static str,
    overlay1: &'static str,
    overlay2: &'static str,
    subtext0: &'static str,
    subtext1: &'static str,
    text: &'static str,
    blue: &'static str,
    sapphire: &'static str,
    sky: &'static str,
    red: &'static str,
    maroon: &'static str,
}

const CATPPUCCIN: [Flavor; 4] = [
    Flavor {
        id: "catppuccin-latte",
        name: "Catppuccin Latte",
        dark: false,
        base: "#eff1f5",
        mantle: "#e6e9ef",
        crust: "#dce0e8",
        surface0: "#ccd0da",
        surface1: "#bcc0cc",
        overlay0: "#9ca0b0",
        overlay1: "#8c8fa1",
        overlay2: "#7c7f93",
        subtext0: "#6c6f85",
        subtext1: "#5c5f77",
        text: "#4c4f69",
        blue: "#1e66f5",
        sapphire: "#209fb5",
        sky: "#04a5e5",
        red: "#d20f39",
        maroon: "#e64553",
    },
    Flavor {
        id: "catppuccin-frappe",
        name: "Catppuccin Frappé",
        dark: true,
        base: "#303446",
        mantle: "#292c3c",
        crust: "#232634",
        surface0: "#414559",
        surface1: "#51576d",
        overlay0: "#737994",
        overlay1: "#838ba7",
        overlay2: "#949cbb",
        subtext0: "#a5adce",
        subtext1: "#b5bfe2",
        text: "#c6d0f5",
        blue: "#8caaee",
        sapphire: "#85c1dc",
        sky: "#99d1db",
        red: "#e78284",
        maroon: "#ea999c",
    },
    Flavor {
        id: "catppuccin-macchiato",
        name: "Catppuccin Macchiato",
        dark: true,
        base: "#24273a",
        mantle: "#1e2030",
        crust: "#181926",
        surface0: "#363a4f",
        surface1: "#494d64",
        overlay0: "#6e738d",
        overlay1: "#8087a2",
        overlay2: "#939ab7",
        subtext0: "#a5adcb",
        subtext1: "#b8c0e0",
        text: "#cad3f5",
        blue: "#8aadf4",
        sapphire: "#7dc4e4",
        sky: "#91d7e3",
        red: "#ed8796",
        maroon: "#ee99a0",
    },
    Flavor {
        id: "catppuccin-mocha",
        name: "Catppuccin Mocha",
        dark: true,
        base: "#1e1e2e",
        mantle: "#181825",
        crust: "#11111b",
        surface0: "#313244",
        surface1: "#45475a",
        overlay0: "#6c7086",
        overlay1: "#7f849c",
        overlay2: "#9399b2",
        subtext0: "#a6adc8",
        subtext1: "#bac2de",
        text: "#cdd6f4",
        blue: "#89b4fa",
        sapphire: "#74c7ec",
        sky: "#89dceb",
        red: "#f38ba8",
        maroon: "#eba0ac",
    },
];

/// Builds the token colours of a Catppuccin flavour. Light flavours use the text colour for the
/// dark shades, dark flavours the base colour, so every token keeps its role of the Nord themes.
fn catppuccin(flavor: &Flavor) -> ThemeInfo {
    let f = flavor;
    let pairs: Vec<(&str, String)> = if f.dark {
        vec![
            ("neutral-100", f.base.into()),
            ("neutral-200", f.surface0.into()),
            ("neutral-300", f.text.into()),
            ("neutral-400", f.overlay2.into()),
            ("neutral-500", f.overlay0.into()),
            ("neutral-600", f.surface1.into()),
            ("neutral-700", f.base.into()),
            ("neutral-800", f.surface0.into()),
            ("black", f.base.into()),
            ("white", f.base.into()),
            ("teal-50", f.surface0.into()),
            ("teal-400", f.sky.into()),
            ("teal-500", f.blue.into()),
            ("teal-600", f.blue.into()),
            ("teal-700", mix(f.blue, f.base, 0.25)),
            ("teal-900", mix(f.blue, f.base, 0.6)),
            ("red-50", mix(f.red, f.base, 0.85)),
            ("red-400", f.maroon.into()),
            ("red-500", f.red.into()),
            ("red-600", f.red.into()),
            ("red-950", mix(f.red, f.base, 0.75)),
        ]
    } else {
        vec![
            ("neutral-100", f.mantle.into()),
            ("neutral-200", f.surface0.into()),
            ("neutral-300", f.surface1.into()),
            ("neutral-400", f.overlay0.into()),
            ("neutral-500", f.overlay1.into()),
            ("neutral-600", f.subtext0.into()),
            ("neutral-700", f.subtext1.into()),
            ("neutral-800", f.text.into()),
            ("black", f.text.into()),
            ("white", f.base.into()),
            ("teal-50", f.crust.into()),
            ("teal-400", f.sapphire.into()),
            ("teal-500", f.blue.into()),
            ("teal-600", f.blue.into()),
            ("teal-700", mix(f.blue, f.text, 0.2)),
            ("teal-900", mix(f.blue, f.text, 0.5)),
            ("red-50", mix(f.red, f.base, 0.88)),
            ("red-400", f.maroon.into()),
            ("red-500", f.red.into()),
            ("red-600", mix(f.red, f.text, 0.15)),
            ("red-950", mix(f.red, f.text, 0.5)),
        ]
    };
    ThemeInfo {
        id: f.id.into(),
        name: f.name.into(),
        dark: f.dark,
        colors: pairs
            .into_iter()
            .map(|(token, value)| (token.to_string(), value))
            .collect(),
    }
}

/// Mixes two `#rrggbb` colours: `t` = 0 gives `a`, `t` = 1 gives `b`.
fn mix(a: &str, b: &str, t: f32) -> String {
    let channels = |hex: &str| -> [f32; 3] {
        let value =
            |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).unwrap_or(0) as f32;
        [value(1..3), value(3..5), value(5..7)]
    };
    let (from, to) = (channels(a), channels(b));
    let mixed: Vec<String> = (0..3)
        .map(|i| format!("{:02x}", (from[i] + (to[i] - from[i]) * t).round() as u8))
        .collect();
    format!("#{}", mixed.concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_theme_is_complete_and_valid() {
        for theme in builtin_themes() {
            assert_eq!(theme.validate(), Ok(()), "{}", theme.id);
        }
    }

    #[test]
    fn builtin_ids_are_unique_and_known() {
        let themes = builtin_themes();
        let mut ids: Vec<&str> = themes.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), themes.len());
        for theme in &themes {
            assert!(is_known(&theme.id));
        }
        assert!(is_known(SYSTEM));
        assert!(!is_known("neon"));
    }

    #[test]
    fn old_names_and_system_resolve_to_nord() {
        assert_eq!(resolve("light", false).id, "nord-light");
        assert_eq!(resolve("dark", false).id, "nord-dark");
        assert_eq!(resolve("system", true).id, "nord-dark");
        assert_eq!(resolve("system", false).id, "nord-light");
        assert_eq!(resolve("catppuccin-mocha", false).id, "catppuccin-mocha");
        assert_eq!(resolve("unknown", false).id, "nord-light");
    }

    #[test]
    fn dark_flavours_are_marked_dark() {
        assert!(!resolve("catppuccin-latte", false).dark);
        assert!(resolve("catppuccin-frappe", false).dark);
        assert!(resolve("catppuccin-macchiato", false).dark);
        assert!(resolve("catppuccin-mocha", false).dark);
    }

    #[test]
    fn mixing_moves_between_the_two_colours() {
        assert_eq!(mix("#000000", "#ffffff", 0.0), "#000000");
        assert_eq!(mix("#000000", "#ffffff", 1.0), "#ffffff");
        assert_eq!(mix("#000000", "#ffffff", 0.5), "#808080");
    }

    #[test]
    fn invalid_colours_and_missing_tokens_are_rejected() {
        let mut theme = nord_light();
        theme.colors.insert("teal-500".into(), "blue".into());
        assert!(theme.validate().is_err());

        let mut theme = nord_light();
        theme.colors.remove("red-500");
        assert!(theme.validate().is_err());

        let mut theme = nord_light();
        theme.colors.insert("green-100".into(), "#00ff00".into());
        assert!(theme.validate().is_err());
    }
}
