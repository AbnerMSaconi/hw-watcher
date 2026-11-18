// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Projects a [`Sample`] + [`PanelConfig`] into Slint view-model rows and theme colors.

use hw_watcher_core::thresholds::{ThresholdMetric, classify};
use hw_watcher_core::{CpuSample, GpuSample, Sample, SourceOrigin, Translator};
use slint::Color;

use crate::config::PanelConfig;
use crate::theme::ThemeConfig;
use crate::view_footer::build_footer;

const MIB_PER_GIB: f64 = 1024.0;

/// One row displayed in the floating panel.
#[derive(Debug, Clone, PartialEq)]
pub struct RowView {
    /// Left label (e.g. `"CPU"`, `"CPU 0"`, `"GPU"`, `"Total"`, `"RAM"`, `"VRAM"`).
    pub label: String,
    /// Customized label color.
    pub label_color: Color,
    /// Primary value text.
    pub col1: String,
    /// Color of `col1`.
    pub col1_color: Color,
    /// Optional secondary value text (empty hides the second column).
    pub col2: String,
    /// Color of `col2`.
    pub col2_color: Color,
}

/// Complete view-model rendered by `ui/panel.slint`.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelView {
    /// Computer name (or `"sim"` for simulated readings) shown in the header, empty if hidden.
    pub source_badge: String,
    /// Visible metric rows (unavailable or disabled sensors are omitted).
    pub rows: Vec<RowView>,
    /// Visible footer lines.
    pub footer_lines: Vec<String>,
    /// Widget surface background color (with alpha).
    pub bg_color: Color,
    /// Widget surface border color (with alpha).
    pub border_color: Color,
    /// Muted header/accent color.
    pub muted_color: Color,
    /// Footer / board text color.
    pub footer_color: Color,
}

/// Builds a [`PanelView`] from `sample` and `cfg` (header shows `computer`), omitting missing sensors.
pub fn build_panel_view(
    sample: &Sample,
    cfg: &PanelConfig,
    computer: &str,
    tr: &dyn Translator,
) -> PanelView {
    let mut rows = Vec::new();
    let theme = &cfg.theme;

    if cfg.widget_items.cpu {
        push_cpu_rows(sample, cfg.separate_cpus, theme, &mut rows);
    }
    if cfg.widget_items.gpu {
        push_gpu_rows(sample, cfg.separate_gpus, theme, &mut rows);
    }
    if cfg.widget_items.total
        && let Some(total_w) = sample.total_power_w()
    {
        let peak_t = sample.peak_temp_c();
        rows.push(RowView {
            label: tr.translate("Total").into_owned(),
            label_color: theme.label_color("total"),
            col1: peak_t.map_or_else(String::new, |t| format!("{t:.0} °C")),
            col1_color: theme.text_slint(),
            col2: format!("{total_w:.0} W"),
            col2_color: theme.text_slint(),
        });
    }
    push_memory_rows(sample, cfg, tr, &mut rows);

    // Simulated readings are labelled as such instead of carrying the real computer name.
    let source_badge = match (cfg.widget_items.show_source_badge, sample.origin) {
        (false, _) => String::new(),
        (true, SourceOrigin::Simulated) => tr.translate("sim").into_owned(),
        (true, _) => computer.to_owned(),
    };

    PanelView {
        source_badge,
        rows,
        footer_lines: build_footer(sample, cfg.widget_items, cfg.separate_gpus, tr),
        bg_color: theme.bg_slint(),
        border_color: theme.border_slint(),
        muted_color: theme.muted_slint(),
        footer_color: theme.label_color("board"),
    }
}

fn push_cpu_rows(sample: &Sample, separate: bool, theme: &ThemeConfig, rows: &mut Vec<RowView>) {
    let cpus = sample.effective_cpus();
    if separate && cpus.len() > 1 {
        for (idx, cpu) in cpus.iter().enumerate() {
            push_one_cpu(
                &format!("CPU {idx}"),
                &format!("cpu:{idx}"),
                *cpu,
                theme,
                rows,
            );
        }
        return;
    }
    let agg = CpuSample {
        package_w: {
            let mut p = cpus.iter().filter_map(|c| c.package_w);
            p.next().map(|f| p.fold(f, |a, b| a + b))
        },
        tctl_c: cpus.iter().filter_map(|c| c.tctl_c).reduce(f64::max),
    };
    push_one_cpu("CPU", "cpu", agg, theme, rows);
}

fn push_one_cpu(
    label: &str,
    color_key: &str,
    cpu: CpuSample,
    theme: &ThemeConfig,
    rows: &mut Vec<RowView>,
) {
    let (t, w) = (cpu.tctl_c, cpu.package_w);
    if t.is_none() && w.is_none() {
        return;
    }
    let (col1, c1) = match (t, w) {
        (Some(temp), _) => (
            format!("{temp:.0} °C"),
            theme.band_slint(classify(ThresholdMetric::CpuTempC, Some(temp))),
        ),
        (None, Some(watts)) => (
            format!("{watts:.0} W"),
            theme.band_slint(classify(ThresholdMetric::CpuPowerW, Some(watts))),
        ),
        (None, None) => return,
    };
    let (col2, c2) = if t.is_some()
        && let Some(watts) = w
    {
        (
            format!("{watts:.0} W"),
            theme.band_slint(classify(ThresholdMetric::CpuPowerW, Some(watts))),
        )
    } else {
        (String::new(), theme.text_slint())
    };
    rows.push(RowView {
        label: label.into(),
        label_color: theme.label_color(color_key),
        col1,
        col1_color: c1,
        col2,
        col2_color: c2,
    });
}

