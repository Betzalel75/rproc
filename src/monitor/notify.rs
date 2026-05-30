//! System-wide threshold notifications.
//!
//! When CPU or RAM usage exceeds a configured threshold (and the cooldown
//! period has elapsed since the last alert for that metric), a desktop
//! notification is fired via `notify-send`. If `notify-send` is unavailable
//! the notification is silently dropped — the sampler thread never blocks
//! on it.
//!
//! A threshold of 0 means "disabled" for that metric.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// User-configurable notification settings, shared lock-free with the
/// sampler thread (read) and the UI thread (write).
#[derive(Clone)]
pub struct Thresholds {
    /// CPU usage percentage (0–100). 0 = notifications disabled.
    pub cpu_pct: Arc<AtomicU8>,
    /// RAM usage percentage (0–100). 0 = notifications disabled.
    pub ram_pct: Arc<AtomicU8>,
    /// Minimum seconds between two notifications for the *same* metric.
    /// Avoids spam when a threshold is crossed repeatedly.
    pub cooldown_secs: Arc<AtomicU64>,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cpu_pct: Arc::new(AtomicU8::new(0)),
            ram_pct: Arc::new(AtomicU8::new(0)),
            cooldown_secs: Arc::new(AtomicU64::new(DEFAULT_COOLDOWN_SECS)),
        }
    }
}

pub const DEFAULT_COOLDOWN_SECS: u64 = 300; // 5 minutes
pub const MIN_COOLDOWN_SECS: u64 = 30;

/// Per-metric notification state: tracks the last time a threshold was
/// breached so we can enforce the cooldown without a global lock.
pub struct Watcher {
    thresholds: Thresholds,
    last_cpu: Instant,
    last_ram: Instant,
}

impl Watcher {
    pub fn new(thresholds: Thresholds) -> Self {
        Self {
            thresholds,
            // Set both timestamps far in the past so the *first* breach
            // fires immediately — the user has just configured thresholds.
            last_cpu: Instant::now() - std::time::Duration::from_secs(u64::MAX / 2),
            last_ram: Instant::now() - std::time::Duration::from_secs(u64::MAX / 2),
        }
    }

    /// Check system metrics against the configured thresholds. Fires a
    /// desktop notification for each breached metric that is outside its
    /// cooldown window. Best-effort: failures are silently ignored.
    pub fn check(&mut self, cpu_pct: f32, ram_pct: f32) {
        let cooldown = std::time::Duration::from_secs(
            self.thresholds.cooldown_secs.load(Ordering::Relaxed),
        );
        let now = Instant::now();

        let cpu_limit = self.thresholds.cpu_pct.load(Ordering::Relaxed);
        if cpu_limit > 0 && cpu_pct >= cpu_limit as f32 && now.duration_since(self.last_cpu) >= cooldown {
            self.last_cpu = now;
            notify(
                "rproc",
                &format!("CPU usage at {cpu_pct:.0}% (threshold: {cpu_limit}%)"),
            );
        }

        let ram_limit = self.thresholds.ram_pct.load(Ordering::Relaxed);
        if ram_limit > 0 && ram_pct >= ram_limit as f32 && now.duration_since(self.last_ram) >= cooldown {
            self.last_ram = now;
            notify(
                "rproc",
                &format!("RAM usage at {ram_pct:.0}% (threshold: {ram_limit}%)"),
            );
        }
    }
}

/// Fire a desktop notification via `notify-send`. Non-blocking spawn,
/// failures are silently dropped — the sampler must never stall on a
/// missing notification daemon.
fn notify(summary: &str, body: &str) {
    let _ = std::process::Command::new("notify-send")
        .args([
            "--app-name=rproc",
            "--icon=rproc",
            "--urgency=normal",
            "--category=system",
            summary,
            body,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
