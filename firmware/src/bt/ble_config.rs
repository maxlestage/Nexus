//! Service GATT BLE de configuration, consommé par l'application iPhone.
//!
//! Structure :
//! - caractéristique RX (write) : l'app écrit un `protocol::Request`
//!   encodé en postcard ;
//! - caractéristique TX (notify) : la manette répond avec un
//!   `protocol::Response`.
//!
//! Les requêtes sont poussées vers la boucle principale via un canal ;
//! c'est elle qui détient l'`Engine` et décide des réponses.

use controller_core::protocol;
use esp_idf_sys as sys;
use esp_idf_sys::esp;
use std::ffi::c_void;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

/// Indices dans la table d'attributs.
const IDX_SVC: usize = 0;
const IDX_RX_DECL: usize = 1;
const IDX_RX_VAL: usize = 2;
const IDX_TX_DECL: usize = 3;
const IDX_TX_VAL: usize = 4;
const IDX_TX_CCCD: usize = 5;
const NUM_ATTRS: usize = 6;

const APP_ID: u16 = 0x4E58; // "NX"

struct BleShared {
    gatts_if: sys::esp_gatt_if_t,
    conn_id: u16,
    connected: bool,
    notifications_on: bool,
    handles: [u16; NUM_ATTRS],
    tx: Sender<Vec<u8>>,
}

static SHARED: Mutex<Option<BleShared>> = Mutex::new(None);

/// UUID 128 bits little-endian depuis la forme texte du protocole.
fn uuid128(text: &str) -> [u8; 16] {
    let hex: Vec<u8> = text
        .bytes()
        .filter(u8::is_ascii_hexdigit)
        .collect();
    assert_eq!(hex.len(), 32);
    let mut out = [0u8; 16];
    for i in 0..16 {
        let hi = (hex[2 * i] as char).to_digit(16).unwrap() as u8;
        let lo = (hex[2 * i + 1] as char).to_digit(16).unwrap() as u8;
        // GATT attend l'UUID en little-endian.
        out[15 - i] = (hi << 4) | lo;
    }
    out
}

pub struct BleConfigService {
    /// Requêtes brutes (postcard) écrites par l'app.
    pub rx: Receiver<Vec<u8>>,
}

impl BleConfigService {
    pub fn start() -> anyhow::Result<Self> {
        let (tx, rx) = channel();
        *SHARED.lock().unwrap() = Some(BleShared {
            gatts_if: sys::ESP_GATT_IF_NONE as _,
            conn_id: 0,
            connected: false,
            notifications_on: false,
            handles: [0; NUM_ATTRS],
            tx,
        });
        unsafe {
            esp!(sys::esp_ble_gatts_register_callback(Some(gatts_event_handler)))?;
            esp!(sys::esp_ble_gap_register_callback(Some(gap_event_handler)))?;
            esp!(sys::esp_ble_gatts_app_register(APP_ID))?;
            esp!(sys::esp_ble_gatt_set_local_mtu(protocol::MAX_MSG_LEN as u16 + 5))?;
        }
        Ok(Self { rx })
    }

    /// Notifie une réponse (encodée postcard) vers l'app.
    pub fn send(payload: &[u8]) -> anyhow::Result<()> {
        let guard = SHARED.lock().unwrap();
        let Some(s) = guard.as_ref() else {
            anyhow::bail!("BLE non initialisé");
        };
        if !s.connected || !s.notifications_on {
            return Ok(());
        }
        unsafe {
            esp!(sys::esp_ble_gatts_send_indicate(
                s.gatts_if,
                s.conn_id,
                s.handles[IDX_TX_VAL],
                payload.len() as u16,
                payload.as_ptr() as *mut _,
                false, // notification (pas d'indication)
            ))?;
        }
        Ok(())
    }

    pub fn is_connected() -> bool {
        SHARED
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.connected)
            .unwrap_or(false)
    }
}

