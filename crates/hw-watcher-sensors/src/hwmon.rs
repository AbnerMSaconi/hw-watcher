// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Sysfs `hwmon` reader (`k10temp` CPU Tctl and `nct6687` motherboard sensors resolved by label).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use hw_watcher_core::{BoardSample, BoardSource};

const HWMON_ROOT: &str = "/sys/class/hwmon";
const MILLI_SCALE: f64 = 1000.0;

/// Reads CPU Tctl/Package temperatures and motherboard sensors from sysfs `hwmon`.
pub struct SysfsHwmonSource {
    cpu_temps: Vec<PathBuf>,
    nct: Option<NctPaths>,
}

struct NctPaths {
    v12: Option<PathBuf>,
    v5: Option<PathBuf>,
    v3_3: Option<PathBuf>,
    vcore: Option<PathBuf>,
    soc: Option<PathBuf>,
    dram: Option<PathBuf>,
    vrm_mos: Option<PathBuf>,
    cpu_socket: Option<PathBuf>,
    system: Option<PathBuf>,
    pch: Option<PathBuf>,
    pcie_x1: Option<PathBuf>,
    cpu_fan: PathBuf,
    sys_fan1: PathBuf,
    sys_fan3: PathBuf,
}

impl SysfsHwmonSource {
    /// Discovers `k10temp` and `nct6687` under `/sys/class/hwmon`.
    pub fn open() -> Self {
        Self::open_at(Path::new(HWMON_ROOT))
    }

    /// Discovers sensors under a custom `hwmon` root (used for fixture tests).
    pub fn open_at(root: &Path) -> Self {
        let mut cpu_temps: Vec<PathBuf> =
            find_all_hwmon(root, &["k10temp", "zenpower", "coretemp"])
                .into_iter()
                .map(|d| {
                    let temps = map_by_label(&d, "temp");
                    temps
                        .get("Tctl")
                        .or_else(|| temps.get("Package id 0"))
                        .or_else(|| temps.get("Package id 1"))
                        .cloned()
                        .unwrap_or_else(|| d.join("temp1_input"))
                })
                .collect();
        cpu_temps.retain(|p| p.exists());
        let nct = find_hwmon(root, "nct6687")
            .map(|d| build_nct_paths(&d))
            .or_else(|| find_generic_board(root).map(|d| build_generic_paths(&d)));
        Self { cpu_temps, nct }
    }

    /// Returns `true` if at least one CPU `hwmon` temperature sensor was discovered.
    pub fn has_cpu(&self) -> bool {
        !self.cpu_temps.is_empty()
    }

    /// Returns `true` if `nct6687` was discovered.
    pub fn has_board(&self) -> bool {
        self.nct.is_some()
    }

    /// Reads per-CPU temperatures for every discovered CPU `hwmon` sensor.
    pub fn read_all_cpu_temps(&self) -> Vec<Option<f64>> {
        self.cpu_temps
            .iter()
            .map(|p| read_milli(p.as_path()))
            .collect()
    }
}

impl BoardSource for SysfsHwmonSource {
    fn read(&self) -> (Option<f64>, BoardSample) {
        let tctl = self
            .read_all_cpu_temps()
            .into_iter()
            .flatten()
            .reduce(f64::max);
        let board = self
            .nct
            .as_ref()
            .map_or_else(BoardSample::default, read_nct);
        (tctl, board)
    }
}

fn build_nct_paths(dir: &Path) -> NctPaths {
    let volts = map_by_label(dir, "in");
    let temps = map_by_label(dir, "temp");
    NctPaths {
        v12: volts.get("+12V").cloned(),
        v5: volts.get("+5V").cloned(),
        v3_3: volts.get("+3.3V").cloned(),
        vcore: volts.get("CPU Vcore").cloned(),
        soc: volts.get("CPU Soc").cloned(),
        dram: volts.get("DRAM").cloned(),
        vrm_mos: temps.get("VRM MOS").cloned(),
        cpu_socket: temps.get("CPU Socket").cloned(),
        system: temps.get("System").cloned(),
        pch: temps.get("PCH").cloned(),
        pcie_x1: temps.get("PCIe x1").cloned(),
        cpu_fan: dir.join("fan1_input"),
        sys_fan1: dir.join("fan3_input"),
        sys_fan3: dir.join("fan5_input"),
    }
}

/// Super-I/O chips with a mainline driver whose temperatures and fans are mapped generically.
const GENERIC_BOARD_CHIPS: &[&str] = &[
    "nct6683", "nct6775", "nct6776", "nct6779", "nct6791", "nct6792", "nct6793", "nct6795",
    "nct6796", "nct6797", "nct6798", "nct6799", "it8620", "it8628", "it8686", "it8688", "it8689",
    "it8705", "it8712", "it8716", "it8718", "it8720", "it8721", "it8728", "it8732", "it8736",
    "it8771", "it8772", "it8781", "it8782", "it8783", "it8790", "it87952",
];

fn find_generic_board(root: &Path) -> Option<PathBuf> {
    find_all_hwmon(root, GENERIC_BOARD_CHIPS).into_iter().next()
}

