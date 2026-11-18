// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Pure parsers and decoders for RAPL energy counters, `/proc/meminfo`, and BAR0 registers.

use crate::model::MemorySample;

const KIB_PER_MIB: f64 = 1024.0;
const UJ_PER_JOULE: f64 = 1_000_000.0;
const BAR0_INVALID: u32 = 0xFFFF_FFFF;
const BAR0_BADF_MASK: u32 = 0xFFFF_0000;
const BAR0_BADF_VAL: u32 = 0xBADF_0000;
const BAR0_MAX_CELSIUS: u32 = 120;

/// Computes average package power in watts from two RAPL energy readings (`uJ`) over `dt_s` seconds.
///
/// Handles counter wrap-around when `max_range_uj > prev_uj`.
pub fn compute_rapl_power_w(
    prev_uj: u64,
    now_uj: u64,
    max_range_uj: u64,
    dt_s: f64,
) -> Option<f64> {
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return None;
    }
    let delta_uj = if now_uj >= prev_uj {
        now_uj - prev_uj
    } else if max_range_uj > prev_uj {
        (max_range_uj - prev_uj) + now_uj
    } else {
        return None;
    };
    Some((delta_uj as f64) / UJ_PER_JOULE / dt_s)
}

/// Parses `/proc/meminfo` text into [`MemorySample`] (`used = MemTotal - MemAvailable`).
///
/// Falls back to `MemTotal - MemFree` only if `MemAvailable` is absent.
pub fn parse_meminfo(raw: &str) -> MemorySample {
    let mut total_kib: Option<u64> = None;
    let mut avail_kib: Option<u64> = None;
    let mut free_kib: Option<u64> = None;

    for line in raw.lines() {
        if let Some(v) = parse_kib_field(line, "MemTotal:") {
            total_kib = Some(v);
        } else if let Some(v) = parse_kib_field(line, "MemAvailable:") {
            avail_kib = Some(v);
        } else if let Some(v) = parse_kib_field(line, "MemFree:") {
            free_kib = Some(v);
        }
    }

    let Some(total) = total_kib else {
        return MemorySample::default();
    };
    let avail = avail_kib.or(free_kib);
    let used_mib = avail.map(|a| (total.saturating_sub(a) as f64) / KIB_PER_MIB);
    MemorySample {
        used_mib,
        total_mib: Some((total as f64) / KIB_PER_MIB),
    }
}

fn parse_kib_field(line: &str, prefix: &str) -> Option<u64> {
    let rest = line.strip_prefix(prefix)?.trim();
    rest.split_whitespace().next()?.parse::<u64>().ok()
}

fn bar0_valid(raw: u32) -> bool {
    raw != BAR0_INVALID && (raw & BAR0_BADF_MASK) != BAR0_BADF_VAL
}

/// Decodes GA102 BAR0 VRAM junction temperature (`offset 0xE2A8`, `(raw & 0xFFF) / 32`).
pub fn decode_bar0_vram_c(raw: u32) -> Option<f64> {
    if !bar0_valid(raw) {
        return None;
    }
    let celsius = (raw & 0xFFF) / 32;
    (celsius < BAR0_MAX_CELSIUS).then_some(f64::from(celsius))
}

/// Decodes GA102 BAR0 GPU hotspot temperature (`offset 0x2046C`, `(raw >> 8) & 0xFF`).
pub fn decode_bar0_hotspot_c(raw: u32) -> Option<f64> {
    if !bar0_valid(raw) {
        return None;
    }
    let celsius = (raw >> 8) & 0xFF;
    (celsius < BAR0_MAX_CELSIUS).then_some(f64::from(celsius))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapl_delta_and_wraparound() {
        let w = compute_rapl_power_w(100_000_000, 125_000_000, 65_532_610_987, 1.0).unwrap();
        assert!((w - 25.0).abs() < 1e-9);

        let wrapped = compute_rapl_power_w(950_000, 50_000, 1_000_000, 0.1).unwrap();
        assert!((wrapped - 1.0).abs() < 1e-9);

        assert_eq!(compute_rapl_power_w(100, 200, 1000, 0.0), None);
        assert_eq!(compute_rapl_power_w(900, 100, 0, 1.0), None);
    }

    #[test]
    fn parses_real_proc_meminfo_sample() {
        let sample = "\
MemTotal:       32792564 kB
MemFree:         4269948 kB
MemAvailable:   27307104 kB
Buffers:             268 kB
";
        let mem = parse_meminfo(sample);
        let expected_total = 32_792_564.0 / 1024.0;
        let expected_used = (32_792_564.0 - 27_307_104.0) / 1024.0;
        assert!((mem.total_mib.unwrap() - expected_total).abs() < 1e-6);
        assert!((mem.used_mib.unwrap() - expected_used).abs() < 1e-6);
    }

    #[test]
    fn rejects_invalid_meminfo_input() {
        assert_eq!(parse_meminfo(""), MemorySample::default());
        assert_eq!(
            parse_meminfo("MemAvailable: 1000 kB"),
            MemorySample::default()
        );
    }

    #[test]
    fn decodes_bar0_vram_and_hotspot_table() {
        let cases = [
            (0x0000_0580, Some(44.0)), // 0x580 = 1408, 1408/32 = 44
            (0xFFFF_FFFF, None),
            (0xBADF_1234, None),
            (0x0000_0F00, None), // 3840/32 = 120 -> rejected (>= 120)
        ];
        for (raw, expected) in cases {
            assert_eq!(decode_bar0_vram_c(raw), expected);
        }
        assert_eq!(decode_bar0_hotspot_c(0x0000_2D00), Some(45.0));
        assert_eq!(decode_bar0_hotspot_c(0xBADF_0001), None);
    }
}
