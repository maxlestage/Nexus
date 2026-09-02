//! Frontière C exposée à Swift.
//!
//! Convention : les fonctions qui produisent des octets écrivent dans `out`
//! (capacité `out_cap`) et renvoient le nombre d'octets écrits, ou une valeur
//! négative en cas d'erreur. `nexus_last_error` détaille alors.

use crate::state::{AppState, BleEvent};
use controller_core::protocol;
use std::cell::RefCell;

pub const NEXUS_ERR_INVALID_INPUT: isize = -1;
pub const NEXUS_ERR_ENCODE: isize = -2;
pub const NEXUS_ERR_BUFFER_TOO_SMALL: isize = -4;
/// Aucune trame n'est en attente d'émission.
pub const NEXUS_NOTHING_TO_SEND: isize = 0;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_error(message: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = message.into());
}

/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
unsafe fn write_out(bytes: &[u8], out: *mut u8, out_cap: usize) -> isize {
    if out.is_null() {
        set_error("tampon de sortie nul");
        return NEXUS_ERR_INVALID_INPUT;
    }
    if bytes.len() > out_cap {
        set_error(format!(
            "tampon trop petit : {} octets nécessaires, {} fournis",
            bytes.len(),
            out_cap
        ));
        return NEXUS_ERR_BUFFER_TOO_SMALL;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
    bytes.len() as isize
}

/// # Safety
/// `ptr` doit pointer sur `len` octets lisibles.
unsafe fn read_in<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        set_error("tampon d'entrée nul");
        return None;
    }
    Some(std::slice::from_raw_parts(ptr, len))
}

/// # Safety
/// `state` doit venir de `nexus_app_new` et ne pas encore avoir été libéré.
unsafe fn app<'a>(state: *mut AppState) -> Option<&'a mut AppState> {
    if state.is_null() {
        set_error("état nul");
        return None;
    }
    Some(&mut *state)
}

// --------------------------------------------------------- cycle de vie

/// Crée l'état de l'application. À libérer avec `nexus_app_free`.
#[no_mangle]
pub extern "C" fn nexus_app_new() -> *mut AppState {
    Box::into_raw(Box::new(AppState::new()))
}

/// # Safety
/// `state` doit venir de `nexus_app_new` et n'être libéré qu'une fois.
#[no_mangle]
pub unsafe extern "C" fn nexus_app_free(state: *mut AppState) {
    if !state.is_null() {
        drop(Box::from_raw(state));
    }
}

/// Copie le dernier message d'erreur du pont (UTF-8, sans zéro terminal).
///
/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_last_error(out: *mut u8, out_cap: usize) -> isize {
    let message = LAST_ERROR.with(|e| e.borrow().clone());
    write_out(message.as_bytes(), out, out_cap)
}

// ------------------------------------------------------------ interface

/// Modèle de vue courant, en JSON. C'est la seule source de l'affichage.
///
/// # Safety
/// `state` doit être valide, `out` inscriptible sur `out_cap` octets.
#[no_mangle]
pub unsafe extern "C" fn nexus_app_view(
    state: *mut AppState,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    let Some(app) = app(state) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    match serde_json::to_vec(&app.view()) {
        Ok(json) => write_out(&json, out, out_cap),
        Err(e) => {
            set_error(format!("sérialisation du modèle de vue : {e}"));
            NEXUS_ERR_ENCODE
        }
    }
}

/// Traite une action de l'interface. `id` désigne la commande, `value` est
/// sa nouvelle valeur en JSON (`null` pour un simple bouton).
///
/// # Safety
/// Les pointeurs doivent être valides pour les longueurs indiquées.
#[no_mangle]
pub unsafe extern "C" fn nexus_app_dispatch(
    state: *mut AppState,
    id: *const u8,
    id_len: usize,
    value_json: *const u8,
    value_len: usize,
) -> isize {
    let Some(app) = app(state) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let (Some(id_bytes), Some(value_bytes)) = (read_in(id, id_len), read_in(value_json, value_len))
    else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let Ok(id) = std::str::from_utf8(id_bytes) else {
        set_error("identifiant d'action non UTF-8");
        return NEXUS_ERR_INVALID_INPUT;
    };
    // Une valeur absente vaut `null` : c'est le cas de tous les boutons.
    let value = if value_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_slice(value_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(format!("valeur d'action illisible : {e}"));
                return NEXUS_ERR_INVALID_INPUT;
            }
        }
    };
    app.dispatch(id, &value);
    0
}

// ------------------------------------------------------------ Bluetooth

/// Signale un changement d'état de la liaison (codes de `BleEvent`).
///
/// # Safety
/// `state` doit être valide.
#[no_mangle]
pub unsafe extern "C" fn nexus_app_ble_event(state: *mut AppState, code: u32) -> isize {
    let Some(app) = app(state) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let Some(event) = BleEvent::from_code(code) else {
        set_error(format!("événement Bluetooth inconnu : {code}"));
        return NEXUS_ERR_INVALID_INPUT;
    };
    app.on_ble_event(event);
    0
}

/// Réinjecte une notification reçue de la manette.
///
/// # Safety
/// `state` doit être valide, `data` lisible sur `data_len` octets.
#[no_mangle]
pub unsafe extern "C" fn nexus_app_ble_data(
    state: *mut AppState,
    data: *const u8,
    data_len: usize,
) -> isize {
    let Some(app) = app(state) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let Some(bytes) = read_in(data, data_len) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    app.on_ble_data(bytes);
    0
}

/// Signale un échec de transport (écriture refusée, délai dépassé…).
///
/// # Safety
/// `state` doit être valide, `message` lisible sur `message_len` octets.
#[no_mangle]
pub unsafe extern "C" fn nexus_app_ble_error(
    state: *mut AppState,
    message: *const u8,
    message_len: usize,
) -> isize {
    let Some(app) = app(state) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let text = read_in(message, message_len)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("Erreur de communication avec la manette.");
    app.on_ble_error(text);
    0
}

/// Récupère la prochaine trame à émettre. Renvoie 0 s'il n'y a rien à faire.
///
/// # Safety
/// `state` doit être valide, `out` inscriptible sur `out_cap` octets.
#[no_mangle]
pub unsafe extern "C" fn nexus_app_take_outgoing(
    state: *mut AppState,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    let Some(app) = app(state) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    match app.take_outgoing() {
        Some(payload) => write_out(&payload, out, out_cap),
        None => NEXUS_NOTHING_TO_SEND,
    }
}

// ----------------------------------------------------------- constantes

/// Taille maximale d'un message du protocole, pour dimensionner les tampons.
#[no_mangle]
pub extern "C" fn nexus_max_message_len() -> usize {
    protocol::MAX_MSG_LEN
}

/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_service_uuid(out: *mut u8, out_cap: usize) -> isize {
    write_out(protocol::CONFIG_SERVICE_UUID.as_bytes(), out, out_cap)
}

/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_rx_char_uuid(out: *mut u8, out_cap: usize) -> isize {
    write_out(protocol::CONFIG_RX_CHAR_UUID.as_bytes(), out, out_cap)
}

/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_tx_char_uuid(out: *mut u8, out_cap: usize) -> isize {
    write_out(protocol::CONFIG_TX_CHAR_UUID.as_bytes(), out, out_cap)
}
