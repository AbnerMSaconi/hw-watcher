// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! AMD (`amdgpu`) and Intel (`i915` / `xe`) GPU telemetry from sysfs (`/sys/class/drm`).
//!
//! Needs no vendor library. NVIDIA GPUs are skipped here because NVML already covers them.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use hw_watcher_core::GpuSample;

const DRM_ROOT: &str = "/sys/class/drm";
const MICRO: f64 = 1_000_000.0;
const MILLI: f64 = 1000.0;
const BYTES_PER_MIB: f64 = 1024.0 * 1024.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vendor {
    Amd,
    Intel,
}

struct DrmGpu {
    vendor: Vendor,
    device: PathBuf,
    hwmon: Option<PathBuf>,
    /// Last `(energy_uj, time)` for drivers that only expose an energy counter.
    energy_prev: Option<(f64, Instant)>,
}

/// Reads every non-NVIDIA GPU exposed through DRM sysfs.
pub struct SysfsDrmGpuSource {
    gpus: Vec<DrmGpu>,
}

impl SysfsDrmGpuSource {
    /// Discovers AMD and Intel GPUs under `/sys/class/drm` (returns `None` when there are none).
    pub fn open() -> Option<Self> {
        Self::open_at(Path::new(DRM_ROOT))
    }

    /// Discovers GPUs under a custom DRM root (used for fixture tests).
    pub fn open_at(root: &Path) -> Option<Self> {
        let mut cards: Vec<PathBuf> = fs::read_dir(root)
            .ok()?
            .flatten()
            .filter(|e| is_card_name(&e.file_name().to_string_lossy()))
            .map(|e| e.path())
            .collect();
        cards.sort();
        let gpus: Vec<DrmGpu> = cards.iter().filter_map(|c| open_card(c)).collect();
        (!gpus.is_empty()).then_some(Self { gpus })
    }

    /// Number of discovered GPUs.
    pub fn len(&self) -> usize {
        self.gpus.len()
    }

    /// Returns `true` when no GPU was discovered.
    pub fn is_empty(&self) -> bool {
        self.gpus.is_empty()
    }

    /// Reads current telemetry for every discovered GPU.
    pub fn read_all(&mut self) -> Vec<GpuSample> {
        self.gpus.iter_mut().map(DrmGpu::read).collect()
    }
}

