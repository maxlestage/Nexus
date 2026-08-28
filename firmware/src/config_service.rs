//! Traitement des requêtes de l'application iPhone (reçues via le service
//! BLE) : remapping, turbo, macros, LEDs, haptique, stats, batterie, OTA.

use crate::bt::ble_config::BleConfigService;
use crate::haptics::{effects, Haptics};
use crate::leds::Leds;
use crate::ota::OtaRequest;
use crate::storage::Storage;
use controller_core::engine::Engine;
use controller_core::protocol::{self, ErrorCode, Request, Response};

pub struct ConfigService {
    ble: BleConfigService,
    firmware_version: &'static str,
    pending_ota: Option<OtaRequest>,
}

impl ConfigService {
    pub fn new(ble: BleConfigService, firmware_version: &'static str) -> Self {
        Self { ble, firmware_version, pending_ota: None }
    }

    /// Une OTA a-t-elle été demandée ? (consommée par `main`).
    pub fn take_ota_request(&mut self) -> Option<OtaRequest> {
        self.pending_ota.take()
    }

    pub fn notify_ota_progress(pct: u8) {
        let mut buf = [0u8; 8];
        if let Ok(n) = protocol::encode(&Response::OtaProgress(pct), &mut buf) {
            let _ = BleConfigService::send(&buf[..n]);
        }
    }

    /// Dépile et traite toutes les requêtes en attente.
    pub fn poll(
        &mut self,
        engine: &mut Engine,
        store: &mut Storage,
        haptics: &mut Haptics,
        leds: &mut Leds,
        battery_mv: u16,
        battery_percent: u8,
    ) -> anyhow::Result<()> {
        while let Ok(raw) = self.ble.rx.try_recv() {
            let response = match protocol::decode_request(&raw) {
                Err(_) => Response::Err(ErrorCode::BadRequest),
                Ok(req) => self.handle(req, engine, store, haptics, leds, battery_mv, battery_percent),
            };
            let mut buf = [0u8; protocol::MAX_MSG_LEN];
            if let Ok(n) = protocol::encode(&response, &mut buf) {
                let _ = BleConfigService::send(&buf[..n]);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle(
        &mut self,
        req: Request,
        engine: &mut Engine,
        store: &mut Storage,
        haptics: &mut Haptics,
        leds: &mut Leds,
        battery_mv: u16,
        battery_percent: u8,
    ) -> Response {
        match req {
            Request::GetInfo => Response::Info {
                protocol_version: protocol::PROTOCOL_VERSION,
                firmware_version: heapless::String::try_from(self.firmware_version)
                    .unwrap_or_default(),
                name: heapless::String::try_from("Nexus One").unwrap_or_default(),
            },
            Request::GetConfig => Response::Config(engine.config_for_save()),
            Request::SetConfig(cfg) => {
                haptics.set_enabled(cfg.haptics.enabled);
                haptics.set_strength(cfg.haptics.strength);
                leds.set_config(cfg.leds);
                engine.set_config(cfg);
                Response::Ok
            }
            Request::SaveConfig => match store.save_config(&engine.config_for_save()) {
                Ok(()) => Response::Ok,
                Err(_) => Response::Err(ErrorCode::StorageFull),
            },
            Request::FactoryReset => {
                let _ = store.factory_reset();
                let cfg = controller_core::config::Config::default();
                haptics.set_enabled(cfg.haptics.enabled);
                haptics.set_strength(cfg.haptics.strength);
                leds.set_config(cfg.leds);
                engine.set_config(cfg);
                Response::Ok
            }
            Request::GetStats => Response::Stats(engine.stats().clone()),
            Request::ResetStats => {
                engine.stats_mut().reset();
                Response::Ok
            }
            Request::TestHaptic(effect) => {
                let _ = haptics.play_effect(effect.clamp(1, 123));
                Response::Ok
            }
            Request::Identify => {
                let _ = haptics.play_effect(effects::RAMP_UP);
                leds.on_press();
                Response::Ok
            }
            Request::GetBattery => Response::Battery {
                millivolts: battery_mv,
                percent: battery_percent,
                // CHRG du TP4056 non câblée par défaut : déduit de la tension.
                charging: battery_mv > 4250,
            },
            Request::StartOta { ssid, password, url } => {
                if self.pending_ota.is_some() {
                    Response::Err(ErrorCode::Busy)
                } else {
                    self.pending_ota = Some(OtaRequest { ssid, password, url });
                    Response::Ok
                }
            }
        }
    }
}
