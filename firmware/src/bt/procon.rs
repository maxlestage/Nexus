//! Émulation Pro Controller en HID Bluetooth classique (BR/EDR), via le
//! composant `esp_hid` d'ESP-IDF (API `esp_hidd_dev_*`).
//!
//! La Switch se connecte à nous comme à un vrai Pro Controller :
//! VID 0x057E / PID 0x2009, classe d'appareil "gamepad", puis dialogue par
//! subcommands (géré par `controller_core::procon`, testé sur PC).

use esp_idf_sys as sys;
use esp_idf_sys::esp;
use std::ffi::{c_void, CString};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

/// Descripteur HID du Pro Controller (issu du reverse engineering
/// communautaire).
///
/// Comme sur la vraie manette : le rapport **0x3F** (11 octets) est la
/// section gamepad standard, servie pendant l'appairage ; les rapports
/// **0x30/0x21/0x81** (63 octets) sont des rapports vendeur, tout comme les
/// sorties 0x01/0x10. Les longueurs déclarées ici doivent correspondre aux
/// payloads réellement envoyés : le composant `esp_hid` d'ESP-IDF vérifie
/// la taille de chaque rapport contre cette map.
pub const PROCON_REPORT_MAP: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x05, // Usage (Gamepad)
    0xA1, 0x01, // Collection (Application)
    // ---- Rapport court 0x3F : 2 octets boutons + hat + 4 axes 16 bits ----
    0x85, 0x3F, //   Report ID (0x3F)
    0x05, 0x09, //   Usage Page (Button)
    0x19, 0x01, //   Usage Min (1)
    0x29, 0x0E, //   Usage Max (14)
    0x15, 0x00, //   Logical Min (0)
    0x25, 0x01, //   Logical Max (1)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x0E, //   Report Count (14)
    0x81, 0x02, //   Input (Data, Var, Abs)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x02, //   Report Count (2)
    0x81, 0x03, //   Input (Const) — bourrage sur 16 bits
    0x05, 0x01, //   Usage Page (Generic Desktop)
    0x09, 0x39, //   Usage (Hat switch)
    0x15, 0x00, 0x25, 0x07, // Logical 0..7
    0x35, 0x00, 0x46, 0x3B, 0x01, // Physical 0..315
    0x65, 0x14, //   Unit (degrés)
    0x75, 0x04, 0x95, 0x01, // 4 bits × 1
    0x81, 0x42, //   Input (Data, Var, Abs, Null state)
    0x75, 0x04, 0x95, 0x01, // 4 bits de bourrage
    0x81, 0x03, //   Input (Const)
    0x09, 0x30, //   Usage (X)
    0x09, 0x31, //   Usage (Y)
    0x09, 0x32, //   Usage (Z)
    0x09, 0x35, //   Usage (Rz)
    0x15, 0x00, //   Logical Min (0)
    0x27, 0xFF, 0xFF, 0x00, 0x00, // Logical Max (65535)
    0x75, 0x10, //   Report Size (16)
    0x95, 0x04, //   Report Count (4)
    0x81, 0x02, //   Input
    // ---- Rapports vendeur (protocole Pro Controller), 63 octets ----
    0x06, 0x00, 0xFF, // Usage Page (Vendor)
    0x85, 0x21, 0x09, 0x01, 0x75, 0x08, 0x95, 0x3F, 0x81, 0x03, // Input 0x21
    0x85, 0x30, 0x09, 0x02, 0x75, 0x08, 0x95, 0x3F, 0x81, 0x03, // Input 0x30
    0x85, 0x81, 0x09, 0x03, 0x75, 0x08, 0x95, 0x3F, 0x81, 0x03, // Input 0x81
    0x85, 0x01, 0x09, 0x04, 0x75, 0x08, 0x95, 0x3F, 0x91, 0x83, // Output 0x01
    0x85, 0x10, 0x09, 0x05, 0x75, 0x08, 0x95, 0x3F, 0x91, 0x83, // Output 0x10
    0x85, 0x80, 0x09, 0x06, 0x75, 0x08, 0x95, 0x3F, 0x91, 0x83, // Output 0x80
    0x85, 0x82, 0x09, 0x07, 0x75, 0x08, 0x95, 0x3F, 0x91, 0x83, // Output 0x82
    0xC0, // End Collection
];

