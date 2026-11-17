//! Projects page: list, create, edit and delete projects.

use dioxus::prelude::*;
use dioxus_i18n::t;
use dto::{ProjectInput, DEFAULT_COLOR};

use crate::api;
use crate::components::ui::{
    Failed, BUTTON, BUTTON_DANGER, BUTTON_PRIMARY, CARD, ERROR_TEXT, INPUT, LABEL, PAGE_TITLE,
};

#[component]
/// The projects page. See the module description.
pub fn ProjectsPage() -> Element {
    let mut projects = use_resource(|| async { api::list_projects().await });
    let mut editing = use_signal(|| None::<i64>);
    let mut name = use_signal(String::new);
    let mut color = use_signal(|| DEFAULT_COLOR.to_string());
    let mut archived = use_signal(|| false);
    let mut failure = use_signal(|| None::<(Failed, String)>);

    // Translations are resolved while rendering, so handlers only capture signals.
    let title = t!("projects-title");
    let loading_text = t!("loading");
    let load_failed = t!("loading-failed");
    let save_failed = t!("save-failed");
    let delete_failed = t!("delete-failed");
    let none_yet = t!("no-projects");
    let name_label = t!("name");
    let color_label = t!("color");
    let archived_label = t!("archived");
    let add_label = t!("add");
    let save_label = t!("save");
    let cancel_label = t!("cancel");
    let edit_label = t!("edit");
    let delete_label = t!("delete");

    let save = move |evt: FormEvent| {
        evt.prevent_default();
        let input = ProjectInput {
            name: name(),
            color: color(),
            archived: archived(),
        };
        let target = editing();
        spawn(async move {
            let result = match target {
                Some(id) => api::update_project(id, &input).await,
                None => api::create_project(&input).await,
            };
            match result {
                Ok(_) => {
                    editing.set(None);
                    name.set(String::new());
                    color.set(DEFAULT_COLOR.to_string());
                    archived.set(false);
                    failure.set(None);
                    projects.restart();
                }
                Err(err) => failure.set(Some((Failed::Save, err.to_string()))),
            }
        });
    };

    let cancel = move |_: MouseEvent| {
        editing.set(None);
        name.set(String::new());
        color.set(DEFAULT_COLOR.to_string());
        archived.set(false);
    };

    let list_view = match &*projects.read() {
        None => rsx! {
            p { {loading_text.clone()} }
        },
        Some(Err(err)) => rsx! {
            p { class: ERROR_TEXT, "{load_failed}: {err}" }
        },
        Some(Ok(list)) if list.is_empty() => rsx! {
            p { {none_yet.clone()} }
        },
        Some(Ok(list)) => {
            let rows = list.clone();
            rsx! {
                ul { class: "flex flex-col gap-2",
                    for project in rows {
                        {
                            let id = project.id;
                            let row_name = project.name.clone();
                            let row_color = project.color.clone();
                            let row_archived = project.archived;
                            let edit_name = row_name.clone();
                            let edit_color = row_color.clone();
                            rsx! {
                                li { class: format!("{CARD} flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between"),
                                    div { class: "flex items-center gap-3 min-w-0",
                                        span {
                                            class: "inline-block w-4 h-4 rounded-full shrink-0",
                                            style: format!("background-color: {row_color}"),
                                        }
                                        span { class: "font-medium truncate", "{row_name}" }
                                        if row_archived {
                                            span { class: "text-sm text-neutral-500", {archived_label.clone()} }
                                        }
                                    }
                                    div { class: "flex gap-2 shrink-0",
                                        button {
                                            class: BUTTON,
                                            r#type: "button",
                                            onclick: move |_| {
                                                editing.set(Some(id));
                                                name.set(edit_name.clone());
                                                color.set(edit_color.clone());
                                                archived.set(row_archived);
                                            },
                                            {edit_label.clone()}
                                        }
                                        button {
                                            class: BUTTON_DANGER,
                                            r#type: "button",
                                            onclick: move |_| {
                                                spawn(async move {
                                                    match api::delete_project(id).await {
                                                        Ok(()) => {
                                                            failure.set(None);
                                                            projects.restart();
                                                        }
                                                        Err(err) => failure.set(Some((Failed::Delete, err.to_string()))),
                                                    }
                                                });
                                            },
                                            {delete_label.clone()}
                                        }
                                    }
                                }
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
            if let Some((kind, message)) = failure() {
                p { class: ERROR_TEXT,
                    {match kind {
                        Failed::Save => save_failed.clone(),
                        Failed::Delete => delete_failed.clone(),
                    }}
                    ": {message}"
                }
            }
            form { class: format!("{CARD} flex flex-col gap-4"), onsubmit: save,
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {name_label.clone()} }
                    input {
                        class: INPUT,
                        r#type: "text",
                        required: true,
                        value: name(),
                        oninput: move |evt| name.set(evt.value()),
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {color_label.clone()} }
                    input {
                        class: "h-11 w-20 rounded-lg border border-neutral-400 cursor-pointer",
                        r#type: "color",
                        value: color(),
                        oninput: move |evt| color.set(evt.value()),
                    }
                }
                label { class: "flex items-center gap-2 min-h-11",
                    input {
                        r#type: "checkbox",
                        checked: archived(),
                        onchange: move |evt| archived.set(evt.checked()),
                    }
                    {archived_label.clone()}
                }
                div { class: "flex flex-wrap gap-2",
                    button { class: BUTTON_PRIMARY, r#type: "submit",
                        if editing().is_some() {
                            {save_label.clone()}
                        } else {
                            {add_label.clone()}
                        }
                    }
                    if editing().is_some() {
                        button { class: BUTTON, r#type: "button", onclick: cancel, {cancel_label.clone()} }
                    }
                }
            }
            {list_view}
        }
    }
}
