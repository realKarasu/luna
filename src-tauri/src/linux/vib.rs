//! Graphics cards from sysfs, and NVIDIA digital vibrance through `nvidia-settings`, which
//! comes with the NVIDIA driver. AMD and Intel offer no vibrance setting on Linux, so their
//! screens keep the gamma curve only.

use crate::screens::MonitorInfo;
use crate::vibrance::{Vendor, VibranceBackend, VibranceRange};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const TOOL: &str = "nvidia-settings";

struct Card {
    vendor: Vendor,
    /// Connector names such as `DP-1` that report a screen.
    connected: Vec<String>,
}

fn cards() -> Vec<Card> {
    let Ok(dir) = std::fs::read_dir("/sys/class/drm") else { return Vec::new() };
    let mut names: Vec<String> = dir.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect();
    names.sort();
    let is_card = |n: &str| n.strip_prefix("card").is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()));
    names
        .iter()
        .filter(|n| is_card(n))
        .filter_map(|card| {
            let base = Path::new("/sys/class/drm").join(card);
            let raw = std::fs::read_to_string(base.join("device").join("vendor")).ok()?;
            let id = u32::from_str_radix(raw.trim().trim_start_matches("0x"), 16).ok()?;
            let prefix = format!("{card}-");
            let connected = names
                .iter()
                .filter_map(|n| n.strip_prefix(&prefix))
                .filter(|conn| {
                    std::fs::read_to_string(Path::new("/sys/class/drm").join(format!("{prefix}{conn}")).join("status"))
                        .is_ok_and(|s| s.trim() == "connected")
                })
                .map(str::to_string)
                .collect();
            Some(Card { vendor: Vendor::from_id(id), connected })
        })
        .collect()
}

/// Which card drives a screen. The X output name often matches the kernel connector name;
/// when it does not, a machine with a single kind of card still has its answer.
fn vendor_for(cards: &[Card], output: &str) -> Vendor {
    if let Some(c) = cards.iter().find(|c| c.connected.iter().any(|n| n.eq_ignore_ascii_case(output))) {
        return c.vendor;
    }
    match cards.first() {
        Some(f) if cards.iter().all(|c| c.vendor == f.vendor) => f.vendor,
        _ => Vendor::Generic,
    }
}

fn run(args: &[&str]) -> Option<String> {
    let out = Command::new(TOOL).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn query(output: &str) -> Option<i32> {
    let text = run(&["-t", "-q", &format!("[DPY:{output}]/DigitalVibrance")])?;
    text.lines().find_map(|l| l.trim().parse().ok())
}

/// How many writes are still waiting, so quitting can let them land.
#[derive(Default)]
struct Pending {
    count: Mutex<usize>,
    idle: Condvar,
}

impl Pending {
    fn add(&self) {
        *self.count.lock().unwrap_or_else(|e| e.into_inner()) += 1;
    }

    fn done(&self, n: usize) {
        let mut c = self.count.lock().unwrap_or_else(|e| e.into_inner());
        *c = c.saturating_sub(n);
        if *c == 0 {
            self.idle.notify_all();
        }
    }

    fn wait(&self, limit: Duration) {
        let end = Instant::now() + limit;
        let mut c = self.count.lock().unwrap_or_else(|e| e.into_inner());
        while *c > 0 {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            c = self.idle.wait_timeout(c, left).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}

/// Writes go through a worker thread: starting `nvidia-settings` takes tens of
/// milliseconds, and only the latest level matters when several queue up.
fn worker(output: String, rx: Receiver<i32>, pending: Arc<Pending>) {
    while let Ok(mut level) = rx.recv() {
        let mut taken = 1;
        while let Ok(next) = rx.try_recv() {
            level = next;
            taken += 1;
        }
        run(&["-a", &format!("[DPY:{output}]/DigitalVibrance={level}")]);
        pending.done(taken);
    }
}

struct NvSettings {
    output: String,
    range: VibranceRange,
    original: i32,
    tx: Sender<i32>,
    pending: Arc<Pending>,
}

impl VibranceBackend for NvSettings {
    fn name(&self) -> &'static str {
        "NVIDIA digital vibrance"
    }

    fn range(&self) -> VibranceRange {
        self.range
    }

    fn get(&mut self) -> Option<i32> {
        query(&self.output)
    }

    fn set(&mut self, level: i32) -> bool {
        let level = level.clamp(self.range.min, self.range.max);
        self.pending.add();
        if self.tx.send(level).is_err() {
            self.pending.done(1);
            return false;
        }
        true
    }

    fn original(&self) -> i32 {
        self.original
    }

    fn set_original(&mut self, level: i32) {
        self.original = level;
    }
}

#[derive(Default)]
pub struct Drivers {
    pending: Arc<Pending>,
}

impl Drivers {
    pub fn new() -> Drivers {
        Drivers::default()
    }

    /// Fills in each screen's graphics card and returns the driver vibrance found per screen.
    pub fn find(&mut self, monitors: &mut [MonitorInfo], _reload: bool) -> HashMap<String, Box<dyn VibranceBackend>> {
        let cards = cards();
        for m in monitors.iter_mut() {
            m.vendor = vendor_for(&cards, &m.id);
            m.gpu = match m.vendor {
                Vendor::Generic => String::new(),
                v => format!("{} graphics", v.label()),
            };
        }
        let mut found: HashMap<String, Box<dyn VibranceBackend>> = HashMap::new();
        let Some(range) = VibranceRange::nvidia(-1024, 1023, 0) else { return found };
        for m in monitors.iter().filter(|m| m.vendor == Vendor::Nvidia) {
            let Some(level) = query(&m.id) else { continue };
            let (tx, rx) = mpsc::channel();
            let pending = self.pending.clone();
            let output = m.id.clone();
            let spawned = std::thread::Builder::new()
                .name("luna-nvidia".into())
                .spawn(move || worker(output, rx, pending))
                .is_ok();
            if spawned {
                found.insert(
                    m.id.clone(),
                    Box::new(NvSettings { output: m.id.clone(), range, original: level, tx, pending: self.pending.clone() }),
                );
            }
        }
        found
    }

    /// Lets the last writes (the original levels on quit) reach the driver.
    pub fn release(&mut self) {
        self.pending.wait(Duration::from_secs(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_card() {
        let cards = vec![
            Card { vendor: Vendor::Amd, connected: vec!["DP-1".into()] },
            Card { vendor: Vendor::Nvidia, connected: vec!["HDMI-A-1".into()] },
        ];
        assert_eq!(vendor_for(&cards, "DP-1"), Vendor::Amd);
        assert_eq!(vendor_for(&cards, "DP-0"), Vendor::Generic);
        let one = vec![Card { vendor: Vendor::Nvidia, connected: Vec::new() }];
        assert_eq!(vendor_for(&one, "DP-0"), Vendor::Nvidia);
    }
}