/// Rapport de sortie reçu de la console (rumble / subcommand).
pub struct HostOutput {
    pub data: Vec<u8>,
}

struct ProconShared {
    dev: *mut sys::esp_hidd_dev_t,
    connected: bool,
    tx: Sender<HostOutput>,
}
unsafe impl Send for ProconShared {}

static SHARED: Mutex<Option<ProconShared>> = Mutex::new(None);

pub struct Procon {
    pub host_rx: Receiver<HostOutput>,
}

impl Procon {
    /// Démarre le périphérique HID classique. À appeler une fois, après
    /// `bt::init_stack()`.
    pub fn start() -> anyhow::Result<Self> {
        let (tx, rx) = channel();

        let device_name = CString::new("Pro Controller")?;
        let manufacturer = CString::new("Nintendo Co., Ltd.")?;
        let serial = CString::new("000000000001")?;

        unsafe {
            // Nom + classe d'appareil "gamepad" pour l'écran d'appairage.
            esp!(sys::esp_bt_gap_set_device_name(device_name.as_ptr()))?;
            let cod = sys::esp_bt_cod_t {
                _bitfield_align_1: [],
                _bitfield_1: sys::esp_bt_cod_t::new_bitfield_1(
                    0x02, // minor : gamepad
                    0x05, // major : peripheral
                    0x00, // service
                    0,
                ),
            };
            esp!(sys::esp_bt_gap_set_cod(cod, sys::esp_bt_cod_mode_t_ESP_BT_SET_COD_ALL))?;

            let report_maps = [sys::esp_hid_raw_report_map_t {
                data: PROCON_REPORT_MAP.as_ptr(),
                len: PROCON_REPORT_MAP.len() as u16,
            }];
            let config = sys::esp_hid_device_config_t {
                vendor_id: 0x057E,
                product_id: 0x2009,
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
                sys::esp_hid_transport_t_ESP_HID_TRANSPORT_BT,
                Some(hidd_event_handler),
                &mut dev,
            ))?;

            *SHARED.lock().unwrap() = Some(ProconShared { dev, connected: false, tx });

            // Visible et connectable : la Switch nous trouve dans
            // "Changer le style/l'ordre".
            esp!(sys::esp_bt_gap_set_scan_mode(
                sys::esp_bt_connection_mode_t_ESP_BT_CONNECTABLE,
                sys::esp_bt_discovery_mode_t_ESP_BT_GENERAL_DISCOVERABLE,
            ))?;
        }

        log::info!("HID classique démarré : en attente de la Switch");
        Ok(Self { host_rx: rx })
    }

    pub fn is_connected() -> bool {
        SHARED.lock().unwrap().as_ref().map(|s| s.connected).unwrap_or(false)
    }

    /// Envoie un rapport d'entrée à la console.
    pub fn send_input_report(report_id: u8, data: &[u8]) -> anyhow::Result<()> {
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
                report_id as usize,
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
    event_data: *mut c_void,
) {
    let param = event_data as *mut sys::esp_hidd_event_data_t;
    match event_id as u32 {
        sys::esp_hidd_event_t_ESP_HIDD_CONNECT_EVENT => {
            log::info!("Switch connectée");
            if let Some(s) = SHARED.lock().unwrap().as_mut() {
                s.connected = true;
            }
        }
        sys::esp_hidd_event_t_ESP_HIDD_DISCONNECT_EVENT => {
            log::info!("Switch déconnectée");
            if let Some(s) = SHARED.lock().unwrap().as_mut() {
                s.connected = false;
            }
        }
        sys::esp_hidd_event_t_ESP_HIDD_OUTPUT_EVENT => {
            // Rumble / subcommand de la console → main loop.
            if param.is_null() {
                return;
            }
            let out = &(*param).output;
            let mut data = Vec::with_capacity(out.length as usize + 1);
            // `report_id` est u16 dans les bindings ESP-IDF 5.x ; les IDs
            // du protocole Pro Controller tiennent tous sur un octet.
            data.push(out.report_id as u8);
            data.extend_from_slice(core::slice::from_raw_parts(out.data, out.length as usize));
            if let Some(s) = SHARED.lock().unwrap().as_ref() {
                let _ = s.tx.send(HostOutput { data });
            }
        }
        _ => {}
    }
}
