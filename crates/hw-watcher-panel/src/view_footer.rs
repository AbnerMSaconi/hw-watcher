// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Footer line builder for GPU clocks, elevated BAR0/NVAPI, and motherboard sensors.

use hw_watcher_core::{BoardSample, GpuSample, Sample, Translator};

use crate::config::WidgetItemsConfig;

/// Builds visible footer lines from `sample` according to `items` and `separate_gpus`.
pub fn build_footer(
    sample: &Sample,
    items: WidgetItemsConfig,
    separate_gpus: bool,
    tr: &dyn Translator,
) -> Vec<String> {
    let mut lines = Vec::new();
    let gpus = sample.effective_gpus();
    if separate_gpus && gpus.len() > 1 {
        for (idx, gpu) in gpus.iter().enumerate() {
            push_gpu_footer(&format!("GPU {idx}"), gpu, items, tr, &mut lines);
        }
    } else if let Some(gpu) = gpus.first() {
        push_gpu_footer("GPU", gpu, items, tr, &mut lines);
    }

    if items.board_v12
        && let Some(v12) = sample.board.v12_v
    {
        lines.push(format!("+12V {v12:.2} V"));
    }
    if items.board_rails {
        push_board_rails(&sample.board, &mut lines);
    }
    if items.board_temps {
        push_board_temps(&sample.board, &mut lines);
    }
    if items.board_fans {
        push_board_fans(&sample.board, tr, &mut lines);
    }
    lines
}

fn push_gpu_footer(
    prefix: &str,
    gpu: &GpuSample,
    items: WidgetItemsConfig,
    tr: &dyn Translator,
    lines: &mut Vec<String>,
) {
    if items.gpu_clocks {
        let mut parts = Vec::new();
        if let Some(mhz) = gpu.clock_mhz {
            parts.push(format!("{mhz:.0} MHz"));
        }
        if let Some(pct) = gpu.usage_pct {
            parts.push(format!("{pct:.0}%"));
        }
        if let Some(pl) = gpu.power_limit_w {
            parts.push(tr.format("limit {w} W", &[("w", &format!("{pl:.0}"))]));
        }
        if !parts.is_empty() {
            lines.push(format!("{prefix}  {}", parts.join(" · ")));
        }
    }
    if items.gpu_elevated {
        let mut elev = Vec::new();
        if let Some(v) = gpu.vram_temp_c {
            elev.push(format!("VRAM {v:.0} °C"));
        }
        if let Some(h) = gpu.hotspot_temp_c {
            elev.push(format!("Hotspot {h:.0} °C"));
        }
        if let Some(v) = gpu.voltage_v {
            elev.push(format!("{v:.3} V"));
        }
        if !elev.is_empty() {
            lines.push(elev.join(" · "));
        }
    }
}

fn push_board_rails(b: &BoardSample, lines: &mut Vec<String>) {
    let mut parts = Vec::new();
    if let Some(v) = b.v5_v {
        parts.push(format!("+5V {v:.2} V"));
    }
    if let Some(v) = b.v3_3_v {
        parts.push(format!("+3.3V {v:.2} V"));
    }
    if let Some(v) = b.vcore_v {
        parts.push(format!("Vcore {v:.3} V"));
    }
    if let Some(v) = b.soc_v {
        parts.push(format!("SoC {v:.3} V"));
    }
    if let Some(v) = b.dram_v {
        parts.push(format!("DRAM {v:.3} V"));
    }
    if !parts.is_empty() {
        lines.push(parts.join(" · "));
    }
}

fn push_board_temps(b: &BoardSample, lines: &mut Vec<String>) {
    let mut parts = Vec::new();
    if let Some(t) = b.vrm_mos_c {
        parts.push(format!("VRM {t:.0} °C"));
    }
    if let Some(t) = b.cpu_socket_c {
        parts.push(format!("Socket {t:.0} °C"));
    }
    if let Some(t) = b.system_c {
        parts.push(format!("Sys {t:.0} °C"));
    }
    if let Some(t) = b.pch_c {
        parts.push(format!("PCH {t:.0} °C"));
    }
    if let Some(t) = b.pcie_x1_c {
        parts.push(format!("PCIe {t:.0} °C"));
    }
    if !parts.is_empty() {
        lines.push(parts.join(" · "));
    }
}

fn push_board_fans(b: &BoardSample, tr: &dyn Translator, lines: &mut Vec<String>) {
    let mut parts = Vec::new();
    if let Some(rpm) = b.cpu_fan_rpm {
        parts.push(format!("CPU {rpm:.0}"));
    }
    if let Some(rpm) = b.sys_fan1_rpm {
        parts.push(format!("Sys1 {rpm:.0}"));
    }
    if let Some(rpm) = b.sys_fan3_rpm {
        parts.push(format!("Sys3 {rpm:.0}"));
    }
    if !parts.is_empty() {
        lines.push(format!(
            "{} {} RPM",
            tr.translate("Fans:"),
            parts.join(" · ")
        ));
    }
}
