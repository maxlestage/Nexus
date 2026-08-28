//! Protocole de configuration entre l'application iPhone et la manette,
//! transporté sur un service BLE GATT (une caractéristique en écriture
//! pour les requêtes, une en notification pour les réponses).
//!
//! Encodage : `postcard` précédé d'aucun en-tête — chaque écriture GATT
//! contient exactement un message (MTU négocié à 512 octets).

use crate::config::Config;
use crate::stats::Stats;
use heapless::String;
use serde::{Deserialize, Serialize};

/// UUIDs du service de configuration (générés pour ce projet).
pub const CONFIG_SERVICE_UUID: &str = "6e400001-c352-11ee-8a5b-325096b39f47";
pub const CONFIG_RX_CHAR_UUID: &str = "6e400002-c352-11ee-8a5b-325096b39f47";
pub const CONFIG_TX_CHAR_UUID: &str = "6e400003-c352-11ee-8a5b-325096b39f47";

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_MSG_LEN: usize = 512;

// `SetConfig` embarque la config entière : pas de boîte possible en
// `no_std` sans alloc, et le message tient largement dans le MTU.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Request {
    /// Identité + versions (première requête de l'app).
    GetInfo,
    GetConfig,
    SetConfig(Config),
    /// Écrit la config courante en NVS.
    SaveConfig,
    /// Restaure la configuration d'usine.
    FactoryReset,
    GetStats,
    ResetStats,
    /// Fait vibrer la manette (test depuis l'app), effet DRV2605 0..=123.
    TestHaptic(u8),
    /// Flash LED pour identifier la manette.
    Identify,
    /// Niveau batterie demandé par l'app.
    GetBattery,
    /// Lance une mise à jour OTA : la manette se connecte au WiFi donné
    /// et télécharge le firmware à l'URL indiquée (HTTPS).
    StartOta {
        ssid: String<32>,
        password: String<64>,
        url: String<128>,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Info {
        protocol_version: u8,
        firmware_version: String<16>,
        name: String<24>,
    },
    Config(Config),
    Stats(Stats),
    Battery {
        /// Tension en millivolts.
        millivolts: u16,
        /// Estimation 0..=100.
        percent: u8,
        charging: bool,
    },
    /// Progression OTA en pourcent, envoyée spontanément pendant la MAJ.
    OtaProgress(u8),
    Ok,
    Err(ErrorCode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    BadRequest,
    ConfigInvalid,
    StorageFull,
    WifiFailed,
    OtaFailed,
    Busy,
}

pub fn encode<T: Serialize>(msg: &T, buf: &mut [u8]) -> Result<usize, postcard::Error> {
    let used = postcard::to_slice(msg, buf)?;
    Ok(used.len())
}

pub fn decode_request(bytes: &[u8]) -> Result<Request, postcard::Error> {
    postcard::from_bytes(bytes)
}

pub fn decode_response(bytes: &[u8]) -> Result<Response, postcard::Error> {
    postcard::from_bytes(bytes)
}