fn is_card_name(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

fn open_card(card: &Path) -> Option<DrmGpu> {
    let device = card.join("device");
    let uevent = fs::read_to_string(device.join("uevent")).ok()?;
    let driver = uevent
        .lines()
        .find_map(|l| l.strip_prefix("DRIVER="))?
        .trim();
    let vendor = match driver {
        "amdgpu" => Vendor::Amd,
        "i915" | "xe" => Vendor::Intel,
        _ => return None,
    };
    let hwmon = fs::read_dir(device.join("hwmon"))
        .ok()
        .and_then(|rd| rd.flatten().map(|e| e.path()).min());
    Some(DrmGpu {
        vendor,
        device,
        hwmon,
        energy_prev: None,
    })
}

impl DrmGpu {
    fn hwmon_value(&self, file: &str) -> Option<f64> {
        read_f64(&self.hwmon.as_ref()?.join(file))
    }

    /// Temperature of the hwmon channel whose label matches `label`, if any.
    fn labeled_temp(&self, label: &str) -> Option<f64> {
        let dir = self.hwmon.as_ref()?;
        (1..=8).find_map(|i| {
            let name = fs::read_to_string(dir.join(format!("temp{i}_label"))).ok()?;
            (name.trim() == label)
                .then(|| read_f64(&dir.join(format!("temp{i}_input"))))
                .flatten()
                .map(|v| v / MILLI)
        })
    }

    fn read(&mut self) -> GpuSample {
        match self.vendor {
            Vendor::Amd => self.read_amd(),
            Vendor::Intel => self.read_intel(),
        }
    }

    fn read_amd(&self) -> GpuSample {
        let vram_used = read_f64(&self.device.join("mem_info_vram_used"));
        let vram_total = read_f64(&self.device.join("mem_info_vram_total"));
        let power = self
            .hwmon_value("power1_average")
            .or_else(|| self.hwmon_value("power1_input"))
            .map(|uw| uw / MICRO);
        GpuSample {
            temp_c: self
                .labeled_temp("edge")
                .or_else(|| self.hwmon_value("temp1_input").map(|v| v / MILLI)),
            power_w: power,
            avg_power_w: power,
            power_limit_w: self
                .hwmon_value("power1_cap")
                .map(|uw| uw / MICRO)
                .filter(|w| *w > 0.0),
            usage_pct: read_f64(&self.device.join("gpu_busy_percent")),
            clock_mhz: self.hwmon_value("freq1_input").map(|hz| hz / MICRO),
            clock_mem_mhz: self.hwmon_value("freq2_input").map(|hz| hz / MICRO),
            vram_used_mib: vram_used.map(|b| b / BYTES_PER_MIB),
            vram_total_mib: vram_total.map(|b| b / BYTES_PER_MIB),
            voltage_v: self.hwmon_value("in0_input").map(|mv| mv / MILLI),
            vram_temp_c: self.labeled_temp("mem"),
            hotspot_temp_c: self.labeled_temp("junction"),
        }
    }

    fn read_intel(&mut self) -> GpuSample {
        let power = self
            .hwmon_value("power1_input")
            .map(|uw| uw / MICRO)
            .or_else(|| self.power_from_energy());
        let temp = self
            .labeled_temp("pkg")
            .or_else(|| self.hwmon_value("temp1_input").map(|v| v / MILLI))
            .or_else(|| self.hwmon_value("temp2_input").map(|v| v / MILLI));
        GpuSample {
            temp_c: temp,
            power_w: power,
            avg_power_w: power,
            power_limit_w: self
                .hwmon_value("power1_max")
                .map(|uw| uw / MICRO)
                .filter(|w| *w > 0.0),
            clock_mhz: self.intel_freq_mhz(),
            vram_temp_c: self.labeled_temp("vram"),
            ..GpuSample::default()
        }
    }

    fn power_from_energy(&mut self) -> Option<f64> {
        let now = Instant::now();
        let current = self.hwmon_value("energy1_input")?;
        let prev = self.energy_prev.replace((current, now))?;
        let dt = now.saturating_duration_since(prev.1).as_secs_f64();
        (dt > 0.0 && current >= prev.0).then(|| (current - prev.0) / MICRO / dt)
    }

    fn intel_freq_mhz(&self) -> Option<f64> {
        let card = self.device.parent()?;
        read_f64(&card.join("gt_cur_freq_mhz"))
            .or_else(|| read_f64(&self.device.join("tile0/gt0/freq0/cur_freq")))
    }
}

fn read_f64(path: &Path) -> Option<f64> {
    fs::read_to_string(path).ok()?.trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn reads_amdgpu_card() {
        let dir = tempfile::tempdir().unwrap();
        let dev = dir.path().join("card0/device");
        write(
            &dev.join("uevent"),
            "DRIVER=amdgpu\nPCI_SLOT_NAME=0000:03:00.0\n",
        );
        write(&dev.join("gpu_busy_percent"), "37\n");
        write(&dev.join("mem_info_vram_used"), "1073741824\n");
        write(&dev.join("mem_info_vram_total"), "8589934592\n");
        let hw = dev.join("hwmon/hwmon4");
        write(&hw.join("temp1_label"), "edge\n");
        write(&hw.join("temp1_input"), "55000\n");
        write(&hw.join("temp2_label"), "junction\n");
        write(&hw.join("temp2_input"), "62000\n");
        write(&hw.join("temp3_label"), "mem\n");
        write(&hw.join("temp3_input"), "58000\n");
        write(&hw.join("power1_average"), "112500000\n");
        write(&hw.join("power1_cap"), "180000000\n");
        write(&hw.join("freq1_input"), "1800000000\n");
        write(&hw.join("freq2_input"), "1000000000\n");
        // Connector entries and non-card directories must be ignored.
        write(&dir.path().join("card0-HDMI-A-1/status"), "connected\n");

        let mut src = SysfsDrmGpuSource::open_at(dir.path()).unwrap();
        assert_eq!(src.len(), 1);
        let g = src.read_all()[0];
        assert_eq!(g.temp_c, Some(55.0));
        assert_eq!(g.hotspot_temp_c, Some(62.0));
        assert_eq!(g.vram_temp_c, Some(58.0));
        assert_eq!(g.power_w, Some(112.5));
        assert_eq!(g.power_limit_w, Some(180.0));
        assert_eq!(g.usage_pct, Some(37.0));
        assert_eq!(g.clock_mhz, Some(1800.0));
        assert_eq!(g.vram_used_mib, Some(1024.0));
        assert_eq!(g.vram_total_mib, Some(8192.0));
        assert_eq!(g.vram_pct(), Some(12.5));
    }

    #[test]
    fn reads_intel_card_and_skips_nvidia() {
        let dir = tempfile::tempdir().unwrap();
        let intel = dir.path().join("card0/device");
        write(&intel.join("uevent"), "DRIVER=i915\n");
        write(&intel.join("hwmon/hwmon2/temp1_input"), "47000\n");
        write(&intel.join("hwmon/hwmon2/power1_input"), "9000000\n");
        write(&dir.path().join("card0/gt_cur_freq_mhz"), "650\n");
        write(&dir.path().join("card1/device/uevent"), "DRIVER=nvidia\n");

        let mut src = SysfsDrmGpuSource::open_at(dir.path()).unwrap();
        assert_eq!(src.len(), 1);
        let g = src.read_all()[0];
        assert_eq!(g.temp_c, Some(47.0));
        assert_eq!(g.power_w, Some(9.0));
        assert_eq!(g.clock_mhz, Some(650.0));
        assert_eq!(g.vram_total_mib, None);
    }

    #[test]
    fn missing_drm_root_yields_none() {
        assert!(SysfsDrmGpuSource::open_at(Path::new("/nonexistent/drm")).is_none());
    }
}
