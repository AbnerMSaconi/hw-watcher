// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

#[cfg(test)]
use super::*;
#[cfg(test)]
use hw_watcher_core::{BoardSample, IdentityTranslator, MemorySample};

#[cfg(test)]
#[test]
fn unprivileged_view_hides_total_and_elevated_footer() {
    let sample = Sample {
        origin: SourceOrigin::Local,
        cpu: CpuSample {
            tctl_c: Some(55.0),
            package_w: None,
        },
        gpu: Some(GpuSample {
            temp_c: Some(36.0),
            power_w: Some(33.0),
            clock_mhz: Some(210.0),
            usage_pct: Some(4.0),
            power_limit_w: Some(420.0),
            vram_used_mib: Some(2048.0),
            vram_total_mib: Some(24576.0),
            ..Default::default()
        }),
        memory: MemorySample {
            used_mib: Some(8192.0),
            total_mib: Some(32768.0),
        },
        board: BoardSample {
            v12_v: Some(12.05),
            ..Default::default()
        },
        ..Default::default()
    };

    let view = build_panel_view(&sample, &PanelConfig::default(), &IdentityTranslator);
    assert_eq!(view.source_badge, "local");
    let labels: Vec<&str> = view.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["CPU", "GPU", "RAM", "VRAM"]);
    assert_eq!(view.rows[0].col1, "55 °C");
    assert_eq!(view.rows[0].col2, "");
    assert_eq!(view.footer_lines.len(), 2);
}

#[cfg(test)]
#[test]
fn multi_device_separation_and_custom_item_colors_apply() {
    let sample = Sample {
        origin: SourceOrigin::Daemon,
        cpus: vec![
            CpuSample {
                tctl_c: Some(50.0),
                package_w: Some(40.0),
            },
            CpuSample {
                tctl_c: Some(62.0),
                package_w: Some(55.0),
            },
        ],
        gpus: vec![
            GpuSample {
                temp_c: Some(44.0),
                power_w: Some(200.0),
                ..Default::default()
            },
            GpuSample {
                temp_c: Some(49.0),
                power_w: Some(150.0),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let mut cfg = PanelConfig {
        separate_cpus: true,
        separate_gpus: true,
        ..Default::default()
    };
    cfg.theme
        .item_label_colors
        .insert("cpu:0".into(), "#56d4dd".into());
    let view = build_panel_view(&sample, &cfg, &IdentityTranslator);
    let labels: Vec<&str> = view.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["CPU 0", "CPU 1", "GPU 0", "GPU 1", "Total"]);
    assert_eq!(
        view.rows[0].label_color,
        Color::from_rgb_u8(0x56, 0xd4, 0xdd)
    );
    assert_eq!(view.rows[4].col2, "445 W");
}
