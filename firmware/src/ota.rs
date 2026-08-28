//! Mise à jour OTA : l'app iPhone envoie SSID + mot de passe + URL du
//! firmware via BLE ; la manette se connecte au WiFi, télécharge l'image
//! dans la partition OTA inactive, puis redémarre dessus.
//!
//! La progression est renvoyée à l'app (`Response::OtaProgress`) et
//! affichée sur le bandeau LED.

use embedded_svc::http::client::Client;
use embedded_svc::io::Read;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::ota::EspOta;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};

pub struct OtaRequest {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
    pub url: heapless::String<128>,
}

/// Exécute la mise à jour complète. Ne retourne `Ok` qu'après avoir marqué
/// la nouvelle partition comme démarrable ; l'appelant redémarre ensuite.
pub fn run(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    req: &OtaRequest,
    mut progress: impl FnMut(u8),
) -> anyhow::Result<()> {
    // 1. WiFi.
    progress(0);
    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sysloop.clone(), Some(nvs))?, sysloop)?;
    let auth = if req.password.is_empty() { AuthMethod::None } else { AuthMethod::WPA2Personal };
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: req.ssid.clone(),
        password: req.password.clone(),
        auth_method: auth,
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    progress(5);

    // 2. Téléchargement en flux vers la partition OTA inactive.
    let mut client = Client::wrap(EspHttpConnection::new(&HttpConfig {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })?);
    let request = client.get(req.url.as_str())?;
    let response = request.submit()?;
    if response.status() != 200 {
        anyhow::bail!("HTTP {}", response.status());
    }
    let total: u64 = response
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut ota = EspOta::new()?;
    let mut update = ota.initiate_update()?;
    let mut body = response;
    let mut buf = [0u8; 4096];
    let mut written: u64 = 0;
    loop {
        let n = body.read(&mut buf)?;
        if n == 0 {
            break;
        }
        update.write(&buf[..n])?;
        written += n as u64;
        if total > 0 {
            progress((5 + written * 90 / total.max(1)) as u8);
        }
    }
    if written == 0 {
        update.abort()?;
        anyhow::bail!("image vide");
    }

    // 3. Validation + bascule de partition de boot.
    update.complete()?;
    progress(100);
    log::info!("OTA terminée : {written} octets écrits, redémarrage");
    Ok(())
}
