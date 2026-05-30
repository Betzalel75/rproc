//! Internationalisation — lock-free global, modular per-language files.
//!
//! Adding a new language:
//! 1. Create `xx.rs` with a `pub const MSG: Messages = Messages { ... };`
//! 2. Add the variant to the `Lang` enum below.
//! 3. Add the arm in `messages()`.
//! 4. Add a `"xx"` case in `Lang::from_str()`.
//! 5. Translate all fields.
//!
//! The `Messages` struct is a flat bag of `&'static str` — no allocations,
//! no hashing, pure pointer dereference per access.

use std::sync::atomic::{AtomicU8, Ordering};

mod en;
mod fr;

// ---------------------------------------------------------------------------
// Language enum + global
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Lang {
    En = 0,
    Fr = 1,
}

impl Lang {
    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "en" | "english" => Some(Lang::En),
            "fr" | "french" | "français" | "francais" => Some(Lang::Fr),
            _ => None,
        }
    }
}

static LANG: AtomicU8 = AtomicU8::new(0); // 0 = en, default

pub fn init(lang: Lang) {
    LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn set_lang(lang: Lang) {
    LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        0 => Lang::En,
        _ => Lang::Fr,
    }
}

/// Probe the system locale and return the best-matching `Lang`.
/// Reads `$LANG` (and falls back to `$LC_ALL`, `$LC_MESSAGES`).
/// Returns `Lang::En` if nothing matches.
pub fn detect_system_lang() -> Lang {
    for var in ["LANG", "LC_ALL", "LC_MESSAGES"] {
        if let Ok(val) = std::env::var(var) {
            let code = val.split('.').next().unwrap_or(&val);
            let lang = code.split('_').next().unwrap_or(code);
            if let Some(l) = Lang::from_str(lang) {
                return l;
            }
        }
    }
    Lang::En
}

/// Returns the active message bundle. Cheap: one atomic load + pointer
/// dereference. Call this wherever you currently have a hardcoded `"Foo"`.
#[inline]
pub fn m() -> &'static Messages {
    messages(current())
}

fn messages(lang: Lang) -> &'static Messages {
    match lang {
        Lang::En => &en::MSG,
        Lang::Fr => &fr::MSG,
    }
}