unsafe fn build_attr_table() -> [sys::esp_gatts_attr_db_t; NUM_ATTRS] {
    // Les buffers statiques ci-dessous doivent survivre à l'appel
    // (Bluedroid garde les pointeurs).
    static SVC_UUID: Mutex<[u8; 16]> = Mutex::new([0; 16]);
    static RX_UUID: Mutex<[u8; 16]> = Mutex::new([0; 16]);
    static TX_UUID: Mutex<[u8; 16]> = Mutex::new([0; 16]);
    static mut PRIMARY_SVC_UUID16: u16 = sys::ESP_GATT_UUID_PRI_SERVICE as u16;
    static mut CHAR_DECL_UUID16: u16 = sys::ESP_GATT_UUID_CHAR_DECLARE as u16;
    static mut CCCD_UUID16: u16 = sys::ESP_GATT_UUID_CHAR_CLIENT_CONFIG as u16;
    static mut PROP_WRITE: u8 = sys::ESP_GATT_CHAR_PROP_BIT_WRITE as u8;
    static mut PROP_NOTIFY: u8 = sys::ESP_GATT_CHAR_PROP_BIT_NOTIFY as u8;
    static mut RX_BUF: [u8; protocol::MAX_MSG_LEN] = [0; protocol::MAX_MSG_LEN];
    static mut TX_BUF: [u8; protocol::MAX_MSG_LEN] = [0; protocol::MAX_MSG_LEN];
    static mut CCCD_BUF: [u8; 2] = [0; 2];

    *SVC_UUID.lock().unwrap() = uuid128(protocol::CONFIG_SERVICE_UUID);
    *RX_UUID.lock().unwrap() = uuid128(protocol::CONFIG_RX_CHAR_UUID);
    *TX_UUID.lock().unwrap() = uuid128(protocol::CONFIG_TX_CHAR_UUID);

    let auto_rsp = sys::esp_attr_control_t { auto_rsp: sys::ESP_GATT_AUTO_RSP as u8 };
    let attr = |uuid_len: u16,
                uuid: *const u8,
                perm: u16,
                max_len: u16,
                len: u16,
                value: *mut u8|
     -> sys::esp_gatts_attr_db_t {
        sys::esp_gatts_attr_db_t {
            attr_control: auto_rsp,
            att_desc: sys::esp_attr_desc_t {
                uuid_length: uuid_len,
                uuid_p: uuid as *mut u8,
                perm,
                max_length: max_len,
                length: len,
                value,
            },
        }
    };

    [
        // Déclaration du service primaire.
        attr(
            2,
            core::ptr::addr_of!(PRIMARY_SVC_UUID16) as *const u8,
            sys::ESP_GATT_PERM_READ as u16,
            16,
            16,
            SVC_UUID.lock().unwrap().as_mut_ptr(),
        ),
        // RX : déclaration + valeur (write).
        attr(
            2,
            core::ptr::addr_of!(CHAR_DECL_UUID16) as *const u8,
            sys::ESP_GATT_PERM_READ as u16,
            1,
            1,
            core::ptr::addr_of_mut!(PROP_WRITE),
        ),
        attr(
            16,
            RX_UUID.lock().unwrap().as_ptr(),
            sys::ESP_GATT_PERM_WRITE as u16,
            protocol::MAX_MSG_LEN as u16,
            0,
            core::ptr::addr_of_mut!(RX_BUF) as *mut u8,
        ),
        // TX : déclaration + valeur (notify) + CCCD.
        attr(
            2,
            core::ptr::addr_of!(CHAR_DECL_UUID16) as *const u8,
            sys::ESP_GATT_PERM_READ as u16,
            1,
            1,
            core::ptr::addr_of_mut!(PROP_NOTIFY),
        ),
        attr(
            16,
            TX_UUID.lock().unwrap().as_ptr(),
            sys::ESP_GATT_PERM_READ as u16,
            protocol::MAX_MSG_LEN as u16,
            0,
            core::ptr::addr_of_mut!(TX_BUF) as *mut u8,
        ),
        attr(
            2,
            core::ptr::addr_of!(CCCD_UUID16) as *const u8,
            (sys::ESP_GATT_PERM_READ | sys::ESP_GATT_PERM_WRITE) as u16,
            2,
            2,
            core::ptr::addr_of_mut!(CCCD_BUF) as *mut u8,
        ),
    ]
}

