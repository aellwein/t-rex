//! Applying the colour theme to the page.

use crate::common::prelude::*;

/// Applies a colour theme: its colours become CSS variables on the root element, and dark themes
/// also set the `dark` class, which switches the `dark:` style variants.
pub fn set_theme(id: &str) {
    let theme = dto::theme::resolve(id, prefers_dark());
    let root = document()
        .document_element()
        .expect("unable to get root element");
    let variables: String = theme
        .colors
        .iter()
        .map(|(token, value)| format!("--color-{token}: {value};"))
        .collect();
    root.set_attribute("style", &variables)
        .expect("unable to set theme colours");
    if theme.dark {
        set_classes("dark");
    } else {
        remove_classes("dark");
    }
}
