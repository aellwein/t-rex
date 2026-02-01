//! Shared style classes and small UI types.
//!
//! Touch targets are at least 44px tall (`min-h-11`) so the app works on phones.

pub const PAGE_TITLE: &str = "text-xl font-bold";
pub const CARD: &str = "rounded-xl border border-neutral-300 dark:border-neutral-500 p-4";
pub const LABEL: &str = "block text-sm font-medium";
pub const INPUT: &str = "w-full min-h-11 rounded-lg border border-neutral-400 bg-white px-3 py-2 dark:bg-neutral-800 dark:border-neutral-500";
pub const BUTTON: &str = "min-h-11 rounded-lg border border-neutral-400 bg-white px-4 py-2 hover:bg-neutral-200 dark:bg-neutral-700 dark:hover:bg-neutral-600 cursor-pointer";
pub const BUTTON_PRIMARY: &str =
    "min-h-11 rounded-lg bg-teal-600 px-4 py-2 text-white hover:bg-teal-700 cursor-pointer";
pub const BUTTON_DANGER: &str = "min-h-11 rounded-lg border border-red-500 px-4 py-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-950 cursor-pointer";
pub const NAV_LINK: &str = "min-h-11 inline-flex items-center rounded-lg border border-neutral-400 px-4 py-2 hover:bg-neutral-200 dark:hover:bg-neutral-600";
pub const ERROR_TEXT: &str = "text-red-600 dark:text-red-400";
pub const SUCCESS_TEXT: &str = "text-teal-600 dark:text-teal-400";

/// Which action failed. Lets a page show a translated prefix chosen at render time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failed {
    Save,
    Delete,
}