unsafe extern "C" fn gatts_event_handler(
    event: sys::esp_gatts_cb_event_t,
    gatts_if: sys::esp_gatt_if_t,
    param: *mut sys::esp_ble_gatts_cb_param_t,
) {
    match event {
        sys::esp_gatts_cb_event_t_ESP_GATTS_REG_EVT => {
            if let Some(s) = SHARED.lock().unwrap().as_mut() {
                s.gatts_if = gatts_if;
            }
            let table = build_attr_table();
            let _ = esp!(sys::esp_ble_gatts_create_attr_tab(
                table.as_ptr(),
                gatts_if,
                NUM_ATTRS as u8,
                0,
            ));
            start_advertising();
        }
        sys::esp_gatts_cb_event_t_ESP_GATTS_CREAT_ATTR_TAB_EVT => {
            let p = &(*param).add_attr_tab;
            if p.status == sys::esp_gatt_status_t_ESP_GATT_OK && p.num_handle as usize == NUM_ATTRS
            {
                let handles = core::slice::from_raw_parts(p.handles, NUM_ATTRS);
                if let Some(s) = SHARED.lock().unwrap().as_mut() {
                    s.handles.copy_from_slice(handles);
                }
                let _ = esp!(sys::esp_ble_gatts_start_service(handles[IDX_SVC]));
            } else {
                log::error!("création table GATT échouée: {}", p.status);
            }
        }
        sys::esp_gatts_cb_event_t_ESP_GATTS_CONNECT_EVT => {
            let p = &(*param).connect;
            if let Some(s) = SHARED.lock().unwrap().as_mut() {
                s.conn_id = p.conn_id;
                s.connected = true;
                s.notifications_on = false;
            }
            log::info!("app iPhone connectée (BLE)");
        }
        sys::esp_gatts_cb_event_t_ESP_GATTS_DISCONNECT_EVT => {
            if let Some(s) = SHARED.lock().unwrap().as_mut() {
                s.connected = false;
                s.notifications_on = false;
            }
            log::info!("app iPhone déconnectée");
            start_advertising();
        }
        sys::esp_gatts_cb_event_t_ESP_GATTS_WRITE_EVT => {
            let p = &(*param).write;
            // Fragment d'écriture préparée (message > MTU−3, courant sur
            // iOS) : la pile l'accumule elle-même dans la valeur de
            // l'attribut (ESP_GATT_AUTO_RSP) ; le message complet est
            // traité à l'EXEC_WRITE_EVT.
            if p.is_prep {
                return;
            }
            let data = core::slice::from_raw_parts(p.value, p.len as usize);
            let mut guard = SHARED.lock().unwrap();
            if let Some(s) = guard.as_mut() {
                if p.handle == s.handles[IDX_TX_CCCD] && data.len() >= 2 {
                    s.notifications_on = data[0] & 0x01 != 0;
                } else if p.handle == s.handles[IDX_RX_VAL] {
                    let _ = s.tx.send(data.to_vec());
                }
            }
        }
        sys::esp_gatts_cb_event_t_ESP_GATTS_EXEC_WRITE_EVT => {
            // Fin d'une écriture longue : si elle est validée, relire la
            // valeur assemblée de la caractéristique RX et la traiter comme
            // un message complet.
            let p = &(*param).exec_write;
            if u32::from(p.exec_write_flag) != sys::ESP_GATT_PREP_WRITE_EXEC {
                return;
            }
            let guard = SHARED.lock().unwrap();
            if let Some(s) = guard.as_ref() {
                let mut len: u16 = 0;
                let mut value: *const u8 = core::ptr::null();
                let err = sys::esp_ble_gatts_get_attr_value(
                    s.handles[IDX_RX_VAL],
                    &mut len,
                    &mut value,
                );
                if err == sys::ESP_OK && !value.is_null() && len > 0 {
                    let data = core::slice::from_raw_parts(value, len as usize);
                    let _ = s.tx.send(data.to_vec());
                }
            }
        }
        _ => {}
    }
}

unsafe extern "C" fn gap_event_handler(
    event: sys::esp_gap_ble_cb_event_t,
    _param: *mut sys::esp_ble_gap_cb_param_t,
) {
    if event == sys::esp_gap_ble_cb_event_t_ESP_GAP_BLE_ADV_DATA_SET_COMPLETE_EVT {
        static mut ADV_PARAMS: sys::esp_ble_adv_params_t = sys::esp_ble_adv_params_t {
            adv_int_min: 0x20,
            adv_int_max: 0x40,
            adv_type: sys::esp_ble_adv_type_t_ADV_TYPE_IND,
            own_addr_type: sys::esp_ble_addr_type_t_BLE_ADDR_TYPE_PUBLIC,
            peer_addr: [0; 6],
            peer_addr_type: sys::esp_ble_addr_type_t_BLE_ADDR_TYPE_PUBLIC,
            channel_map: sys::esp_ble_adv_channel_t_ADV_CHNL_ALL,
            adv_filter_policy:
                sys::esp_ble_adv_filter_t_ADV_FILTER_ALLOW_SCAN_ANY_CON_ANY,
        };
        let _ = esp!(sys::esp_ble_gap_start_advertising(core::ptr::addr_of_mut!(ADV_PARAMS)));
    }
}

fn start_advertising() {
    // Advertise le nom + l'UUID du service pour que l'app filtre le scan.
    static DEVICE_NAME: &core::ffi::CStr = c"Nexus One Config";
    unsafe {
        let _ = esp!(sys::esp_ble_gap_set_device_name(DEVICE_NAME.as_ptr()));
        static mut SVC_UUID: [u8; 16] = [0; 16];
        SVC_UUID = uuid128(protocol::CONFIG_SERVICE_UUID);
        static mut ADV_DATA: sys::esp_ble_adv_data_t = sys::esp_ble_adv_data_t {
            set_scan_rsp: false,
            include_name: true,
            include_txpower: false,
            min_interval: 0x0006,
            max_interval: 0x0010,
            appearance: 0x03C4, // gamepad
            manufacturer_len: 0,
            p_manufacturer_data: core::ptr::null_mut(),
            service_data_len: 0,
            p_service_data: core::ptr::null_mut(),
            service_uuid_len: 16,
            p_service_uuid: core::ptr::null_mut(),
            flag: (sys::ESP_BLE_ADV_FLAG_GEN_DISC | sys::ESP_BLE_ADV_FLAG_BREDR_NOT_SPT) as u8,
        };
        ADV_DATA.p_service_uuid = core::ptr::addr_of_mut!(SVC_UUID) as *mut u8;
        let _ = esp!(sys::esp_ble_gap_config_adv_data(core::ptr::addr_of_mut!(ADV_DATA)));
    }
}