/// Maps the common temperature labels and the first three fans of a generic Super-I/O chip.
///
/// Voltage rails are left empty: their channel order differs per motherboard and cannot be
/// guessed safely.
fn build_generic_paths(dir: &Path) -> NctPaths {
    let temps = map_by_label(dir, "temp");
    let pick = |names: &[&str]| names.iter().find_map(|n| temps.get(*n).cloned());
    NctPaths {
        v12: None,
        v5: None,
        v3_3: None,
        vcore: None,
        soc: None,
        dram: None,
        vrm_mos: pick(&["VRM MOS", "VRM"]),
        cpu_socket: pick(&["CPU Socket", "CPUTIN", "CPU"]),
        system: pick(&["System", "SYSTIN", "Motherboard"]),
        pch: pick(&["PCH", "PCH_CHIP_TEMP", "PCHTIN"]),
        pcie_x1: None,
        cpu_fan: dir.join("fan1_input"),
        sys_fan1: dir.join("fan2_input"),
        sys_fan3: dir.join("fan3_input"),
    }
}

fn read_nct(n: &NctPaths) -> BoardSample {
    BoardSample {
        v12_v: n.v12.as_deref().and_then(read_milli),
        v5_v: n.v5.as_deref().and_then(read_milli),
        v3_3_v: n.v3_3.as_deref().and_then(read_milli),
        vcore_v: n.vcore.as_deref().and_then(read_milli),
        soc_v: n.soc.as_deref().and_then(read_milli),
        dram_v: n.dram.as_deref().and_then(read_milli),
        vrm_mos_c: n.vrm_mos.as_deref().and_then(read_milli),
        cpu_socket_c: n.cpu_socket.as_deref().and_then(read_milli),
        system_c: n.system.as_deref().and_then(read_milli),
        pch_c: n.pch.as_deref().and_then(read_milli),
        pcie_x1_c: n.pcie_x1.as_deref().and_then(read_milli),
        cpu_fan_rpm: read_integer(&n.cpu_fan),
        sys_fan1_rpm: read_integer(&n.sys_fan1),
        sys_fan3_rpm: read_integer(&n.sys_fan3),
    }
}

fn find_hwmon(root: &Path, target: &str) -> Option<PathBuf> {
    find_all_hwmon(root, &[target]).into_iter().next()
}

fn find_all_hwmon(root: &Path, targets: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    dirs.sort();
    dirs.into_iter()
        .filter(|dir| {
            fs::read_to_string(dir.join("name")).is_ok_and(|name| targets.contains(&name.trim()))
        })
        .collect()
}

fn map_by_label(dir: &Path, prefix: &str) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.starts_with(prefix) || !name.ends_with("_label") {
            continue;
        }
        let Ok(label) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let input = dir.join(name.replace("_label", "_input"));
        if input.exists() {
            out.insert(label.trim().to_owned(), input);
        }
    }
    out
}

fn read_milli(path: &Path) -> Option<f64> {
    let raw = fs::read_to_string(path).ok()?.trim().parse::<f64>().ok()?;
    Some(raw / MILLI_SCALE)
}

fn read_integer(path: &Path) -> Option<f64> {
    fs::read_to_string(path).ok()?.trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_k10temp_and_nct6687_by_label() {
        let dir = tempfile::tempdir().unwrap();
        let k10 = dir.path().join("hwmon0");
        let nct = dir.path().join("hwmon1");
        fs::create_dir_all(&k10).unwrap();
        fs::create_dir_all(&nct).unwrap();

        fs::write(k10.join("name"), "k10temp\n").unwrap();
        fs::write(k10.join("temp1_label"), "Tctl\n").unwrap();
        fs::write(k10.join("temp1_input"), "56250\n").unwrap();

        fs::write(nct.join("name"), "nct6687\n").unwrap();
        fs::write(nct.join("in0_label"), "+12V\n").unwrap();
        fs::write(nct.join("in0_input"), "12048\n").unwrap();
        fs::write(nct.join("in4_label"), "DRAM\n").unwrap();
        fs::write(nct.join("in4_input"), "1212\n").unwrap();
        fs::write(nct.join("temp1_label"), "VRM MOS\n").unwrap();
        fs::write(nct.join("temp1_input"), "41000\n").unwrap();
        fs::write(nct.join("fan1_input"), "1385\n").unwrap();

        let sensor = SysfsHwmonSource::open_at(dir.path());
        assert!(sensor.has_cpu() && sensor.has_board());
        let (tctl, board) = sensor.read();
        assert_eq!(tctl, Some(56.25));
        assert_eq!(board.v12_v, Some(12.048));
        assert_eq!(board.dram_v, Some(1.212));
        assert_eq!(board.vrm_mos_c, Some(41.0));
        assert_eq!(board.cpu_fan_rpm, Some(1385.0));
        assert_eq!(board.sys_fan1_rpm, None);
    }

    #[test]
    fn maps_generic_superio_temps_and_fans() {
        let dir = tempfile::tempdir().unwrap();
        let chip = dir.path().join("hwmon0");
        fs::create_dir_all(&chip).unwrap();
        fs::write(chip.join("name"), "nct6798\n").unwrap();
        fs::write(chip.join("temp1_label"), "SYSTIN\n").unwrap();
        fs::write(chip.join("temp1_input"), "33000\n").unwrap();
        fs::write(chip.join("temp2_label"), "CPUTIN\n").unwrap();
        fs::write(chip.join("temp2_input"), "38000\n").unwrap();
        fs::write(chip.join("fan1_input"), "900\n").unwrap();
        fs::write(chip.join("fan2_input"), "1100\n").unwrap();

        let sensor = SysfsHwmonSource::open_at(dir.path());
        assert!(sensor.has_board());
        let (_, board) = sensor.read();
        assert_eq!(board.system_c, Some(33.0));
        assert_eq!(board.cpu_socket_c, Some(38.0));
        assert_eq!(board.cpu_fan_rpm, Some(900.0));
        assert_eq!(board.sys_fan1_rpm, Some(1100.0));
        assert_eq!(board.v12_v, None);
    }
}
