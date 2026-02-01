use dioxus::prelude::*;
use dioxus_i18n::{prelude::*, t};

use crate::common::prelude::window;
use crate::components::ui::NAV_LINK;
use crate::pages::absences::AbsencesPage;
use crate::pages::holidays::HolidaysPage;
use crate::pages::home::HomePage;
use crate::pages::projects::ProjectsPage;
use crate::pages::reports::ReportsPage;
use crate::pages::settings::{to_app_language, SettingsPage};
use crate::pages::tokens::TokensPage;
use crate::{api, STATE};

#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        HomePage {},
        #[route("/projects")]
        ProjectsPage {},
        #[route("/absences")]
        AbsencesPage {},
        #[route("/holidays")]
        HolidaysPage {},
        #[route("/tokens")]
        TokensPage {},
        #[route("/reports")]
        ReportsPage {},
        #[route("/settings")]
        SettingsPage {},
    #[end_layout]
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

/// Shown for unknown URLs.
#[component]
fn NotFound(route: Vec<String>) -> Element {
    let path = format!("/{}", route.join("/"));
    rsx! {
        div { class: "flex flex-col gap-4",
            p { "Page not found: {path}" }
            Link { to: Route::HomePage {}, class: NAV_LINK, {t!("nav-calendar")} }
        }
    }
}

/// Sends the browser to the identity provider and back to `/`.
///
/// `/auth` is served by the auth layer; it redirects straight to the provider, so the
/// user does not see an intermediate page.
fn sign_in() {
    if let Err(err) = window().location().set_href("/auth?redirect=%2F") {
        tracing::error!("could not start sign-in: {err:?}");
    }
}

/// Whether a request failed because nobody is signed in.
fn is_signed_out<T>(result: &api::ApiResult<T>) -> bool {
    matches!(result, Err(api::ApiError::Status { code: 401, .. }))
}

/// Page frame shared by every route: header, navigation and the current page.
#[component]
pub fn Shell() -> Element {
    let settings = use_resource(|| async { api::get_settings().await });
    let me = use_resource(|| async { api::get_me().await });
    let version = use_resource(|| async { api::get_version().await });
    let mut i18n = i18n();

    // Not signed in: go to the identity provider right away.
    use_effect(move || {
        if let Some(result) = &*me.read() {
            if is_signed_out(result) {
                sign_in();
            }
        }
    });

    // Once the saved settings arrive, apply their theme and language.
    // Only the resource is read here, so changes made in the header do not get reverted.
    use_effect(move || {
        if let Some(Ok(current)) = &*settings.read() {
            STATE.write().theme = current.theme.clone();
            STATE.write().language = to_app_language(current.language);
            STATE.write().show_email = current.show_email;
            i18n.set_language(to_app_language(current.language).into());
        }
    });

    let version_label = match &*version.read() {
        Some(Ok(info)) => format!("build {}", info.version),
        _ => String::new(),
    };

    // The name is shown unless the user chose to show the email instead.
    let show_email = STATE.read().show_email;
    let label = match &*me.read() {
        Some(Ok(account)) if show_email => account.email.clone(),
        Some(Ok(account)) => account.name.clone(),
        _ => None,
    };

    rsx! {
        div { class: "container mx-auto max-w-5xl px-4 my-4",
            header { class: "flex flex-wrap items-center gap-2 py-2",
                span { class: "text-2xl font-bold mr-auto", {t!("title")} }
                if let Some(label) = label {
                    span { class: "text-sm text-neutral-600 dark:text-neutral-300 break-all", "{label}" }
                }
            }
            nav { class: "flex flex-wrap gap-2 mb-4 print:hidden",
                Link { to: Route::ReportsPage {}, class: NAV_LINK, {t!("nav-reports")} }
                Link { to: Route::HomePage {}, class: NAV_LINK, {t!("nav-calendar")} }
                Link { to: Route::ProjectsPage {}, class: NAV_LINK, {t!("nav-projects")} }
                Link { to: Route::AbsencesPage {}, class: NAV_LINK, {t!("nav-absences")} }
                Link { to: Route::HolidaysPage {}, class: NAV_LINK, {t!("nav-holidays")} }
                Link { to: Route::TokensPage {}, class: NAV_LINK, {t!("nav-tokens")} }
                Link { to: Route::SettingsPage {}, class: NAV_LINK, {t!("nav-settings")} }
            }
            main { Outlet::<Route> {} }
            footer { class: "mt-8 text-xs text-neutral-500 dark:text-neutral-400 text-center print:hidden",
                {version_label}
            }
        }
    }
}
