use std::sync::atomic::Ordering;

use crate::daemon;
use crate::monitor::notify;
use crate::settings::{MAX_REFRESH_MS, MIN_REFRESH_MS, REFRESH_PRESETS, Settings};
use crate::theme::{self, Theme};
use crate::ui::widgets;

#[derive(Default)]
pub struct State {}

pub fn show(ui: &mut egui::Ui, _state: &mut State, settings: &Settings) {
    ui.heading("Settings");
    ui.label(
        egui::RichText::new("Tweak how rproc samples and displays system data.")
            .color(theme::text_dim()),
    );
    ui.add_space(16.0);

    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Refresh rate").strong().size(15.0));
                ui.label(
                    egui::RichText::new(
                        "How often the sampler thread polls the system. \
                         Lower intervals feel snappier but use more CPU.",
                    )
                    .color(theme::text_dim())
                    .small(),
                );
            });
        });
        ui.add_space(10.0);

        let mut current = settings.refresh_ms();

        // Preset chips
        ui.horizontal_wrapped(|ui| {
            for (ms, label) in REFRESH_PRESETS {
                let selected = current == *ms;
                if preset_chip(ui, label, selected).clicked() {
                    settings.set_refresh_ms(*ms);
                    current = *ms;
                }
            }
        });

        ui.add_space(12.0);

        // Fine slider for arbitrary values.
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Custom")
                    .color(theme::text_dim())
                    .small(),
            );
            let mut value = current;
            let resp = ui.add(
                egui::Slider::new(&mut value, MIN_REFRESH_MS..=MAX_REFRESH_MS)
                    .logarithmic(true)
                    .suffix(" ms"),
            );
            if resp.changed() {
                settings.set_refresh_ms(value);
                current = value;
            }
        });

        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!("Currently sampling every {}", format_ms(current)))
                .color(theme::accent())
                .strong(),
        );
    });

    ui.add_space(12.0);

    widgets::card(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Background history")
                    .strong()
                    .size(15.0),
            );
            ui.label(
                egui::RichText::new(
                    "Run a tiny background process that records the last 60 s of \
                     CPU, memory, disk, network and GPU activity. When on, rproc \
                     shows that recent history the moment you reopen it, even after \
                     a restart. When off, no background process runs, but history \
                     starts empty each time you open the window.",
                )
                .color(theme::text_dim())
                .small(),
            );
        });
        ui.add_space(10.0);

        let mut enabled = settings.daemon_enabled();
        if ui
            .checkbox(
                &mut enabled,
                egui::RichText::new("Keep the last 60 seconds in the background").strong(),
            )
            .changed()
        {
            settings.set_daemon_enabled(enabled);
            // Apply the change immediately: start the daemon now, or stop the
            // one that's currently running.
            if enabled {
                daemon::spawn_if_absent();
            } else {
                daemon::stop();
            }
        }

        ui.add_space(6.0);
        let (status, color) = if enabled {
            ("Background sampler running", theme::accent())
        } else {
            ("Background sampler off", theme::text_dim())
        };
        ui.label(egui::RichText::new(status).color(color).strong());
    });

    ui.add_space(12.0);

    // --- Notification thresholds --------------------------------------------
    widgets::card(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Notifications")
                    .strong()
                    .size(15.0),
            );
            ui.label(
                egui::RichText::new(
                    "Get a desktop notification when CPU or RAM usage exceeds a \
                     threshold. Set a value to 0% to disable alerts for that metric. \
                     Notifications respect a cooldown to avoid spam.",
                )
                .color(theme::text_dim())
                .small(),
            );
        });
        ui.add_space(10.0);

        let thresh = settings.thresholds();

        // CPU threshold
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("CPU threshold").strong());
            ui.add_space(20.0);
            let mut cpu = thresh.cpu_pct.load(Ordering::Relaxed);
            if ui
                .add(
                    egui::Slider::new(&mut cpu, 0..=100)
                        .suffix("%")
                        .text_color(theme::graph_cpu()),
                )
                .changed()
            {
                thresh.cpu_pct.store(cpu, Ordering::Relaxed);
                settings.save_external();
            }
        });

        // RAM threshold
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("RAM threshold").strong());
            ui.add_space(18.0);
            let mut ram = thresh.ram_pct.load(Ordering::Relaxed);
            if ui
                .add(
                    egui::Slider::new(&mut ram, 0..=100)
                        .suffix("%")
                        .text_color(theme::graph_ram()),
                )
                .changed()
            {
                thresh.ram_pct.store(ram, Ordering::Relaxed);
                settings.save_external();
            }
        });

        // Cooldown
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Cooldown").strong());
            ui.add_space(44.0);
            let mut cooldown = thresh.cooldown_secs.load(Ordering::Relaxed);
            if ui
                .add(
                    egui::Slider::new(&mut cooldown, notify::MIN_COOLDOWN_SECS..=3600)
                        .suffix(" s")
                        .logarithmic(true),
                )
                .changed()
            {
                thresh.cooldown_secs.store(cooldown, Ordering::Relaxed);
                settings.save_external();
            }
        });
    });

    ui.add_space(12.0);

    // --- Theme toggle -------------------------------------------------------
    widgets::card(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Appearance").strong().size(15.0));
            ui.label(
                egui::RichText::new(
                    "Switch between dark, light, or follow your system preference. \
                     Changes take effect immediately.",
                )
                .color(theme::text_dim())
                .small(),
            );
        });
        ui.add_space(10.0);

        let current = settings.theme();
        ui.horizontal(|ui| {
            for (choice, label) in [
                (Theme::System, "System"),
                (Theme::Dark, "Dark"),
                (Theme::Light, "Light"),
            ] {
                let selected = current == choice;
                if ui
                    .selectable_label(selected, egui::RichText::new(label).strong())
                    .clicked()
                    && !selected
                {
                    settings.set_theme(choice);
                }
            }
        });
    });

    ui.add_space(12.0);

    widgets::card(ui, |ui| {
        ui.label(egui::RichText::new("About").strong().size(15.0));
        ui.add_space(4.0);
        widgets::stat(ui, "Version", env!("CARGO_PKG_VERSION"));
        widgets::stat(
            ui,
            "Build",
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
        );
    });
}

fn preset_chip(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let bg = if selected {
        egui::Color32::from_rgba_unmultiplied(0x60, 0xCD, 0xFF, 50)
    } else {
        theme::panel_bg()
    };
    let fg = if selected {
        theme::accent()
    } else {
        theme::text()
    };
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(fg).strong())
            .fill(bg)
            .corner_radius(egui::CornerRadius::same(6))
            .min_size(egui::vec2(80.0, 28.0)),
    )
}

fn format_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else if ms.is_multiple_of(1000) {
        format!("{} s", ms / 1000)
    } else {
        format!("{:.1} s", ms as f64 / 1000.0)
    }
}
