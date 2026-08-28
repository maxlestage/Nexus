//! Mode PC : gamepad HID **BLE** générique, reconnu nativement par
//! Windows, macOS et Linux (aucun pilote requis).
//!
//! Le rapport correspond à `controller_core::report::pack_pc_report` :
//! 16 boutons + 4 axes 8 bits.

use esp_idf_sys as sys;
use esp_idf_sys::esp;
use std::ffi::{c_void, CString};
use std::sync::Mutex;

pub const PC_REPORT_ID: u8 = 1;

/// Descripteur HID : gamepad 16 boutons, X/Y/Z/Rz 8 bits.
pub const PC_REPORT_MAP: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x05, // Usage (Gamepad)
    0xA1, 0x01, // Collection (Application)
    0x85, PC_REPORT_ID, // Report ID
    0x05, 0x09, // Usage Page (Button)
    0x19, 0x01, // Usage Min (1)
    0x29, 0x10, // Usage Max (16)
    0x15, 0x00, // Logical Min (0)
    0x25, 0x01, // Logical Max (1)
    0x75, 0x01, // Report Size (1)
    0x95, 0x10, // Report Count (16)
    0x81, 0x02, // Input (Data, Var, Abs)
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x30, // Usage (X)
    0x09, 0x31, // Usage (Y)
    0x09, 0x32, // Usage (Z)
    0x09, 0x35, // Usage (Rz)
    0x15, 0x00, // Logical Min (0)
    0x26, 0xFF, 0x00, // Logical Max (255)
    0x75, 0x08, // Report Size (8)
    0x95, 0x04, // Report Count (4)
    0x81, 0x02, // Input
    0xC0, // End Collection
];

struct PcShared {
    dev: *mut sys::esp_hidd_dev_t,
    connected: bool,
}
unsafe impl Send for PcShared {}

static SHARED: Mutex<Option<PcShared>> = Mutex::new(None);

pub struct PcHid;

impl PcHid {
    pub fn start() -> anyhow::Result<Self> {
        let device_name = CString::new("Nexus One")?;
        let manufacturer = CString::new("Nexus")?;
        let serial = CString::new("0001")?;

        unsafe {
            let report_maps = [sys::esp_hid_raw_report_map_t {
                data: PC_REPORT_MAP.as_ptr(),
                len: PC_REPORT_MAP.len() as u16,
            }];
            let config = sys::esp_hid_device_config_t {
                vendor_id: 0x16C0,
                product_id: 0x05DF,
                version: 0x0100,
                device_name: device_name.as_ptr() as *mut _,
                manufacturer_name: manufacturer.as_ptr() as *mut _,
                serial_number: serial.as_ptr() as *mut _,
                report_maps: report_maps.as_ptr() as *mut _,
                report_maps_len: 1,
            };
            let mut dev: *mut sys::esp_hidd_dev_t = core::ptr::null_mut();
            esp!(sys::esp_hidd_dev_init(
                &config,
                sys::esp_hid_transport_t_ESP_HID_TRANSPORT_BLE,
                Some(hidd_event_handler),
                &mut dev,
            ))?;
            *SHARED.lock().unwrap() = Some(PcShared { dev, connected: false });
        }
        log::info!("HID BLE (mode PC) démarré : visible comme « Nexus One »");
        Ok(Self)
    }

    pub fn is_connected() -> bool {
        SHARED.lock().unwrap().as_ref().map(|s| s.connected).unwrap_or(false)
    }

    pub fn send_report(data: &[u8]) -> anyhow::Result<()> {
        let guard = SHARED.lock().unwrap();
        let Some(s) = guard.as_ref() else {
            anyhow::bail!("HID non initialisé");
        };
        if !s.connected {
            return Ok(());
        }
        unsafe {
            esp!(sys::esp_hidd_dev_input_set(
                s.dev,
                0,
                PC_REPORT_ID as usize,
                data.as_ptr() as *mut _,
                data.len(),
            ))?;
        }
        Ok(())
    }
}

unsafe extern "C" fn hidd_event_handler(
    _handler_args: *mut c_void,
    _base: sys::esp_event_base_t,
    event_id: i32,
    _event_data: *mut c_void,
) {
    match event_id as u32 {
        sys::esp_hidd_event_t_ESP_HIDD_CONNECT_EVENT => {
            log::info!("PC connecté");
            if let Some(s) = SHARED.lock().unwrap().as_mut() {
                s.connected = true;
            }
        }
        sys::esp_hidd_event_t_ESP_HIDD_DISCONNECT_EVENT => {
            log::info!("PC déconnecté");
            if let Some(s) = SHARED.lock().unwrap().as_mut() {
                s.connected = false;
            }
        }
        _ => {}
    }
}