// ---------------------------------------------------------------------------
// Message bundle — one field per translatable string.
// Field names mirror the English text where possible.
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct Messages {
    // -- Sidebar ------------------------------------------------------------
    pub tab_processes: &'static str,
    pub tab_performance: &'static str,
    pub tab_startup: &'static str,
    pub tab_services: &'static str,
    pub tab_settings: &'static str,

    // -- Processes tab ------------------------------------------------------
    pub proc_heading: &'static str,
    pub proc_end_task: &'static str,
    pub proc_force_kill: &'static str,
    pub proc_end_task_n: &'static str,   // "End task ({n})"
    pub proc_force_kill_n: &'static str, // "Force kill ({n})"
    pub proc_filter_hint: &'static str,
    pub proc_view_tree: &'static str,
    pub proc_view_grouped: &'static str,
    pub proc_col_name: &'static str,
    pub proc_col_pid: &'static str,
    pub proc_col_user: &'static str,
    pub proc_col_cpu: &'static str,      // "CPU  {:.0}%"
    pub proc_col_mem: &'static str,      // "Memory  {:.0}%"
    pub proc_col_disk: &'static str,
    pub proc_col_status: &'static str,
    pub proc_section_apps: &'static str,
    pub proc_section_bg: &'static str,
    pub proc_count: &'static str,        // "{total} processes"
    pub proc_sampling: &'static str,     // "sampling…"

    // Process context menu
    pub ctx_end_task: &'static str,
    pub ctx_force_kill: &'static str,
    pub ctx_suspend: &'static str,
    pub ctx_resume: &'static str,
    pub ctx_open_location: &'static str,
    pub ctx_search_online: &'static str,
    pub ctx_copy_pid: &'static str,
    pub ctx_copy_name: &'static str,
    pub ctx_copy_cmd: &'static str,
    pub ctx_properties: &'static str,
    pub ctx_end_all: &'static str,        // "End all ({n})"
    pub ctx_force_kill_all: &'static str, // "Force kill all ({n})"
    pub ctx_suspend_all: &'static str,
    pub ctx_resume_all: &'static str,
    pub ctx_copy_all_pids: &'static str,
    pub ctx_properties_main: &'static str, // "Properties (main process)"

    // Properties window
    pub prop_title: &'static str,         // "Properties: {name}"
    pub prop_reload: &'static str,
    pub prop_pid: &'static str,
    pub prop_parent_pid: &'static str,
    pub prop_user: &'static str,
    pub prop_status: &'static str,
    pub prop_cpu: &'static str,
    pub prop_mem_rss: &'static str,
    pub prop_virt_mem: &'static str,
    pub prop_threads: &'static str,
    pub prop_fds: &'static str,
    pub prop_running_for: &'static str,
    pub prop_executable: &'static str,
    pub prop_working_dir: &'static str,
    pub prop_config: &'static str,
    pub prop_cmdline: &'static str,
    pub prop_dash: &'static str,          // "—" placeholder

    // Process status labels
    pub status_running: &'static str,
    pub status_idle: &'static str,
    pub status_waiting: &'static str,
    pub status_stopped: &'static str,
    pub status_zombie: &'static str,

    // -- Performance tab ----------------------------------------------------
    pub perf_cpu: &'static str,
    pub perf_memory: &'static str,
    pub perf_gpu: &'static str,           // "GPU {i} ({vendor})"
    pub perf_disk: &'static str,          // "Disk {name}"
    pub perf_show_details: &'static str,
    pub perf_hide_details: &'static str,
    pub perf_cpu_total_plot: &'static str,
    pub perf_current: &'static str,
    pub perf_uptime: &'static str,
    pub perf_per_core: &'static str,
    pub perf_core: &'static str,          // "Core {n}"
    pub perf_ram_plot: &'static str,
    pub perf_used: &'static str,
    pub perf_available: &'static str,
    pub perf_swap_total: &'static str,
    pub perf_swap_used: &'static str,
    pub perf_swap_plot: &'static str,
    pub perf_disk_plot: &'static str,
    pub perf_read: &'static str,
    pub perf_write: &'static str,
    pub perf_total: &'static str,
    pub perf_used_space: &'static str,
    pub perf_mount: &'static str,
    pub perf_net_plot: &'static str,
    pub perf_receive: &'static str,
    pub perf_send: &'static str,
    pub perf_total_recv: &'static str,
    pub perf_total_sent: &'static str,
    pub perf_gpu_plot: &'static str,
    pub perf_gpu_util: &'static str,
    pub perf_gpu_vram: &'static str,
    pub perf_gpu_temp: &'static str,
    pub perf_gpu_power: &'static str,
    pub perf_gpu_core_clock: &'static str,
    pub perf_gpu_mem_clock: &'static str,
    pub perf_gpu_unavailable: &'static str,
    pub perf_no_disk: &'static str,
    pub perf_no_iface: &'static str,
    pub perf_no_gpu: &'static str,

    // Interface kind labels
    pub iface_wifi: &'static str,
    pub iface_mobile: &'static str,
    pub iface_usb: &'static str,
    pub iface_ethernet: &'static str,
    pub iface_network: &'static str,

    // -- Startup tab --------------------------------------------------------
    pub startup_heading: &'static str,
    pub startup_filter_hint: &'static str,
    pub startup_desc: &'static str,
    pub startup_min_boot: &'static str,
    pub startup_section_normal: &'static str,
    pub startup_section_protected: &'static str,
    pub startup_protected_desc: &'static str,
    pub startup_enabled: &'static str,
    pub startup_disabled: &'static str,
    pub startup_protected_label: &'static str,
    pub startup_boot_presets: &'static [(&'static str, &'static str)], // [(label, value_display)]

    // -- Services tab -------------------------------------------------------
    pub svc_heading: &'static str,
    pub svc_filter_hint: &'static str,
    pub svc_running_only: &'static str,
    pub svc_desc: &'static str,
    pub svc_col_unit: &'static str,
    pub svc_col_scope: &'static str,
    pub svc_col_active: &'static str,
    pub svc_col_sub: &'static str,
    pub svc_col_desc: &'static str,
    pub svc_col_actions: &'static str,
    pub svc_start: &'static str,
    pub svc_stop: &'static str,
    pub svc_restart: &'static str,
    pub svc_scope_system: &'static str,
    pub svc_scope_user: &'static str,
    pub svc_units_shown: &'static str,    // "{n} units shown"
    pub svc_reload: &'static str,
    pub svc_ctx_copy_unit: &'static str,
    pub svc_ctx_properties: &'static str,
    pub svc_prop_title: &'static str,
    pub svc_prop_unit: &'static str,
    pub svc_prop_scope: &'static str,
    pub svc_prop_desc: &'static str,
    pub svc_prop_load: &'static str,
    pub svc_prop_active: &'static str,
    pub svc_prop_sub: &'static str,
    pub svc_prop_unit_file_state: &'static str,
    pub svc_prop_main_pid: &'static str,
    pub svc_prop_user: &'static str,
    pub svc_prop_mem: &'static str,
    pub svc_prop_tasks: &'static str,
    pub svc_prop_unit_file: &'static str,
    pub svc_prop_dropins: &'static str,
    pub svc_prop_workdir: &'static str,
    pub svc_prop_execstart: &'static str,

    // -- Settings tab -------------------------------------------------------
    pub set_heading: &'static str,
    pub set_desc: &'static str,
    pub set_refresh_title: &'static str,
    pub set_refresh_desc: &'static str,
    pub set_custom: &'static str,
    pub set_currently: &'static str,      // "Currently sampling every {}"
    pub set_bg_title: &'static str,
    pub set_bg_desc: &'static str,
    pub set_bg_checkbox: &'static str,
    pub set_bg_running: &'static str,
    pub set_bg_off: &'static str,
    pub set_notif_title: &'static str,
    pub set_notif_desc: &'static str,
    pub set_notif_cpu: &'static str,
    pub set_notif_ram: &'static str,
    pub set_notif_cooldown: &'static str,
    pub set_appearance_title: &'static str,
    pub set_appearance_desc: &'static str,
    pub set_theme_system: &'static str,
    pub set_theme_dark: &'static str,
    pub set_theme_light: &'static str,
    pub set_about_title: &'static str,
    pub set_version: &'static str,
    pub set_build: &'static str,
    pub set_build_debug: &'static str,
    pub set_build_release: &'static str,

    // -- Notifications ------------------------------------------------------
    pub notif_cpu_body: &'static str,     // "CPU usage at {:.0}% (threshold: {}%)"
    pub notif_ram_body: &'static str,

    // -- Misc ---------------------------------------------------------------
    pub open_in_fm: &'static str,         // "Open in file manager"
    pub gpu_unavailable_detail_intel: &'static str,
    pub gpu_unavailable_detail_other: &'static str,
    pub gpu_unavailable_fix_label: &'static str,
    pub gpu_unavailable_fix_cap: &'static str,
    pub gpu_unavailable_fix_sysctl: &'static str,
    pub perf_partitions: &'static str,
    pub perf_partition: &'static str,

    // -- Startup scope badges -----------------------------------------------
    pub scope_user_autostart: &'static str,
    pub scope_system_autostart: &'static str,
    pub scope_systemd_system: &'static str,
    pub scope_systemd_user: &'static str,

    // -- Startup context menu -----------------------------------------------
    pub startup_ctx_copy_name: &'static str,
    pub startup_ctx_open_desktop: &'static str,
    pub startup_ctx_properties: &'static str,

    // -- Startup properties -------------------------------------------------
    pub startup_prop_title: &'static str,
    pub startup_prop_name: &'static str,
    pub startup_prop_source: &'static str,
    pub startup_prop_description: &'static str,
    pub startup_prop_boot_time: &'static str,
    pub startup_prop_state: &'static str,
    pub startup_prop_state_protected: &'static str,
    pub startup_prop_state_enabled: &'static str,
    pub startup_prop_state_disabled: &'static str,
    pub startup_prop_desktop_file: &'static str,
    pub startup_prop_exec: &'static str,
    pub startup_prop_icon: &'static str,
}
