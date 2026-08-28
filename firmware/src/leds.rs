//! Éclairage WS2812B (8 LEDs) via le périphérique RMT.
//!
//! Modes : éteint, couleur fixe, respiration, arc-en-ciel, flash sur appui.
//! La LED 0 sert aussi d'indicateur d'état (appairage, batterie faible,
//! numéro de joueur, progression OTA).

use controller_core::config::{LedConfig, LedMode};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::driver::color::LedPixelColorGrb24;
use ws2812_esp32_rmt_driver::LedPixelEsp32Rmt;

pub const NUM_LEDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusOverlay {
    None,
    /// Clignotement bleu : en attente d'appairage.
    Pairing,
    /// Clignotement rouge : batterie faible.
    LowBattery,
    /// Vert progressif : OTA en cours (0..=100).
    OtaProgress(u8),
}

pub struct Leds<'d> {
    driver: LedPixelEsp32Rmt<'d, RGB8, LedPixelColorGrb24>,
    config: LedConfig,
    overlay: StatusOverlay,
    player: u8,
    /// Décroît après chaque appui (mode React).
    react_level: u8,
}

impl<'d> Leds<'d> {
    pub fn new(
        driver: LedPixelEsp32Rmt<'d, RGB8, LedPixelColorGrb24>,
        config: LedConfig,
    ) -> Self {
        Self { driver, config, overlay: StatusOverlay::Pairing, player: 0, react_level: 0 }
    }

    pub fn set_config(&mut self, config: LedConfig) {
        self.config = config;
    }

    pub fn set_overlay(&mut self, overlay: StatusOverlay) {
        self.overlay = overlay;
    }

    pub fn set_player(&mut self, player: u8) {
        self.player = player;
    }

    pub fn on_press(&mut self) {
        self.react_level = 255;
    }

    /// Rafraîchit le bandeau. À appeler à ~50 Hz avec une horloge en ms.
    pub fn render(&mut self, now_ms: u32) -> anyhow::Result<()> {
        let mut px = [RGB8::default(); NUM_LEDS];
        let c = RGB8::new(self.config.r, self.config.g, self.config.b);
        let bright = self.config.brightness as u16;

        match self.config.mode {
            LedMode::Off => {}
            LedMode::Solid => px = [scale(c, bright); NUM_LEDS],
            LedMode::Breathe => {
                // Triangle 0..255..0 sur 3 s.
                let t = (now_ms % 3000) as u16;
                let level = if t < 1500 { t * 255 / 1500 } else { (3000 - t) * 255 / 1500 };
                px = [scale(c, bright * level / 255); NUM_LEDS];
            }
            LedMode::Rainbow => {
                for (i, p) in px.iter_mut().enumerate() {
                    let hue = ((now_ms / 10) as u16 + (i as u16 * 256 / NUM_LEDS as u16)) % 256;
                    *p = scale(hsv(hue as u8), bright);
                }
            }
            LedMode::React => {
                px = [scale(c, bright * self.react_level as u16 / 255); NUM_LEDS];
                self.react_level = self.react_level.saturating_sub(12);
            }
        }

        // Superpositions d'état sur la LED 0 (et 1..4 pour le joueur).
        match self.overlay {
            StatusOverlay::None => {
                if self.player > 0 {
                    for i in 0..(self.player.min(4) as usize) {
                        px[i] = scale(RGB8::new(255, 255, 255), bright / 2);
                    }
                }
            }
            StatusOverlay::Pairing => {
                let on = (now_ms / 250) % 2 == 0;
                px[0] = if on { RGB8::new(0, 0, 180) } else { RGB8::default() };
            }
            StatusOverlay::LowBattery => {
                let on = (now_ms / 500) % 2 == 0;
                px[0] = if on { RGB8::new(180, 0, 0) } else { RGB8::default() };
            }
            StatusOverlay::OtaProgress(pct) => {
                let lit = (pct as usize * NUM_LEDS).div_ceil(100).min(NUM_LEDS);
                px = [RGB8::default(); NUM_LEDS];
                for p in px.iter_mut().take(lit) {
                    *p = RGB8::new(0, 120, 0);
                }
            }
        }

        self.driver.write(px.iter().copied())?;
        Ok(())
    }
}

fn scale(c: RGB8, level: u16) -> RGB8 {
    let level = level.min(255);
    RGB8::new(
        (c.r as u16 * level / 255) as u8,
        (c.g as u16 * level / 255) as u8,
        (c.b as u16 * level / 255) as u8,
    )
}

/// Conversion teinte → RGB (S=V=max), suffisante pour l'arc-en-ciel.
fn hsv(h: u8) -> RGB8 {
    let region = h / 43;
    let rem = (h % 43) * 6;
    let q = 255 - rem;
    match region {
        0 => RGB8::new(255, rem, 0),
        1 => RGB8::new(q, 255, 0),
        2 => RGB8::new(0, 255, rem),
        3 => RGB8::new(0, q, 255),
        4 => RGB8::new(rem, 0, 255),
        _ => RGB8::new(255, 0, q),
    }
}
