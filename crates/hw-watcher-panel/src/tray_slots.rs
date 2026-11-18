// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Coordinates 1 primary SNI tray icon + 0..7 extra taskbar metric badges.

use hw_watcher_core::{
    Band, Sample, Translator, TrayMetric, TrayRendering, build_tray_slots, format_tooltip_lines_for,
};
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, ToolTip, TrayMethods};
use tokio::sync::mpsc::UnboundedSender;

use crate::config::PanelConfig;
use crate::i18n::PoTranslator;
use crate::theme::ThemeConfig;
use crate::tray::{TrayCommand, WatcherTray};
use crate::tray_icon::render_themed_badge;

struct MetricSlotTray {
    slot_index: usize,
    tx: UnboundedSender<TrayCommand>,
    rendering: TrayRendering,
    tooltip_lines: Vec<String>,
    theme: ThemeConfig,
}

impl ksni::Tray for MetricSlotTray {
    fn id(&self) -> String {
        format!("hw-watcher-slot-{}", self.slot_index)
    }

    fn title(&self) -> String {
        PoTranslator::active().translate("HW Watcher").into_owned()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![render_themed_badge(&self.rendering, &self.theme)]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: PoTranslator::active().translate("HW Watcher").into_owned(),
            description: self.tooltip_lines.join("\n"),
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCommand::ToggleVisible);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let tr = PoTranslator::active();
        vec![
            StandardItem {
                label: tr.translate("Show / Hide").into_owned(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCommand::ToggleVisible);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: tr.translate("Control Panel...").into_owned(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCommand::OpenSettings);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: tr.translate("Quit").into_owned(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Manages the primary SNI tray icon and any extra taskbar metric slots.
pub struct TrayCoordinator {
    tx: UnboundedSender<TrayCommand>,
    primary: Option<ksni::Handle<WatcherTray>>,
    extras: Vec<ksni::Handle<MetricSlotTray>>,
    /// Fingerprint of everything last pushed to the tray, to skip redundant D-Bus updates.
    last_key: Option<String>,
}

impl TrayCoordinator {
    /// Spawns the primary tray icon and initial extra taskbar slots from `cfg`.
    pub async fn spawn(tx: UnboundedSender<TrayCommand>, cfg: &PanelConfig) -> Self {
        let empty = Sample::default();
        let tr = PoTranslator::active();
        let slots = build_tray_slots(
            &cfg.ordered_tray_metrics(),
            usize::from(cfg.tray_count),
            cfg.separate_cpus,
            cfg.separate_gpus,
            &empty,
        );
        let primary_rend = slots
            .first()
            .map_or_else(static_badge, |s| s.rendering.clone());
        let tray = WatcherTray {
            tx: tx.clone(),
            sample: empty.clone(),
            rendering: primary_rend,
            tooltip_lines: Vec::new(),
            always_on_top: cfg.always_on_top,
            skip_taskbar: cfg.skip_taskbar,
            tray_metric: cfg.tray_metric,
            tooltip_sections: cfg.tooltip,
            theme: cfg.theme.clone(),
        };
        let primary = tray.spawn().await.ok();
        let mut coord = Self {
            tx,
            primary,
            extras: Vec::new(),
            last_key: None,
        };
        coord.sync_and_refresh(&empty, cfg, &tr).await;
        coord
    }

    /// Updates all active tray icons and dynamically spawns/despawns extra slots.
    pub async fn sync_and_refresh(
        &mut self,
        sample: &Sample,
        cfg: &PanelConfig,
        tr: &dyn Translator,
    ) {
        let slots = build_tray_slots(
            &cfg.ordered_tray_metrics(),
            usize::from(cfg.tray_count),
            cfg.separate_cpus,
            cfg.separate_gpus,
            sample,
        );
        let primary_rend = slots
            .first()
            .map_or_else(static_badge, |s| s.rendering.clone());
        let lines = format_tooltip_lines_for(
            sample,
            cfg.tooltip,
            cfg.separate_cpus,
            cfg.separate_gpus,
            tr,
        );
        let available: Vec<bool> = TrayMetric::ALL
            .iter()
            .map(|m| m.is_available(sample))
            .collect();
        let key = format!(
            "{slots:?}|{lines:?}|{available:?}|{}|{}|{:?}|{:?}|{:?}",
            cfg.always_on_top, cfg.skip_taskbar, cfg.tray_metric, cfg.tooltip, cfg.theme
        );
        if self.last_key.as_deref() == Some(key.as_str()) {
            return;
        }
        self.last_key = Some(key);
        if let Some(ref h) = self.primary {
            let s = sample.clone();
            let cfg_c = cfg.clone();
            let lines = lines.clone();
            h.update(move |t| {
                t.always_on_top = cfg_c.always_on_top;
                t.skip_taskbar = cfg_c.skip_taskbar;
                t.tray_metric = cfg_c.tray_metric;
                t.tooltip_sections = cfg_c.tooltip;
                t.theme = cfg_c.theme;
                t.tooltip_lines = lines;
                t.rendering = primary_rend;
                t.sample = s;
            })
            .await;
        }

        let want_extras = slots.len().saturating_sub(1);
        while self.extras.len() > want_extras {
            if let Some(handle) = self.extras.pop() {
                handle.shutdown().await;
            }
        }
        for (i, slot) in slots.iter().skip(1).enumerate() {
            if let Some(existing) = self.extras.get(i) {
                let rend = slot.rendering.clone();
                let l = lines.clone();
                let theme = cfg.theme.clone();
                existing
                    .update(move |t| {
                        t.rendering = rend;
                        t.tooltip_lines = l;
                        t.theme = theme;
                    })
                    .await;
            } else {
                let extra = MetricSlotTray {
                    slot_index: i + 1,
                    tx: self.tx.clone(),
                    rendering: slot.rendering.clone(),
                    tooltip_lines: lines.clone(),
                    theme: cfg.theme.clone(),
                };
                if let Ok(handle) = extra.spawn().await {
                    self.extras.push(handle);
                }
            }
        }
    }
}

fn static_badge() -> TrayRendering {
    TrayRendering {
        text: "E".into(),
        band: Band::Normal,
    }
}
