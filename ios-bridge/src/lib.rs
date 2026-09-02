//! Pont C entre `controller-core` et l'application iOS.
//!
//! Swift ne réimplémente pas l'encodage `postcard` du protocole : il passe
//! par ce pont, qui réutilise exactement le code embarqué dans le firmware.
//! L'interchange côté Swift est du JSON, bien plus simple à manipuler avec
//! `Codable` — la forme des messages sur le fil, elle, reste définie ici.
//!
//! Convention de toutes les fonctions : elles écrivent dans `out` (capacité
//! `out_cap`) et renvoient le nombre d'octets écrits, ou une valeur négative
//! en cas d'erreur (voir `NexusStatus`). `nexus_last_error` détaille alors.

use controller_core::config::Config;
use controller_core::protocol::{self, Request, Response};
use controller_core::stats::Stats;
use serde_json::json;
use std::cell::RefCell;

/// Codes d'erreur renvoyés par les fonctions du pont.
pub const NEXUS_ERR_INVALID_INPUT: isize = -1;
pub const NEXUS_ERR_ENCODE: isize = -2;
pub const NEXUS_ERR_DECODE: isize = -3;
/// Le tampon fourni est trop petit ; rappeler avec au moins `out_cap` octets.
pub const NEXUS_ERR_BUFFER_TOO_SMALL: isize = -4;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_error(msg: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = msg.into());
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

unsafe fn encode_request(req: &Request, out: *mut u8, out_cap: usize) -> isize {
    let mut buf = [0u8; protocol::MAX_MSG_LEN];
    match protocol::encode(req, &mut buf) {
        Ok(n) => write_out(&buf[..n], out, out_cap),
        Err(e) => {
            set_error(format!("encodage postcard : {e:?}"));
            NEXUS_ERR_ENCODE
        }
    }
}

// ---------------------------------------------------------------- erreurs

/// Copie le dernier message d'erreur (UTF-8, sans zéro terminal).
///
/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_last_error(out: *mut u8, out_cap: usize) -> isize {
    let msg = LAST_ERROR.with(|e| e.borrow().clone());
    write_out(msg.as_bytes(), out, out_cap)
}

/// Taille maximale d'un message du protocole, pour dimensionner les tampons.
#[no_mangle]
pub extern "C" fn nexus_max_message_len() -> usize {
    protocol::MAX_MSG_LEN
}

// ------------------------------------------------------------------ UUID

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

// --------------------------------------------------------------- requêtes

/// Requêtes sans paramètre, désignées par un code (voir `NexusSimpleRequest`
/// dans l'en-tête C).
///
/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_encode_simple_request(
    kind: u32,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    let req = match kind {
        0 => Request::GetInfo,
        1 => Request::GetConfig,
        2 => Request::SaveConfig,
        3 => Request::FactoryReset,
        4 => Request::GetStats,
        5 => Request::ResetStats,
        6 => Request::Identify,
        7 => Request::GetBattery,
        _ => {
            set_error(format!("code de requête inconnu : {kind}"));
            return NEXUS_ERR_INVALID_INPUT;
        }
    };
    encode_request(&req, out, out_cap)
}

/// Encode `SetConfig` à partir d'une configuration au format JSON.
///
/// # Safety
/// `json` doit pointer sur `json_len` octets lisibles, `out` sur `out_cap`
/// octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_encode_set_config(
    json: *const u8,
    json_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    let Some(bytes) = read_in(json, json_len) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let cfg: Config = match serde_json::from_slice(bytes) {
        Ok(c) => c,
        Err(e) => {
            set_error(format!("configuration JSON invalide : {e}"));
            return NEXUS_ERR_DECODE;
        }
    };
    encode_request(&Request::SetConfig(cfg), out, out_cap)
}

/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_encode_test_haptic(
    effect: u8,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    encode_request(&Request::TestHaptic(effect), out, out_cap)
}

