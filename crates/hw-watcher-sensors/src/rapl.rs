// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Sysfs RAPL CPU package energy reader (`/sys/class/powercap/intel-rapl:*`).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use hw_watcher_core::{CpuPowerSource, compute_rapl_power_w};

const POWERCAP_ROOT: &str = "/sys/class/powercap";
const DEFAULT_RAPL_DIR: &str = "/sys/class/powercap/intel-rapl:0";

struct RaplPackage {
    energy_path: PathBuf,
    max_range_uj: u64,
    previous: Option<(u64, Instant)>,
}

impl RaplPackage {
    fn open_at(dir: &Path) -> Option<Self> {
        let energy_path = dir.join("energy_uj");
        let initial = read_u64(&energy_path)?;
        let max_range_uj = read_u64(&dir.join("max_energy_range_uj")).unwrap_or(0);
        Some(Self {
            energy_path,
            max_range_uj,
            previous: Some((initial, Instant::now())),
        })
    }

    fn read_w(&mut self) -> Option<f64> {
        let now = Instant::now();
        let current = read_u64(&self.energy_path)?;
        let prev = self.previous.replace((current, now))?;
        let dt_s = now.saturating_duration_since(prev.1).as_secs_f64();
        compute_rapl_power_w(prev.0, current, self.max_range_uj, dt_s)
    }
}

/// Reads CPU package power in watts from one or more sysfs RAPL package counters.
pub struct SysfsRaplSource {
    packages: Vec<RaplPackage>,
}

impl SysfsRaplSource {
    /// Opens all readable `/sys/class/powercap/intel-rapl:<N>` packages (or `None` if none).
    pub fn open() -> Option<Self> {
        Self::open_all_at(Path::new(POWERCAP_ROOT))
            .or_else(|| Self::open_at(Path::new(DEFAULT_RAPL_DIR)))
    }

    /// Opens a single RAPL domain directory at `dir` (returns `None` on error or missing counter).
    pub fn open_at(dir: &Path) -> Option<Self> {
        RaplPackage::open_at(dir).map(|pkg| Self {
            packages: vec![pkg],
        })
    }

    /// Discovers all `intel-rapl:<N>` package directories under `root`.
    pub fn open_all_at(root: &Path) -> Option<Self> {
        let entries = fs::read_dir(root).ok()?;
        let mut dirs: Vec<(usize, PathBuf)> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name();
                let s = name.to_str()?;
                let suffix = s.strip_prefix("intel-rapl:")?;
                if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                let idx = suffix.parse::<usize>().ok()?;
                Some((idx, e.path()))
            })
            .collect();
        dirs.sort_by_key(|(idx, _)| *idx);
        let packages: Vec<RaplPackage> = dirs
            .into_iter()
            .filter_map(|(_, dir)| RaplPackage::open_at(&dir))
            .collect();
        (!packages.is_empty()).then_some(Self { packages })
    }

    /// Reads package power (W) for each discovered CPU socket/package.
    pub fn read_all_packages_w(&mut self) -> Vec<Option<f64>> {
        self.packages.iter_mut().map(RaplPackage::read_w).collect()
    }
}

impl CpuPowerSource for SysfsRaplSource {
    fn read_package_w(&mut self) -> Option<f64> {
        let readings = self.read_all_packages_w();
        let mut vals = readings.into_iter().flatten();
        let first = vals.next()?;
        Some(vals.fold(first, |a, b| a + b))
    }
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rapl_power_across_counter_wraparound() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("energy_uj"), "990000\n").unwrap();
        fs::write(dir.path().join("max_energy_range_uj"), "1000000\n").unwrap();

        let mut rapl = SysfsRaplSource::open_at(dir.path()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(dir.path().join("energy_uj"), "10000\n").unwrap();

        let watts = rapl.read_package_w().unwrap();
        assert!(watts.is_finite() && watts > 0.0);
    }

    #[test]
    fn returns_none_when_rapl_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(SysfsRaplSource::open_at(dir.path()).is_none());
    }

    #[test]
    fn discovers_multiple_rapl_packages_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let p0 = dir.path().join("intel-rapl:0");
        let p1 = dir.path().join("intel-rapl:1");
        let sub = dir.path().join("intel-rapl:0:0");
        fs::create_dir_all(&p0).unwrap();
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&sub).unwrap();
        fs::write(p0.join("energy_uj"), "100000\n").unwrap();
        fs::write(p1.join("energy_uj"), "200000\n").unwrap();
        fs::write(sub.join("energy_uj"), "50000\n").unwrap();

        let mut rapl = SysfsRaplSource::open_all_at(dir.path()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(p0.join("energy_uj"), "120000\n").unwrap();
        fs::write(p1.join("energy_uj"), "240000\n").unwrap();

        let all = rapl.read_all_packages_w();
        assert_eq!(all.len(), 2);
        assert!(all[0].unwrap() > 0.0);
        assert!(all[1].unwrap() > all[0].unwrap());
    }
}
