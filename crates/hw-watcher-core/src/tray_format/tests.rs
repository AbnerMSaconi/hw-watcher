// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

use super::*;
use crate::model::{BoardSample, CpuSample, GpuSample, MemorySample};
use crate::traits::IdentityTranslator;

fn sample_unprivileged() -> Sample {
    Sample {
        cpu: CpuSample {
            tctl_c: Some(54.0),
            package_w: None,
        },
        gpu: Some(GpuSample {
            temp_c: Some(36.0),
            power_w: Some(33.0),
            clock_mhz: Some(1860.0),
            usage_pct: Some(12.0),
            power_limit_w: Some(420.0),
            vram_used_mib: Some(4096.0),
            vram_total_mib: Some(24576.0),
            ..Default::default()
        }),
        memory: MemorySample {
            used_mib: Some(8192.0),
            total_mib: Some(32768.0),
        },
        board: BoardSample {
            v12_v: Some(12.048),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn tray_metric_falls_back_when_total_power_unavailable() {
    let s = sample_unprivileged();
    assert!(!TrayMetric::TotalPowerW.is_available(&s));
    let rendered = format_tray_rendering(TrayMetric::TotalPowerW, &s);
    assert_eq!(rendered.text, "33");
}

#[test]
fn tray_renders_clock_ram_and_vram_compactly() {
    let s = sample_unprivileged();
    assert_eq!(
        format_tray_rendering(TrayMetric::GpuClockMhz, &s).text,
        "1.9G"
    );
    assert_eq!(format_tray_rendering(TrayMetric::RamPct, &s).text, "25%");
    assert_eq!(format_tray_rendering(TrayMetric::VramPct, &s).text, "17%");
}

#[test]
fn tooltip_hides_missing_cpu_power_and_total_when_unprivileged() {
    let s = sample_unprivileged();
    let lines = format_tooltip_lines(&s, TooltipSections::default(), &IdentityTranslator);
    assert_eq!(lines[0], "CPU  54 °C");
    assert_eq!(lines[1], "GPU  36 °C  33 W");
    assert!(lines.iter().all(|l| !l.starts_with("Total")));
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("RAM  8.0/32.0 GiB (25%)"))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("VRAM 4.0/24.0 GiB (17%)"))
    );
}

#[test]
fn tooltip_respects_section_toggles_and_shows_elevated_fields() {
    let mut s = sample_unprivileged();
    s.cpu.package_w = Some(65.0);
    if let Some(ref mut g) = s.gpu {
        g.vram_temp_c = Some(62.0);
        g.hotspot_temp_c = Some(58.0);
    }
    let sec = TooltipSections {
        power: true,
        temps: true,
        memory: false,
        clocks: false,
        board: false,
    };
    let lines = format_tooltip_lines(&s, sec, &IdentityTranslator);
    assert_eq!(
        lines,
        vec![
            "CPU  54 °C  65 W",
            "GPU  36 °C  33 W",
            "VRAM 62 °C  Hotspot 58 °C",
            "Total 98 W",
        ]
    );
}

#[test]
fn build_tray_slots_respects_count_and_multi_gpu_separation() {
    let s = Sample {
        cpus: vec![
            CpuSample {
                tctl_c: Some(50.0),
                package_w: Some(45.0),
            },
            CpuSample {
                tctl_c: Some(61.0),
                package_w: Some(55.0),
            },
        ],
        gpus: vec![
            GpuSample {
                temp_c: Some(41.0),
                power_w: Some(150.0),
                ..Default::default()
            },
            GpuSample {
                temp_c: Some(68.0),
                power_w: Some(220.0),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert_eq!(
        build_tray_slots(&[TrayMetric::GpuTempC], 0, true, true, &s),
        []
    );

    let slots = build_tray_slots(
        &[TrayMetric::GpuTempC, TrayMetric::CpuTempC],
        3,
        true,
        true,
        &s,
    );
    assert_eq!(slots.len(), 3);
    assert_eq!(slots[0].rendering.text, "41");
    assert_eq!(slots[0].device_index, Some(0));
    assert_eq!(slots[1].rendering.text, "68");
    assert_eq!(slots[1].device_index, Some(1));
    assert_eq!(slots[2].rendering.text, "50");
    assert_eq!(slots[2].device_index, Some(0));
}