/// Encode `StartOta`. Les trois chaînes sont en UTF-8, sans zéro terminal.
///
/// # Safety
/// Chaque pointeur doit être valide pour la longueur associée.
#[no_mangle]
pub unsafe extern "C" fn nexus_encode_start_ota(
    ssid: *const u8,
    ssid_len: usize,
    password: *const u8,
    password_len: usize,
    url: *const u8,
    url_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    let (Some(s), Some(p), Some(u)) = (
        read_in(ssid, ssid_len),
        read_in(password, password_len),
        read_in(url, url_len),
    ) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let to_str = |b: &[u8]| std::str::from_utf8(b).map(|s| s.to_owned());
    let (Ok(s), Ok(p), Ok(u)) = (to_str(s), to_str(p), to_str(u)) else {
        set_error("champ OTA non UTF-8");
        return NEXUS_ERR_INVALID_INPUT;
    };
    let req = Request::StartOta {
        ssid: match heapless::String::try_from(s.as_str()) {
            Ok(v) => v,
            Err(_) => {
                set_error("SSID trop long (32 caractères maximum)");
                return NEXUS_ERR_INVALID_INPUT;
            }
        },
        password: match heapless::String::try_from(p.as_str()) {
            Ok(v) => v,
            Err(_) => {
                set_error("mot de passe trop long (64 caractères maximum)");
                return NEXUS_ERR_INVALID_INPUT;
            }
        },
        url: match heapless::String::try_from(u.as_str()) {
            Ok(v) => v,
            Err(_) => {
                set_error("URL trop longue (128 caractères maximum)");
                return NEXUS_ERR_INVALID_INPUT;
            }
        },
    };
    encode_request(&req, out, out_cap)
}

// -------------------------------------------------------------- réponses

/// Décode une réponse reçue en BLE et la rend en JSON, sous la forme
/// `{"kind": "...", ...}` — la discrimination est explicite pour que Swift
/// n'ait pas à connaître la représentation des énumérations de serde.
///
/// # Safety
/// `data` doit pointer sur `data_len` octets lisibles, `out` sur `out_cap`
/// octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_decode_response(
    data: *const u8,
    data_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    let Some(bytes) = read_in(data, data_len) else {
        return NEXUS_ERR_INVALID_INPUT;
    };
    let resp = match protocol::decode_response(bytes) {
        Ok(r) => r,
        Err(e) => {
            set_error(format!("réponse illisible : {e:?}"));
            return NEXUS_ERR_DECODE;
        }
    };
    let value = match resp {
        Response::Info {
            protocol_version,
            firmware_version,
            name,
        } => json!({
            "kind": "info",
            "protocolVersion": protocol_version,
            "firmwareVersion": firmware_version.as_str(),
            "name": name.as_str(),
        }),
        Response::Config(cfg) => json!({ "kind": "config", "config": cfg }),
        Response::Stats(s) => json!({ "kind": "stats", "stats": s }),
        Response::Battery {
            millivolts,
            percent,
            charging,
        } => json!({
            "kind": "battery",
            "millivolts": millivolts,
            "percent": percent,
            "charging": charging,
        }),
        Response::OtaProgress(p) => json!({ "kind": "otaProgress", "percent": p }),
        Response::Ok => json!({ "kind": "ok" }),
        Response::Err(code) => json!({ "kind": "error", "code": format!("{code:?}") }),
    };
    match serde_json::to_vec(&value) {
        Ok(v) => write_out(&v, out, out_cap),
        Err(e) => {
            set_error(format!("sérialisation JSON : {e}"));
            NEXUS_ERR_ENCODE
        }
    }
}

/// Configuration d'usine, en JSON — sert de gabarit à l'application.
///
/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_default_config_json(out: *mut u8, out_cap: usize) -> isize {
    match serde_json::to_vec(&Config::default()) {
        Ok(v) => write_out(&v, out, out_cap),
        Err(e) => {
            set_error(format!("sérialisation JSON : {e}"));
            NEXUS_ERR_ENCODE
        }
    }
}

