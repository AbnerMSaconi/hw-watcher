// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Tooltip formatting for single-device and multi-CPU/multi-GPU snapshots.

use crate::model::{GpuSample, Sample};
use crate::traits::Translator;

use super::TooltipSections;

const MIB_PER_GIB: f64 = 1024.0;

/// Formats only enabled tooltip sections whose underlying sensors are present in `sample`.
pub fn format_tooltip_lines(
    sample: &Sample,
    sections: TooltipSections,
    tr: &dyn Translator,
) -> Vec<String> {
    format_tooltip_lines_for(sample, sections, false, false, tr)
}

/// Formats tooltip lines with optional per-CPU and per-GPU separation.
pub fn format_tooltip_lines_for(
    sample: &Sample,
    sections: TooltipSections,
    split_cpus: bool,
    separate_gpus: bool,
    tr: &dyn Translator,
) -> Vec<String> {
    let mut lines = Vec::new();
    if sections.power || sections.temps {
        push_cpu_lines(sample, sections, split_cpus, &mut lines);
        push_gpu_lines(sample, sections, separate_gpus, &mut lines);
    }
    if sections.power
        && let Some(total_w) = sample.total_power_w()
    {
        lines.push(tr.format("Total {w} W", &[("w", &format!("{total_w:.0}"))]));
    }
    if sections.memory {
        push_memory_lines(sample, separate_gpus, &mut lines);
    }
    if sections.clocks {
        push_gpu_clock_lines(sample, separate_gpus, tr, &mut lines);
    }
    if sections.board
        && let Some(v12) = sample.board.v12_v
    {
        lines.push(format!("+12V {v12:.2} V"));
    }
    if lines.is_empty() {
        lines.push(tr.translate("Waiting for telemetry…").into_owned());
    }
    lines
}

fn push_cpu_lines(
    sample: &Sample,
    sections: TooltipSections,
    separate_cpus: bool,
    lines: &mut Vec<String>,
) {
    let cpus = sample.effective_cpus();
    if separate_cpus && cpus.len() > 1 {
        for (idx, cpu) in cpus.iter().enumerate() {
            let t = sections
                .temps
                .then_some(cpu.tctl_c)
                .flatten()
                .map(|v| format!("{v:.0} °C"));
            let w = sections
                .power
                .then_some(cpu.package_w)
                .flatten()
                .map(|v| format!("{v:.0} W"));
            if let Some(line) = join_pair(&format!("CPU {idx}"), t, w) {
                lines.push(line);
            }
        }
        return;
    }

    let cpu_t = sections
        .temps
        .then(|| cpus.iter().filter_map(|c| c.tctl_c).reduce(f64::max))
        .flatten()
        .map(|v| format!("{v:.0} °C"));
    let cpu_w = sections
        .power
        .then(|| {
            let mut vals = cpus.iter().filter_map(|c| c.package_w);
            let first = vals.next()?;
            Some(vals.fold(first, |a, b| a + b))
        })
        .flatten()
        .map(|v| format!("{v:.0} W"));
    if let Some(line) = join_pair("CPU", cpu_t, cpu_w) {
        lines.push(line);
    }
}

fn push_gpu_lines(
    sample: &Sample,
    sections: TooltipSections,
    separate_gpus: bool,
    lines: &mut Vec<String>,
) {
    let gpus = sample.effective_gpus();
    if gpus.is_empty() {
        return;
    }
    if separate_gpus && gpus.len() > 1 {
        for (idx, gpu) in gpus.iter().enumerate() {
            push_single_gpu_lines(&format!("GPU {idx}"), gpu, sections, lines);
        }
        return;
    }
    let gpu = &gpus[0];
    let agg_t = gpus.iter().filter_map(|g| g.temp_c).reduce(f64::max);
    let agg_w = {
        let mut vals = gpus.iter().filter_map(|g| g.power_w);
        vals.next().map(|first| vals.fold(first, |a, b| a + b))
    };
    let merged = GpuSample {
        temp_c: agg_t,
        power_w: agg_w,
        ..*gpu
    };
    push_single_gpu_lines("GPU", &merged, sections, lines);
}

fn push_single_gpu_lines(
    prefix: &str,
    gpu: &GpuSample,
    sections: TooltipSections,
    lines: &mut Vec<String>,
) {
    let gpu_t = sections
        .temps
        .then_some(gpu.temp_c)
        .flatten()
        .map(|v| format!("{v:.0} °C"));
    let gpu_w = sections
        .power
        .then_some(gpu.power_w)
        .flatten()
        .map(|v| format!("{v:.0} W"));
    if let Some(line) = join_pair(prefix, gpu_t, gpu_w) {
        lines.push(line);
    }
    if sections.temps
        && let (Some(v), Some(h)) = (gpu.vram_temp_c, gpu.hotspot_temp_c)
    {
        lines.push(format!("VRAM {v:.0} °C  Hotspot {h:.0} °C"));
    }
}

fn push_memory_lines(sample: &Sample, separate_gpus: bool, lines: &mut Vec<String>) {
    if let (Some(used), Some(total), Some(pct)) = (
        sample.memory.used_mib,
        sample.memory.total_mib,
        sample.memory.used_pct(),
    ) {
        lines.push(format!(
            "RAM  {:.1}/{:.1} GiB ({pct:.0}%)",
            used / MIB_PER_GIB,
            total / MIB_PER_GIB
        ));
    }
    let gpus = sample.effective_gpus();
    if separate_gpus && gpus.len() > 1 {
        for (idx, gpu) in gpus.iter().enumerate() {
            if let (Some(used), Some(total), Some(pct)) =
                (gpu.vram_used_mib, gpu.vram_total_mib, gpu.vram_pct())
            {
                lines.push(format!(
                    "VRAM {idx} {:.1}/{:.1} GiB ({pct:.0}%)",
                    used / MIB_PER_GIB,
                    total / MIB_PER_GIB
                ));
            }
        }
    } else if let Some(gpu) = gpus.first()
        && let (Some(used), Some(total), Some(pct)) =
            (gpu.vram_used_mib, gpu.vram_total_mib, gpu.vram_pct())
    {
        lines.push(format!(
            "VRAM {:.1}/{:.1} GiB ({pct:.0}%)",
            used / MIB_PER_GIB,
            total / MIB_PER_GIB
        ));
    }
}

fn push_gpu_clock_lines(
    sample: &Sample,
    separate_gpus: bool,
    tr: &dyn Translator,
    lines: &mut Vec<String>,
) {
    let gpus = sample.effective_gpus();
    if separate_gpus && gpus.len() > 1 {
        for (idx, gpu) in gpus.iter().enumerate() {
            push_one_clock_line(&format!("GPU {idx}"), gpu, tr, lines);
        }
    } else if let Some(gpu) = gpus.first() {
        push_one_clock_line("GPU", gpu, tr, lines);
    }
}

fn push_one_clock_line(
    prefix: &str,
    gpu: &GpuSample,
    tr: &dyn Translator,
    lines: &mut Vec<String>,
) {
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

fn join_pair(prefix: &str, a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(format!("{prefix}  {x}  {y}")),
        (Some(x), None) | (None, Some(x)) => Some(format!("{prefix}  {x}")),
        (None, None) => None,
    }
}
