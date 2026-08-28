//! Client BLE (CoreBluetooth via btleplug) : découverte de la manette,
//! écriture des requêtes et réception des réponses du protocole
//! `controller_core::protocol`.

use anyhow::{anyhow, Context, Result};
use btleplug::api::{
    Central, CharPropFlags, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use controller_core::protocol::{
    self, Request, Response, CONFIG_RX_CHAR_UUID, CONFIG_SERVICE_UUID, CONFIG_TX_CHAR_UUID,
};
use futures::StreamExt;
use std::time::Duration;
use uuid::Uuid;

pub struct BleClient {
    peripheral: Peripheral,
    rx_char: Characteristic,
    /// Réponses spontanées (progression OTA) en plus des réponses directes.
    pub events: tokio::sync::mpsc::UnboundedReceiver<Response>,
    responses: tokio::sync::mpsc::UnboundedReceiver<Response>,
}

impl BleClient {
    /// Scanne puis se connecte à la première manette « Nexus One » trouvée.
    pub async fn connect() -> Result<Self> {
        let service_uuid = Uuid::parse_str(CONFIG_SERVICE_UUID)?;
        let rx_uuid = Uuid::parse_str(CONFIG_RX_CHAR_UUID)?;
        let tx_uuid = Uuid::parse_str(CONFIG_TX_CHAR_UUID)?;

        let manager = Manager::new().await?;
        let adapter: Adapter = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .context("pas d'adaptateur Bluetooth")?;

        adapter
            .start_scan(ScanFilter { services: vec![service_uuid] })
            .await?;

        // Jusqu'à 10 s pour trouver la manette.
        let mut found: Option<Peripheral> = None;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            for p in adapter.peripherals().await? {
                if let Ok(Some(props)) = p.properties().await {
                    if props.services.contains(&service_uuid)
                        || props.local_name.as_deref() == Some("Nexus One Config")
                    {
                        found = Some(p);
                        break;
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
        adapter.stop_scan().await.ok();
        let peripheral = found.context("manette introuvable — est-elle allumée ?")?;

        peripheral.connect().await?;
        peripheral.discover_services().await?;

        let chars = peripheral.characteristics();
        let rx_char = chars
            .iter()
            .find(|c| c.uuid == rx_uuid)
            .context("caractéristique RX absente")?
            .clone();
        let tx_char = chars
            .iter()
            .find(|c| c.uuid == tx_uuid && c.properties.contains(CharPropFlags::NOTIFY))
            .context("caractéristique TX absente")?
            .clone();

        peripheral.subscribe(&tx_char).await?;

        // Tâche de fond : démultiplexe notifications → réponses / événements.
        let (resp_tx, responses) = tokio::sync::mpsc::unbounded_channel();
        let (event_tx, events) = tokio::sync::mpsc::unbounded_channel();
        let mut notifications = peripheral.notifications().await?;
        tokio::spawn(async move {
            while let Some(n) = notifications.next().await {
                if let Ok(resp) = protocol::decode_response(&n.value) {
                    match resp {
                        Response::OtaProgress(_) => {
                            let _ = event_tx.send(resp);
                        }
                        other => {
                            let _ = resp_tx.send(other);
                        }
                    }
                }
            }
        });

        Ok(Self { peripheral, rx_char, events, responses })
    }

    /// Envoie une requête et attend la réponse (2 s de timeout).
    pub async fn request(&mut self, req: &Request) -> Result<Response> {
        // Purge les réponses arrivées après un timeout précédent : sans
        // cela, la réponse tardive d'une vieille requête serait prise pour
        // celle de la nouvelle et décalerait la file en permanence.
        while self.responses.try_recv().is_ok() {}

        let mut buf = [0u8; protocol::MAX_MSG_LEN];
        let n = protocol::encode(req, &mut buf).map_err(|e| anyhow!("encode: {e:?}"))?;
        self.peripheral
            .write(&self.rx_char, &buf[..n], WriteType::WithResponse)
            .await?;
        tokio::time::timeout(Duration::from_secs(2), self.responses.recv())
            .await
            .context("délai dépassé")?
            .context("connexion fermée")
    }

    /// La liaison est-elle encore établie ? (pour détecter une manette
    /// éteinte ou un redémarrage post-OTA)
    pub async fn is_connected(&self) -> bool {
        self.peripheral.is_connected().await.unwrap_or(false)
    }

    pub async fn disconnect(&self) {
        let _ = self.peripheral.disconnect().await;
    }
}
