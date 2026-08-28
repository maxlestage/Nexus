//! Moteur turbo : rafales automatiques sur les boutons marqués turbo.
//!
//! Deux usages :
//! - configuration depuis l'app iPhone (`TurboConfig::enabled_mask`) ;
//! - à la volée sur la manette : maintenir `TurboMod` et appuyer sur un
//!   bouton bascule le turbo pour ce bouton (feedback haptique + LED).

use crate::config::TurboConfig;

pub struct TurboEngine {
    /// Bitmask des boutons logiques en mode turbo.
    enabled_mask: u32,
    rate_hz: u8,
}

impl TurboEngine {
    pub fn new(cfg: &TurboConfig) -> Self {
        Self {
            enabled_mask: cfg.enabled_mask,
            rate_hz: cfg.rate_hz.clamp(1, 30),
        }
    }

    pub fn apply_config(&mut self, cfg: &TurboConfig) {
        self.enabled_mask = cfg.enabled_mask;
        self.rate_hz = cfg.rate_hz.clamp(1, 30);
    }

    pub fn enabled_mask(&self) -> u32 {
        self.enabled_mask
    }

    /// Bascule le turbo sur un bouton (via le modificateur TurboMod).
    /// Retourne `true` si le turbo est maintenant actif sur ce bouton.
    pub fn toggle(&mut self, button_mask: u32) -> bool {
        self.enabled_mask ^= button_mask;
        self.enabled_mask & button_mask != 0
    }

    /// Filtre les boutons logiques maintenus : ceux en mode turbo sont
    /// hachés à `rate_hz` (rapport cyclique 50 %), les autres passent tels
    /// quels.
    pub fn apply(&self, held_mask: u32, now_ms: u32) -> u32 {
        let turbo_held = held_mask & self.enabled_mask;
        if turbo_held == 0 {
            return held_mask;
        }
        let period_ms = 1000 / self.rate_hz as u32;
        let on = (now_ms % period_ms) < period_ms / 2;
        if on {
            held_mask
        } else {
            held_mask & !turbo_held
        }
    }
}
