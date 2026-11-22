use crate::lang;
use crate::lang::Key::*;

use libc::{c_char, c_int, c_void};
use std::ffi::CStr;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HscBattery {
    pub level_percent: c_int,
    pub status: BatteryStatus,
    pub voltage_mv: c_int,
    pub time_to_full_min: c_int,
    pub time_to_empty_min: c_int,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum BatteryStatus {
    #[default]
    Unavailable,
    Charging,
    Available,
    HidError,
    Timeout,
}

const BATTERY_CAPABILITY: c_int = 1;

#[link(name = "headsetcontrol_static")]
unsafe extern "C" {
    unsafe fn hsc_discover(headsets: *mut *mut c_void) -> c_int;
    unsafe fn hsc_free_headsets(headsets: *mut c_void, count: c_int);
    unsafe fn hsc_get_name(headset: *mut c_void) -> *const c_char;
    unsafe fn hsc_get_product_name(headset: *mut c_void) -> *const c_char;
    unsafe fn hsc_supports(headset: *mut c_void, cap: c_int) -> bool;
    unsafe fn hsc_get_battery(headset: *mut c_void, battery: *mut HscBattery) -> c_int;
}

pub fn query_device() -> Option<Device> {
    unsafe {
        let mut headsets: *mut c_void = std::ptr::null_mut();
        let count = hsc_discover(&mut headsets);

        if count <= 0 {
            log_outcome(Outcome::NoneFound(count));
            return None;
        }

        let headset_array =
            std::slice::from_raw_parts(headsets as *const *mut c_void, count as usize);

        let headset = headset_array[0]; // Just take the first headset found
        let model = model_name(headset);
        let mut result = None;
        let outcome;

        if hsc_supports(headset, BATTERY_CAPABILITY) {
            let mut battery = HscBattery {
                level_percent: 0,
                status: BatteryStatus::Unavailable,
                voltage_mv: -1,
                time_to_full_min: -1,
                time_to_empty_min: -1,
            };
            let rc = hsc_get_battery(headset, &mut battery);
            if rc == 0 {
                // Copy the name before freeing: it points into the headset object.
                // The product name is NULL when the device reports none; the
                // model name is always set.
                let product_name = c_string(hsc_get_product_name(headset))
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| model_name(headset));

                outcome = Outcome::Reading(model, battery.status);
                result = Some(Device {
                    product_name,
                    battery,
                });
            } else {
                outcome = Outcome::ReadFailed(model, rc);
            }
        } else {
            outcome = Outcome::NoBatteryStatus(model);
        }

        // Always free the headsets, even when the battery read fails. Each headset
        // holds open HID handles that are only closed here; skipping this on the
        // failure path leaked handles every poll while the headset was off.
        hsc_free_headsets(headsets, count);
        log_outcome(outcome);
        result
    }
}

/// An owned copy of a C string, None for NULL
unsafe fn c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// The headset model's name, which HeadsetControl always sets
unsafe fn model_name(headset: *mut c_void) -> String {
    unsafe { c_string(hsc_get_name(headset)) }.unwrap_or_else(|| "Unknown".to_string())
}

/// What a poll found, logged when it changes so that a tray showing no
/// headset leaves a trace of why
#[derive(Clone, Debug, PartialEq)]
enum Outcome {
    NoneFound(c_int),
    NoBatteryStatus(String),
    ReadFailed(String, c_int),
    Reading(String, BatteryStatus),
}

impl Outcome {
    fn is_failure(&self) -> bool {
        !matches!(self, Outcome::Reading(..))
    }
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::NoneFound(count) => {
                write!(
                    f,
                    "No supported headset found (hsc_discover returned {count})"
                )
            }
            Outcome::NoBatteryStatus(model) => {
                write!(f, "{model} does not report its battery status")
            }
            Outcome::ReadFailed(model, rc) => {
                // hsc_result_t in headsetcontrol_c.h
                let reason = match rc {
                    -2 => "not supported",
                    -3 => "device offline",
                    -4 => "timeout",
                    -5 => "HID error",
                    -6 => "invalid parameter",
                    _ => "error",
                };
                write!(f, "{model}: failed to read the battery: {reason} ({rc})")
            }
            Outcome::Reading(model, status) => write!(f, "{model}: battery status {status:?}"),
        }
    }
}

/// Decides which poll outcomes to log. Every change is logged, except a
/// change back to an outcome logged in the last ten minutes: that one is
/// held back until it has lasted half a minute, so that a read that flaps
/// every poll does not fill the log. The next line says how many changes
/// were not logged, and a line held back says since when it holds.
struct OutcomeLog {
    logged: Option<Outcome>, // what the log says last
    seen: Option<Outcome>,   // what the last poll found
    seen_since: Option<Instant>,
    recent: Vec<(Outcome, Instant)>,
    unlogged_changes: u32,
}

impl OutcomeLog {
    const QUIET: Duration = Duration::from_secs(600);
    const LASTING: Duration = Duration::from_secs(30);

