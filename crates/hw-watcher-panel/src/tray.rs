// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Modular SNI system tray icon (`ksni`) menu and primary icon state.

use hw_watcher_core::{Sample, TooltipSections, Translator, TrayMetric, TrayRendering};
use ksni::menu::{CheckmarkItem, StandardItem, SubMenu};
use ksni::{Icon, MenuItem, ToolTip};
use tokio::sync::mpsc::UnboundedSender;

use crate::i18n::PoTranslator;
use crate::theme::ThemeConfig;
use crate::tray_icon::render_themed_badge;

/// Commands emitted from the tray icon, panel menu, or Control Panel to the app loop.
#[derive(Debug, Clone)]
pub enum TrayCommand {
    /// Show or hide the floating window.
    ToggleVisible,
    /// Open or focus the Control Panel / Settings window.
    OpenSettings,
    /// Close and drop the Control Panel / Settings window.
    CloseSettings,
    /// Toggle always-on-top window flag.
    AlwaysOnTop(bool),
    /// Toggle hiding the floating window from the taskbar and pager.
    SkipTaskbar(bool),
    /// Select which metric the primary tray icon renders.
    SetTrayMetric(TrayMetric),
    /// Update modular tooltip visibility sections.
    SetTooltipSections(TooltipSections),
    /// Apply an updated configuration from the Control Panel.
    ApplyConfig(Box<crate::config::PanelConfig>),
    /// Exit the application.
    Quit,
}

/// Primary SNI tray state owned by `ksni`.
pub struct WatcherTray {
    /// Command sender back to the app event loop.
    pub tx: UnboundedSender<TrayCommand>,
    /// Latest telemetry snapshot.
    pub sample: Sample,
    /// Current rendered icon text + band.
    pub rendering: TrayRendering,
    /// Current tooltip lines.
    pub tooltip_lines: Vec<String>,
    /// Current always-on-top state.
    pub always_on_top: bool,
    /// Current taskbar hiding state.
    pub skip_taskbar: bool,
    /// Selected primary tray icon metric.
    pub tray_metric: TrayMetric,
    /// Enabled tooltip sections.
    pub tooltip_sections: TooltipSections,
    /// Active theme colors for the badge icon.
    pub theme: ThemeConfig,
}

impl ksni::Tray for WatcherTray {
    fn id(&self) -> String {
        "hw-watcher".into()
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
        let metric_items = build_metric_menu(self, &tr);
        let tooltip_items = build_tooltip_menu(self.tooltip_sections, &tr);

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
            CheckmarkItem {
                label: tr.translate("Always on top").into_owned(),
                checked: self.always_on_top,
                activate: Box::new(|t: &mut Self| {
                    t.always_on_top = !t.always_on_top;
                    let _ = t.tx.send(TrayCommand::AlwaysOnTop(t.always_on_top));
                }),
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: tr.translate("Hide from taskbar").into_owned(),
                checked: self.skip_taskbar,
                activate: Box::new(|t: &mut Self| {
                    t.skip_taskbar = !t.skip_taskbar;
                    let _ = t.tx.send(TrayCommand::SkipTaskbar(t.skip_taskbar));
                }),
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: tr.translate("Tray icon metric").into_owned(),
                submenu: metric_items,
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: tr.translate("Tooltip items").into_owned(),
                submenu: tooltip_items,
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

fn build_metric_menu(tray: &WatcherTray, tr: &dyn Translator) -> Vec<MenuItem<WatcherTray>> {
    let resolved = tray.tray_metric.resolve_for(&tray.sample);
    TrayMetric::ALL
        .into_iter()
        .filter(|m| m.is_available(&tray.sample) || *m == resolved)
        .map(|metric| {
            CheckmarkItem {
                label: tr.translate(metric.label_key()).into_owned(),
                checked: metric == resolved,
                activate: Box::new(move |t: &mut WatcherTray| {
                    t.tray_metric = metric;
                    let _ = t.tx.send(TrayCommand::SetTrayMetric(metric));
                }),
                ..Default::default()
            }
            .into()
        })
        .collect()
}

type TooltipToggle = (&'static str, bool, fn(&mut TooltipSections));

fn build_tooltip_menu(sec: TooltipSections, tr: &dyn Translator) -> Vec<MenuItem<WatcherTray>> {
    let entries: [TooltipToggle; 5] = [
        ("Power", sec.power, |s| s.power = !s.power),
        ("Temperatures", sec.temps, |s| s.temps = !s.temps),
        ("Memory (RAM / VRAM)", sec.memory, |s| s.memory = !s.memory),
        ("Clocks & load", sec.clocks, |s| s.clocks = !s.clocks),
        ("Board (+12V)", sec.board, |s| s.board = !s.board),
    ];
    entries
        .into_iter()
        .map(|(key, checked, toggle)| {
            CheckmarkItem {
                label: tr.translate(key).into_owned(),
                checked,
                activate: Box::new(move |t: &mut WatcherTray| {
                    toggle(&mut t.tooltip_sections);
                    let _ =
                        t.tx.send(TrayCommand::SetTooltipSections(t.tooltip_sections));
                }),
                ..Default::default()
            }
            .into()
        })
        .collect()
}