fn push_gpu_rows(sample: &Sample, separate: bool, theme: &ThemeConfig, rows: &mut Vec<RowView>) {
    let gpus = sample.effective_gpus();
    if gpus.is_empty() {
        return;
    }
    if separate && gpus.len() > 1 {
        for (idx, gpu) in gpus.iter().enumerate() {
            push_one_gpu(
                &format!("GPU {idx}"),
                &format!("gpu:{idx}"),
                gpu,
                theme,
                rows,
            );
        }
        return;
    }
    let agg = GpuSample {
        temp_c: gpus.iter().filter_map(|g| g.temp_c).reduce(f64::max),
        power_w: {
            let mut p = gpus.iter().filter_map(|g| g.power_w);
            p.next().map(|f| p.fold(f, |a, b| a + b))
        },
        ..gpus[0]
    };
    push_one_gpu("GPU", "gpu", &agg, theme, rows);
}

fn push_one_gpu(
    label: &str,
    color_key: &str,
    gpu: &GpuSample,
    theme: &ThemeConfig,
    rows: &mut Vec<RowView>,
) {
    let (temp, watts) = (gpu.temp_c, gpu.power_w);
    if temp.is_none() && watts.is_none() {
        return;
    }
    let text = theme.text_slint();
    let (col1, c1, col2, c2) = match (temp, watts) {
        (Some(t), Some(w)) => (
            format!("{t:.0} °C"),
            theme.band_slint(classify(ThresholdMetric::GpuTempC, Some(t))),
            format!("{w:.0} W"),
            theme.band_slint(classify(ThresholdMetric::GpuPowerW, Some(w))),
        ),
        (Some(t), None) => (
            format!("{t:.0} °C"),
            theme.band_slint(classify(ThresholdMetric::GpuTempC, Some(t))),
            String::new(),
            text,
        ),
        (None, Some(w)) => (
            format!("{w:.0} W"),
            theme.band_slint(classify(ThresholdMetric::GpuPowerW, Some(w))),
            String::new(),
            text,
        ),
        (None, None) => return,
    };
    rows.push(RowView {
        label: label.into(),
        label_color: theme.label_color(color_key),
        col1,
        col1_color: c1,
        col2,
        col2_color: c2,
    });
}

fn push_memory_rows(
    sample: &Sample,
    cfg: &PanelConfig,
    tr: &dyn Translator,
    rows: &mut Vec<RowView>,
) {
    let theme = &cfg.theme;
    if cfg.widget_items.ram
        && let (Some(used), Some(total), Some(pct)) = (
            sample.memory.used_mib,
            sample.memory.total_mib,
            sample.memory.used_pct(),
        )
    {
        let color = theme.band_slint(classify(ThresholdMetric::RamPct, Some(pct)));
        rows.push(RowView {
            label: tr.translate("RAM").into_owned(),
            label_color: theme.label_color("ram"),
            col1: format!("{:.1}/{:.1} GiB", used / MIB_PER_GIB, total / MIB_PER_GIB),
            col1_color: color,
            col2: format!("{pct:.0}%"),
            col2_color: color,
        });
    }
    if !cfg.widget_items.vram {
        return;
    }
    let gpus = sample.effective_gpus();
    if cfg.separate_gpus && gpus.len() > 1 {
        for (idx, gpu) in gpus.iter().enumerate() {
            if let (Some(used), Some(total), Some(pct)) =
                (gpu.vram_used_mib, gpu.vram_total_mib, gpu.vram_pct())
            {
                let color = theme.band_slint(classify(ThresholdMetric::VramPct, Some(pct)));
                rows.push(RowView {
                    label: format!("{} {idx}", tr.translate("VRAM")),
                    label_color: theme.label_color(&format!("vram:{idx}")),
                    col1: format!("{:.1}/{:.1} GiB", used / MIB_PER_GIB, total / MIB_PER_GIB),
                    col1_color: color,
                    col2: format!("{pct:.0}%"),
                    col2_color: color,
                });
            }
        }
    } else if let Some(gpu) = gpus.first()
        && let (Some(used), Some(total), Some(pct)) =
            (gpu.vram_used_mib, gpu.vram_total_mib, gpu.vram_pct())
    {
        let color = theme.band_slint(classify(ThresholdMetric::VramPct, Some(pct)));
        rows.push(RowView {
            label: tr.translate("VRAM").into_owned(),
            label_color: theme.label_color("vram"),
            col1: format!("{:.1}/{:.1} GiB", used / MIB_PER_GIB, total / MIB_PER_GIB),
            col1_color: color,
            col2: format!("{pct:.0}%"),
            col2_color: color,
        });
    }
}

#[cfg(test)]
#[path = "view_model_tests.rs"]
mod tests;