    const fn new() -> Self {
        Self {
            logged: None,
            seen: None,
            seen_since: None,
            recent: Vec::new(),
            unlogged_changes: 0,
        }
    }

    /// The line to log for this poll's outcome, if any
    fn note(&mut self, outcome: &Outcome, now: Instant) -> Option<String> {
        let changed = self.seen.as_ref() != Some(outcome);
        if changed {
            self.seen = Some(outcome.clone());
            self.seen_since = Some(now);
        }
        let since = self.seen_since.unwrap_or(now);
        if self.logged.as_ref() == Some(outcome) {
            if changed {
                self.unlogged_changes += 1; // back to what the log says, after changes not logged
            }
            return None;
        }
        let logged_lately = self
            .recent
            .iter()
            .any(|(logged, at)| logged == outcome && now.duration_since(*at) < Self::QUIET);
        if logged_lately && now.duration_since(since) < Self::LASTING {
            if changed {
                self.unlogged_changes += 1;
            }
            return None;
        }

        let mut line = outcome.to_string();
        let mut unlogged = std::mem::take(&mut self.unlogged_changes);
        if !changed {
            // logged late: the change to this outcome is one of those counted
            unlogged = unlogged.saturating_sub(1);
            line += &format!(" (for {} s)", now.duration_since(since).as_secs());
        }
        match unlogged {
            0 => {}
            1 => line += " (1 change before it not logged)",
            n => line += &format!(" ({n} changes before it not logged)"),
        }
        self.recent.retain(|(logged, _)| logged != outcome);
        self.recent.push((outcome.clone(), now));
        self.logged = Some(outcome.clone());
        Some(line)
    }
}

fn log_outcome(outcome: Outcome) {
    static STATE: Mutex<OutcomeLog> = Mutex::new(OutcomeLog::new());
    let mut state = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(line) = state.note(&outcome, Instant::now()) {
        if outcome.is_failure() {
            log::warn!("{line}");
        } else {
            log::info!("{line}");
        }
    }
}

pub struct Device {
    pub product_name: String,
    pub battery: HscBattery,
}

impl Device {
    pub fn status_text(&self) -> Option<&'static str> {
        match self.battery.status {
            BatteryStatus::Charging => Some(lang::t(device_charging)),
            BatteryStatus::Available => None,
            BatteryStatus::Unavailable => Some(lang::t(battery_unavailable)),
            _ => Some(lang::t(device_disconnected)),
        }
    }
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.battery.level_percent > 0 {
            write!(
                f,
                "{name}: {battery}%",
                name = self.product_name,
                battery = self.battery.level_percent,
            )?;
        } else {
            write!(f, "{}", self.product_name)?;
        }

        if let Some(status) = self.status_text() {
            write!(f, " {status}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn off() -> Outcome {
        Outcome::ReadFailed("Model".to_string(), -3)
    }
    fn on() -> Outcome {
        Outcome::Reading("Model".to_string(), BatteryStatus::Available)
    }

    /// Feeds one outcome per second and collects what would be logged
    fn run(log: &mut OutcomeLog, start: Instant, polls: &[(u64, Outcome)]) -> Vec<String> {
        polls
            .iter()
            .filter_map(|(second, outcome)| log.note(outcome, start + Duration::from_secs(*second)))
            .collect()
    }

    #[test]
    fn logs_every_new_outcome() {
        let mut log = OutcomeLog::new();
        let lines = run(
            &mut log,
            Instant::now(),
            &[(0, off()), (1, off()), (2, on()), (3, on())],
        );
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("device offline (-3)"));
        assert!(lines[1].contains("battery status Available"));
    }

    #[test]
    fn a_lasting_change_back_is_logged_late() {
        // Off, on, off again: the second off is held back, then logged after 30 s
        let mut log = OutcomeLog::new();
        let polls: Vec<(u64, Outcome)> = [(0, off()), (5, on())]
            .into_iter()
            .chain((10..=45).map(|s| (s, off())))
            .collect();
        let lines = run(&mut log, Instant::now(), &polls);
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(
            lines[2].contains("device offline") && lines[2].contains("(for 30 s)"),
            "{lines:?}"
        );
        assert!(!lines[2].contains("not logged"), "{lines:?}");
    }

    #[test]
    fn flapping_is_held_back_and_counted() {
        let mut log = OutcomeLog::new();
        let start = Instant::now();
        let polls: Vec<(u64, Outcome)> = (0..200)
            .map(|s| (s, if s % 2 == 0 { on() } else { off() }))
            .collect();
        assert_eq!(run(&mut log, start, &polls).len(), 2);
        // Ten minutes on, the next change is logged with the count of those held back
        let later = run(&mut log, start, &[(700, on())]);
        assert_eq!(later.len(), 1);
        assert!(
            later[0].contains("(198 changes before it not logged)"),
            "{later:?}"
        );
    }
}
