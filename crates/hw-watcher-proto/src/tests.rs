// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

use super::*;

#[test]
fn wire_roundtrip_preserves_present_and_missing_fields() {
    let original = Sample {
        timestamp_ms: 1_700_000_000_000,
        origin: SourceOrigin::Daemon,
        caps: Capabilities {
            nvml: true,
            meminfo: true,
            ..Default::default()
        },
        gpu: Some(GpuSample {
            temp_c: Some(42.0),
            power_w: Some(150.0),
            vram_used_mib: Some(2048.0),
            vram_total_mib: Some(24576.0),
            vram_temp_c: None,
            ..Default::default()
        }),
        cpu: CpuSample {
            package_w: None,
            tctl_c: Some(55.0),
        },
        cpus: vec![CpuSample {
            package_w: None,
            tctl_c: Some(55.0),
        }],
        gpus: vec![],
        memory: MemorySample {
            used_mib: Some(10_240.0),
            total_mib: Some(32_768.0),
        },
        board: BoardSample {
            v12_v: Some(12.024),
            ..Default::default()
        },
    };
    let wire = WireSample::from(&original);
    assert!(wire.vram_temp_c.is_nan());
    assert!(wire.cpu_package_w.is_nan());
    let decoded = Sample::from(wire);
    assert_eq!(decoded, original);
}
