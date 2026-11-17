//! Tokens page: the command line tokens of the account. A token signs one computer in. Revoking it
//! signs that computer out.

use chrono::{DateTime, Local, Utc};
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::api;
use crate::common::prelude::window;
use crate::components::ui::{BUTTON_DANGER, CARD, ERROR_TEXT, PAGE_TITLE};

/// Formats the creation time for the local time zone. Falls back to the stored text.
fn created_text(created_at: &str) -> String {
    match DateTime::parse_from_rfc3339(created_at) {
        Ok(time) => time
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
        Err(_) => created_at.to_string(),
    }
}

/// Whether the token is past its expiry time.
fn is_expired(expires_at: &str) -> bool {
    DateTime::parse_from_rfc3339(expires_at).is_ok_and(|time| time < Utc::now())
}

#[component]
/// The tokens page. See the module description.
pub fn TokensPage() -> Element {
    let mut tokens = use_resource(|| async { api::list_tokens().await });
    let mut failure = use_signal(|| None::<String>);

    let title = t!("tokens-title");
    let note = t!("tokens-note");
    let loading_text = t!("loading");
    let load_failed = t!("loading-failed");
    let delete_failed = t!("delete-failed");
    let none_yet = t!("tokens-empty");
    let created_label = t!("tokens-created");
    let expires_label = t!("tokens-expires");
    let expired_label = t!("tokens-expired");
    let revoke_label = t!("tokens-revoke");
    let revoke_confirm = use_signal(|| t!("tokens-revoke-confirm"));

    let body = match &*tokens.read() {
        None => rsx! { p { {loading_text.clone()} } },
        Some(Err(err)) => rsx! { p { class: ERROR_TEXT, "{load_failed}: {err}" } },
        Some(Ok(list)) if list.is_empty() => rsx! { p { {none_yet.clone()} } },
        Some(Ok(list)) => {
            let items = list.clone();
            rsx! {
                ul { class: "flex flex-col gap-3",
                    for token in items {
                        li { key: "{token.id}", class: format!("{CARD} flex flex-wrap items-center gap-4"),
                            div { class: "flex flex-col mr-auto min-w-0",
                                span { class: "font-medium break-all", "{token.name}" }
                                span { class: "text-sm text-neutral-600 dark:text-neutral-300",
                                    "{created_label}: {created_text(&token.created_at)}"
                                }
                                span { class: "text-sm text-neutral-600 dark:text-neutral-300",
                                    "{expires_label}: {created_text(&token.expires_at)}"
                                }
                                if is_expired(&token.expires_at) {
                                    span { class: ERROR_TEXT, {expired_label.clone()} }
                                }
                            }
                            button {
                                class: BUTTON_DANGER,
                                r#type: "button",
                                onclick: move |_| {
                                    let id = token.id;
                                    let confirmed = window()
                                        .confirm_with_message(&revoke_confirm.read())
                                        .unwrap_or(false);
                                    if !confirmed {
                                        return;
                                    }
                                    spawn(async move {
                                        match api::delete_token(id).await {
                                            Ok(()) => {
                                                failure.set(None);
                                                tokens.restart();
                                            }
                                            Err(err) => failure.set(Some(err.to_string())),
                                        }
                                    });
                                },
                                {revoke_label.clone()}
                            }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div { class: "flex flex-col gap-4",
            h1 { class: PAGE_TITLE, {title} }
            p { class: "text-sm text-neutral-600 dark:text-neutral-300", {note} }
            if let Some(message) = failure.read().clone() {
                p { class: ERROR_TEXT, "{delete_failed}: {message}" }
            }
            {body}
        }
    }
}
