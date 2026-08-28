//! Pile Bluetooth (Bluedroid en mode dual) :
//!
//! - `procon` : périphérique HID Bluetooth **classique** (BR/EDR) qui se
//!   présente comme un Pro Controller — c'est ce que la Switch attend.
//! - `pc_hid` : gamepad HID **BLE** générique pour Windows/macOS.
//! - `ble_config` : service GATT BLE de configuration pour l'app iPhone
//!   (actif dans les deux modes).
//!
//! Le mode (Switch ou PC) est choisi au démarrage : maintenir la gâchette
//! `MiddleLower` pendant la mise sous tension bascule en mode PC.

pub mod ble_config;
pub mod pc_hid;
pub mod procon;

use esp_idf_sys as sys;
use esp_idf_sys::esp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtMode {
    /// Bluetooth classique, émulation Pro Controller.
    Switch,
    /// BLE HID gamepad générique (Windows / macOS).
    Pc,
}

/// Initialise le contrôleur BT en mode dual (BR/EDR + BLE) et Bluedroid.
pub fn init_stack() -> anyhow::Result<()> {
    unsafe {
        let mut cfg: sys::esp_bt_controller_config_t = bt_controller_default_config();
        esp!(sys::esp_bt_controller_init(&mut cfg))?;
        esp!(sys::esp_bt_controller_enable(sys::esp_bt_mode_t_ESP_BT_MODE_BTDM))?;
        esp!(sys::esp_bluedroid_init())?;
        esp!(sys::esp_bluedroid_enable())?;
    }
    Ok(())
}

/// Adresse MAC Bluetooth locale (sert d'identité Pro Controller).
pub fn local_mac() -> [u8; 6] {
    let mut mac = [0u8; 6];
    unsafe {
        let p = sys::esp_bt_dev_get_address();
        if !p.is_null() {
            core::ptr::copy_nonoverlapping(p, mac.as_mut_ptr(), 6);
        }
    }
    mac
}

/// Équivalent du macro C `BT_CONTROLLER_INIT_CONFIG_DEFAULT()`.
fn bt_controller_default_config() -> sys::esp_bt_controller_config_t {
    // esp-idf-sys expose la config par défaut via bindgen ; les champs
    // essentiels sont recopiés du SDK (ESP-IDF v5.2, cible ESP32).
    sys::esp_bt_controller_config_t {
        controller_task_stack_size: sys::ESP_TASK_BT_CONTROLLER_STACK as _,
        controller_task_prio: sys::ESP_TASK_BT_CONTROLLER_PRIO as _,
        hci_uart_no: sys::BT_HCI_UART_NO_DEFAULT as _,
        hci_uart_baudrate: sys::BT_HCI_UART_BAUDRATE_DEFAULT,
        scan_duplicate_mode: sys::SCAN_DUPLICATE_MODE as _,
        scan_duplicate_type: sys::SCAN_DUPLICATE_TYPE_VALUE as _,
        normal_adv_size: sys::NORMAL_SCAN_DUPLICATE_CACHE_SIZE as _,
        mesh_adv_size: sys::MESH_DUPLICATE_SCAN_CACHE_SIZE as _,
        send_adv_reserved_size: sys::SCAN_SEND_ADV_RESERVED_SIZE as _,
        controller_debug_flag: sys::CONTROLLER_ADV_LOST_DEBUG_BIT,
        mode: sys::esp_bt_mode_t_ESP_BT_MODE_BTDM as _,
        ble_max_conn: sys::CONFIG_BTDM_CTRL_BLE_MAX_CONN_EFF as _,
        bt_max_acl_conn: sys::CONFIG_BTDM_CTRL_BR_EDR_MAX_ACL_CONN_EFF as _,
        bt_sco_datapath: sys::CONFIG_BTDM_CTRL_BR_EDR_SCO_DATA_PATH_EFF as _,
        auto_latency: false,
        bt_legacy_auth_vs_evt: false,
        bt_max_sync_conn: sys::CONFIG_BTDM_CTRL_BR_EDR_MAX_SYNC_CONN_EFF as _,
        ble_sca: sys::CONFIG_BTDM_BLE_SLEEP_CLOCK_ACCURACY_INDEX_EFF as _,
        pcm_role: 0,
        pcm_polar: 0,
        hli: false,
        dup_list_refresh_period: sys::DUPL_SCAN_CACHE_REFRESH_PERIOD as _,
        ble_scan_backoff: false,
        magic: sys::ESP_BT_CONTROLLER_CONFIG_MAGIC_VAL,
    }
}
