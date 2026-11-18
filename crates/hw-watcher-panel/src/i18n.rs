// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Embedded UI translation catalog (English source, Portuguese catalog).

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};

use hw_watcher_core::Translator;

include!(concat!(env!("OUT_DIR"), "/catalog_pt.rs"));

static PORTUGUESE: AtomicBool = AtomicBool::new(false);

/// Checks `LC_ALL`, `LC_MESSAGES`, and `LANG`; any `pt*` locale selects Portuguese.
pub fn detect_portuguese() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|var| std::env::var(var).ok())
        .find(|val| !val.is_empty())
        .is_some_and(|loc| loc.to_ascii_lowercase().starts_with("pt"))
}

/// Sets the active language globally for Slint and [`PoTranslator`].
pub fn set_portuguese(portuguese: bool) {
    PORTUGUESE.store(portuguese, Ordering::Relaxed);
}

/// Returns whether Portuguese is currently selected.
pub fn is_portuguese() -> bool {
    PORTUGUESE.load(Ordering::Relaxed)
}

/// Applies the active language to Slint `@tr(...)` strings.
pub fn apply_to_slint() {
    let lang = if is_portuguese() { "pt" } else { "" };
    if let Err(e) = slint::select_bundled_translation(lang) {
        tracing::warn!("UI translation unavailable: {e}");
    }
}

/// Translator backed by the `.po` catalog compiled into the binary.
#[derive(Debug, Default, Clone, Copy)]
pub struct PoTranslator {
    portuguese: bool,
}

impl PoTranslator {
    /// Creates a translator reflecting the current global language flag.
    pub fn active() -> Self {
        Self {
            portuguese: is_portuguese(),
        }
    }

    /// Creates a translator explicitly configured for Portuguese (`true`) or English (`false`).
    #[cfg(test)]
    pub const fn new(portuguese: bool) -> Self {
        Self { portuguese }
    }
}

impl Translator for PoTranslator {
    fn translate<'a>(&self, key: &'a str) -> Cow<'a, str> {
        if self.portuguese
            && let Some((_, pt)) = CATALOG_PT.iter().find(|(en, _)| *en == key)
        {
            return Cow::Borrowed(pt);
        }
        Cow::Borrowed(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markers(s: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut rest = s;
        while let Some(i) = rest.find('{') {
            let Some(j) = rest[i..].find('}') else {
                break;
            };
            out.push(&rest[i..=i + j]);
            rest = &rest[i + j + 1..];
        }
        out.sort_unstable();
        out
    }

    #[test]
    fn catalog_preserves_placeholders_and_unique_keys() {
        for (i, (en, pt)) in CATALOG_PT.iter().enumerate() {
            assert_eq!(markers(en), markers(pt), "placeholder mismatch for {en}");
            assert!(
                CATALOG_PT[i + 1..].iter().all(|(other, _)| other != en),
                "duplicate catalog key: {en}"
            );
        }
    }

    #[test]
    fn translates_portuguese_and_keeps_english_source() {
        assert_eq!(PoTranslator::new(false).translate("Hide"), "Hide");
        assert_eq!(PoTranslator::new(true).translate("Hide"), "Esconder");
    }
}
