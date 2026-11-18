// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Authorship, license and provenance notice (the program's "Appropriate Legal Notices").
//!
//! The same text is shown by `hw-watcher about`, in the Control Panel footer and is embedded in the
//! binary, so every distributed copy carries the creator's identity. `NOTICE` requires derivative
//! works to preserve it (GNU GPL v3, sections 5(d) and 7(b)).

use std::hint::black_box;

/// Name of the original author and copyright holder.
pub const AUTHOR: &str = "AbnerMSaconi";
/// SPDX license identifier of the original work.
pub const LICENSE: &str = "GPL-3.0-only";
/// Home of the original project.
pub const REPOSITORY: &str = "https://github.com/AbnerMSaconi/hw-watcher";

/// Machine-greppable provenance record embedded in the binary (`strings hw-watcher | grep hw-watcher-provenance`).
static PROVENANCE: &str = concat!(
    "hw-watcher-provenance;name=hw-watcher;author=AbnerMSaconi;license=GPL-3.0-only;",
    "repo=https://github.com/AbnerMSaconi/hw-watcher;version=",
    env!("CARGO_PKG_VERSION"),
    ";commit=",
    env!("HW_WATCHER_COMMIT")
);

/// One-line notice for the Control Panel footer.
pub fn short_notice() -> String {
    format!("(c) 2026 {AUTHOR} · {LICENSE}")
}

/// Full notice printed by `hw-watcher about`.
pub fn full_notice() -> String {
    format!(
        "HW Watcher {} (commit {})\n\
         Copyright (C) 2026 {AUTHOR}\n\
         License: GNU General Public License v3.0 only (see LICENSE and NOTICE)\n\
         Original project: {REPOSITORY}\n\
         \n\
         This is free software: you may redistribute and modify it under the GPL v3, keeping this\n\
         notice and the author attribution. It comes with ABSOLUTELY NO WARRANTY.\n\
         \n\
         provenance: {}",
        env!("CARGO_PKG_VERSION"),
        env!("HW_WATCHER_COMMIT"),
        black_box(PROVENANCE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_carry_author_license_and_origin() {
        let full = full_notice();
        assert!(full.contains("Copyright (C) 2026 AbnerMSaconi"));
        assert!(full.contains("GNU General Public License v3.0"));
        assert!(full.contains(REPOSITORY));
        assert!(full.contains("hw-watcher-provenance;"));
        assert!(short_notice().contains(AUTHOR));
    }
}
