use crate::lang;
use crate::lang::Key::*;

use libc::{c_char, c_int, c_void};
use std::ffi::CStr;

#[repr(C)]
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
    // unsafe fn hsc_get_name(headset: *mut c_void) -> *const c_char;
    unsafe fn hsc_get_product_name(headset: *mut c_void) -> *const c_char;
    unsafe fn hsc_supports(headset: *mut c_void, cap: c_int) -> bool;
    unsafe fn hsc_get_battery(headset: *mut c_void, battery: *mut HscBattery) -> c_int;
}

pub fn query_device() -> Option<Device> {
    unsafe {
        let mut headsets: *mut c_void = std::ptr::null_mut();
        let count = hsc_discover(&mut headsets);

        if count > 0 {
            let headset_array =
                std::slice::from_raw_parts(headsets as *const *mut c_void, count as usize);

            let headset = headset_array[0]; // Just take the first headset found
            if hsc_supports(headset, BATTERY_CAPABILITY) {
                let mut battery = HscBattery {
                    level_percent: 0,
                    status: BatteryStatus::Unavailable,
                    voltage_mv: -1,
                    time_to_full_min: -1,
                    time_to_empty_min: -1,
                };
                if hsc_get_battery(headset, &mut battery) == 0 {
                    let product_name = CStr::from_ptr(hsc_get_product_name(headset))
                        .to_str()
                        .unwrap_or("Unknown")
                        .to_string();

                    hsc_free_headsets(headsets, count);
                    return Some(Device {
                        product_name,
                        battery,
                    });
                }
            }
            // Also release discovered devices after unsupported/failed reads.
            hsc_free_headsets(headsets, count);
        }
    }

    None
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
