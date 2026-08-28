//! Firmware de la manette une main « Nexus One ».
//!
//! Boucle principale à 1 kHz : scan des boutons → moteur logique
//! (`controller-core`) → rapport HID vers la Switch (BT classique) ou le
//! PC (BLE), avec en parallèle le service BLE de configuration (app
//! iPhone), l'haptique, les LEDs, la batterie et les statistiques.

mod battery;
mod bt;
mod config_service;
mod haptics;
mod inputs;
mod leds;
mod ota;
mod storage;

use bt::BtMode;
use controller_core::buttons::PhysicalInput;
use controller_core::engine::{Engine, InputFrame};
use controller_core::procon::{ProconIdentity, ProconProtocol};
use controller_core::report::{pack_pc_report, pack_short_report, pack_standard_report};
use esp_idf_hal::adc::attenuation::DB_11;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use std::time::{Duration, Instant};
use ws2812_esp32_rmt_driver::LedPixelEsp32Rmt;

const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    log::info!("Nexus One v{FIRMWARE_VERSION}");

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    // ---- Persistance : config + stats.
    let mut store = storage::Storage::new(nvs_partition.clone())?;
    let config = store.load_config();
    let stats = store.load_stats();
    let mut engine = Engine::new(config.clone(), stats);

    // ---- Entrées : 16 boutons (ordre = PhysicalInput::ALL) + joystick.
    let pins: [AnyIOPin; 16] = [
        peripherals.pins.gpio32.into(), // FaceTop
        peripherals.pins.gpio33.into(), // FaceRight
        peripherals.pins.gpio25.into(), // FaceBottom
        peripherals.pins.gpio26.into(), // FaceLeft
        peripherals.pins.gpio27.into(), // IndexUpper
        peripherals.pins.gpio14.into(), // IndexLower
        peripherals.pins.gpio13.into(), // MiddleUpper
        peripherals.pins.gpio4.into(),  // MiddleLower
        peripherals.pins.gpio5.into(),  // Palm
        peripherals.pins.gpio17.into(), // StickClick
        peripherals.pins.gpio18.into(), // Plus
        peripherals.pins.gpio19.into(), // Minus
        peripherals.pins.gpio23.into(), // Home
        peripherals.pins.gpio15.into(), // Capture
        peripherals.pins.gpio0.into(),  // TurboMod (bouton BOOT possible)
        peripherals.pins.gpio39.into(), // ShiftMod (pull-up externe)
    ];
    let mut buttons = inputs::Buttons::new(pins)?;

    let adc = AdcDriver::new(peripherals.adc1)?;
    let adc_cfg = AdcChannelConfig { attenuation: DB_11, ..Default::default() };
    let mut stick_x = AdcChannelDriver::new(&adc, peripherals.pins.gpio34, &adc_cfg)?;
    let mut stick_y = AdcChannelDriver::new(&adc, peripherals.pins.gpio35, &adc_cfg)?;
    let mut vbat = AdcChannelDriver::new(&adc, peripherals.pins.gpio36, &adc_cfg)?;

    // Auto-calibration du centre du stick (au repos pendant le boot).
    let (mut cx, mut cy) = (0u32, 0u32);
    for _ in 0..64 {
        cx += adc.read(&mut stick_x)? as u32;
        cy += adc.read(&mut stick_y)? as u32;
    }
    let stick = inputs::StickScaler::new((cx / 64) as u16, (cy / 64) as u16);

    // ---- Mode : MiddleLower maintenu au boot → PC, sinon Switch.
    let mode = if buttons.raw_is_pressed(PhysicalInput::MiddleLower) {
        BtMode::Pc
    } else {
        BtMode::Switch
    };
    log::info!("mode: {mode:?}");

    // ---- Haptique (DRV2605, I2C) & LEDs (WS2812B, RMT).
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &I2cConfig::new().baudrate(400u32.kHz().into()),
    )?;
    let mut haptics = haptics::Haptics::new(i2c)?;
    haptics.set_enabled(config.haptics.enabled);
    haptics.set_strength(config.haptics.strength);

    let ws2812 = LedPixelEsp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio16)?;
    let mut leds = leds::Leds::new(ws2812, config.leds);

    // ---- Bluetooth.
    bt::init_stack()?;
    let ble_service = bt::ble_config::BleConfigService::start()?;
    let mut procon_proto = ProconProtocol::new(ProconIdentity { mac: bt::local_mac() });
    let procon = match mode {
        BtMode::Switch => Some(bt::procon::Procon::start()?),
        BtMode::Pc => {
            bt::pc_hid::PcHid::start()?;
            None
        }
    };

    let mut config_srv = config_service::ConfigService::new(ble_service, FIRMWARE_VERSION);

    // Le modem WiFi n'est utilisé que si une OTA est demandée.
    let mut ota_modem = Some(peripherals.modem);

    // ---- Boucle principale (1 kHz).
    let started = Instant::now();
    let mut report_timer: u8 = 0;
    let mut last_report_ms: u32 = 0;
    let mut last_led_ms: u32 = 0;
    let mut last_battery_ms: u32 = 0;
    let mut battery_percent: u8 = 100;
    let mut battery_mv: u16 = 4200;

    loop {
        let now_ms = started.elapsed().as_millis() as u32;

        // 1. Entrées → moteur logique.
        let physical = buttons.scan();
        let (raw_x, raw_y) = (adc.read(&mut stick_x)?, adc.read(&mut stick_y)?);
        let (sx, sy) = stick.scale(raw_x, raw_y);
        let out = engine.tick(InputFrame { physical, stick_x: sx, stick_y: sy, now_ms });

        // 2. Retour local : clic haptique et LEDs réactives.
        if out.press_edge {
            leds.on_press();
            if engine.config().haptics.click_on_press {
                let _ = haptics.play_effect(haptics::effects::SOFT_CLICK);
            }
        }
        if let Some(on) = out.turbo_toggled {
            let effect = if on {
                haptics::effects::DOUBLE_CLICK
            } else {
                haptics::effects::SHORT_BUZZ
            };
            let _ = haptics.play_effect(effect);
        }

        // 3. Sortie HID. L'overlay LED est décidé ici, batterie comprise :
        // un `set_overlay` inconditionnel écraserait l'alerte batterie posée
        // ailleurs avant même le rendu à 50 Hz.
        let overlay_for = |connected: bool| {
            if battery_percent <= 10 {
                leds::StatusOverlay::LowBattery
            } else if connected {
                leds::StatusOverlay::None
            } else {
                leds::StatusOverlay::Pairing
            }
        };
        match mode {
            BtMode::Switch => {
                let connected = bt::procon::Procon::is_connected();
                leds.set_overlay(overlay_for(connected));

                // Rapports de sortie de la console (rumble, subcommands).
                if let Some(p) = &procon {
                    while let Ok(host) = p.host_rx.try_recv() {
                        let (reply, fx) = procon_proto.handle_output_report(
                            &host.data,
                            &out.state,
                            report_timer,
                            battery::percent_to_procon_level(battery_percent),
                        );
                        if let Some(r) = reply {
                            // Bourré à 63 octets : le composant esp_hid
                            // vérifie la longueur contre le report map, qui
                            // déclare les entrées vendeur à 63 octets.
                            let _ = bt::procon::Procon::send_input_report(
                                r.report_id,
                                &r.data[..63],
                            );
                        }
                        if let Some(amp) = fx.rumble_amplitude {
                            let _ = haptics.rumble(amp);
                        }
                        if let Some(n) = fx.player_number {
                            leds.set_player(n);
                        }
                    }
                }

                // Rapports d'entrée à ~66 Hz : 0x30 (complet, 63 octets
                // déclarés au report map) une fois le mode full demandé,
                // sinon 0x3F (court) — l'écran d'appairage de la console
                // s'appuie sur ces rapports simples.
                if connected && now_ms.wrapping_sub(last_report_ms) >= 15 {
                    last_report_ms = now_ms;
                    if procon_proto.input_mode == 0x30 {
                        report_timer = report_timer.wrapping_add(1);
                        let mut buf = [0u8; 63];
                        pack_standard_report(
                            &out.state,
                            report_timer,
                            battery::percent_to_procon_level(battery_percent),
                            &mut buf,
                        );
                        let _ = bt::procon::Procon::send_input_report(0x30, &buf);
                    } else {
                        let mut buf = [0u8; controller_core::report::SHORT_REPORT_LEN];
                        pack_short_report(&out.state, &mut buf);
                        let _ = bt::procon::Procon::send_input_report(0x3F, &buf);
                    }
                }
            }
            BtMode::Pc => {
                let connected = bt::pc_hid::PcHid::is_connected();
                leds.set_overlay(overlay_for(connected));
                if connected && now_ms.wrapping_sub(last_report_ms) >= 10 {
                    last_report_ms = now_ms;
                    let mut buf = [0u8; controller_core::report::PC_REPORT_LEN];
                    pack_pc_report(&out.state, &mut buf);
                    let _ = bt::pc_hid::PcHid::send_report(&buf);
                }
            }
        }

        // 4. Batterie (toutes les 5 s) ; l'alerte LED correspondante est
        // gérée par `overlay_for` à l'étape 3.
        if now_ms.wrapping_sub(last_battery_ms) >= 5000 {
            last_battery_ms = now_ms;
            battery_mv = battery::adc_to_battery_mv(adc.read(&mut vbat)?);
            battery_percent = battery::mv_to_percent(battery_mv);
        }

        // 5. Service de configuration (app iPhone).
        config_srv.poll(
            &mut engine,
            &mut store,
            &mut haptics,
            &mut leds,
            battery_mv,
            battery_percent,
        )?;

        // 6. LEDs à ~50 Hz.
        if now_ms.wrapping_sub(last_led_ms) >= 20 {
            last_led_ms = now_ms;
            let _ = leds.render(now_ms);
        }

        // 7. Stats : écriture NVS différée pour ménager la flash.
        if engine.stats_mut().should_persist() {
            let snapshot = engine.stats().clone();
            if store.save_stats(&snapshot).is_ok() {
                engine.stats_mut().mark_persisted();
            }
        }

        // 8. OTA demandée par l'app : on quitte la boucle et on lance la
        // mise à jour (le HID est coupé pendant le téléchargement).
        if let Some(req) = config_srv.take_ota_request() {
            log::info!("OTA demandée : {}", req.url);
            let modem = ota_modem.take().expect("une seule OTA par démarrage");
            let res = ota::run(modem, sysloop.clone(), nvs_partition.clone(), &req, |pct| {
                leds.set_overlay(leds::StatusOverlay::OtaProgress(pct));
                let _ = leds.render(pct as u32 * 100);
                config_service::ConfigService::notify_ota_progress(pct);
            });
            match res {
                Ok(()) => {
                    log::info!("OTA réussie, redémarrage");
                    unsafe { esp_idf_svc::sys::esp_restart() };
                }
                Err(e) => {
                    log::error!("OTA échouée: {e}");
                    unsafe { esp_idf_svc::sys::esp_restart() };
                }
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}