/// Statistiques vides, en JSON.
///
/// # Safety
/// `out` doit pointer sur au moins `out_cap` octets inscriptibles.
#[no_mangle]
pub unsafe extern "C" fn nexus_empty_stats_json(out: *mut u8, out_cap: usize) -> isize {
    match serde_json::to_vec(&Stats::default()) {
        Ok(v) => write_out(&v, out, out_cap),
        Err(e) => {
            set_error(format!("sérialisation JSON : {e}"));
            NEXUS_ERR_ENCODE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un aller-retour complet : JSON → postcard → JSON doit préserver la
    /// configuration. C'est le contrat sur lequel repose l'app Swift.
    #[test]
    fn config_survives_json_postcard_roundtrip() {
        let mut json_buf = vec![0u8; 8192];
        let n = unsafe { nexus_default_config_json(json_buf.as_mut_ptr(), json_buf.len()) };
        assert!(n > 0, "sérialisation de la config par défaut");

        let mut wire = vec![0u8; protocol::MAX_MSG_LEN];
        let w = unsafe {
            nexus_encode_set_config(json_buf.as_ptr(), n as usize, wire.as_mut_ptr(), wire.len())
        };
        assert!(w > 0, "encodage SetConfig : {}", last_error());

        let decoded = protocol::decode_request(&wire[..w as usize]).unwrap();
        match decoded {
            Request::SetConfig(cfg) => assert_eq!(cfg, Config::default()),
            other => panic!("requête inattendue : {other:?}"),
        }
    }

    #[test]
    fn response_decodes_to_tagged_json() {
        let resp = Response::Battery {
            millivolts: 3900,
            percent: 76,
            charging: true,
        };
        let mut wire = vec![0u8; protocol::MAX_MSG_LEN];
        let n = protocol::encode(&resp, &mut wire).unwrap();

        let mut out = vec![0u8; 4096];
        let m = unsafe { nexus_decode_response(wire.as_ptr(), n, out.as_mut_ptr(), out.len()) };
        assert!(m > 0);
        let v: serde_json::Value = serde_json::from_slice(&out[..m as usize]).unwrap();
        assert_eq!(v["kind"], "battery");
        assert_eq!(v["millivolts"], 3900);
        assert_eq!(v["charging"], true);
    }

    #[test]
    fn simple_requests_encode() {
        let mut out = vec![0u8; protocol::MAX_MSG_LEN];
        for kind in 0..8u32 {
            let n = unsafe { nexus_encode_simple_request(kind, out.as_mut_ptr(), out.len()) };
            assert!(n > 0, "requête {kind} : {}", last_error());
            protocol::decode_request(&out[..n as usize]).expect("relecture");
        }
        let bad = unsafe { nexus_encode_simple_request(99, out.as_mut_ptr(), out.len()) };
        assert_eq!(bad, NEXUS_ERR_INVALID_INPUT);
    }

    #[test]
    fn buffer_too_small_is_reported() {
        let mut tiny = [0u8; 2];
        let r = unsafe { nexus_default_config_json(tiny.as_mut_ptr(), tiny.len()) };
        assert_eq!(r, NEXUS_ERR_BUFFER_TOO_SMALL);
        assert!(last_error().contains("trop petit"));
    }

    #[test]
    fn uuids_match_the_protocol() {
        let mut out = [0u8; 64];
        let n = unsafe { nexus_service_uuid(out.as_mut_ptr(), out.len()) };
        assert_eq!(
            std::str::from_utf8(&out[..n as usize]).unwrap(),
            protocol::CONFIG_SERVICE_UUID
        );
    }

    fn last_error() -> String {
        let mut buf = [0u8; 512];
        let n = unsafe { nexus_last_error(buf.as_mut_ptr(), buf.len()) };
        String::from_utf8_lossy(&buf[..n.max(0) as usize]).into_owned()
    }
}
