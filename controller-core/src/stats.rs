//! Statistiques d'utilisation : compteur d'appuis par entrée physique,
//! persisté périodiquement en NVS et consultable depuis l'app iPhone.

use crate::buttons::NUM_PHYSICAL;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Stats {
    /// Appuis par entrée physique (indexé par `PhysicalInput as usize`).
    pub presses: [u32; NUM_PHYSICAL],
    /// Temps de jeu cumulé en secondes.
    pub uptime_s: u32,
    /// Nombre de macros déclenchées.
    pub macros_fired: u32,
}

pub struct StatsTracker {
    stats: Stats,
    prev_physical: u16,
    /// Compte les changements depuis la dernière sauvegarde.
    dirty_presses: u32,
    last_uptime_ms: u32,
    uptime_acc_ms: u32,
}

impl StatsTracker {
    pub fn new(initial: Stats) -> Self {
        Self {
            stats: initial,
            prev_physical: 0,
            dirty_presses: 0,
            last_uptime_ms: 0,
            uptime_acc_ms: 0,
        }
    }

    /// À appeler à chaque tick avec l'état physique débouncé.
    pub fn tick(&mut self, physical: u16, now_ms: u32) {
        let rising = physical & !self.prev_physical;
        if rising != 0 {
            for i in 0..NUM_PHYSICAL {
                if rising & (1 << i) != 0 {
                    self.stats.presses[i] = self.stats.presses[i].saturating_add(1);
                    self.dirty_presses += 1;
                }
            }
        }
        self.prev_physical = physical;

        let dt = now_ms.wrapping_sub(self.last_uptime_ms);
        self.last_uptime_ms = now_ms;
        // Ignore le premier tick / les sauts d'horloge.
        if dt < 1000 {
            self.uptime_acc_ms += dt;
            while self.uptime_acc_ms >= 1000 {
                self.uptime_acc_ms -= 1000;
                self.stats.uptime_s = self.stats.uptime_s.saturating_add(1);
            }
        }
    }

    pub fn on_macro_fired(&mut self) {
        self.stats.macros_fired = self.stats.macros_fired.saturating_add(1);
        self.dirty_presses += 1;
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    pub fn reset(&mut self) {
        self.stats = Stats::default();
        self.dirty_presses = 1;
    }

    /// `true` si assez de nouveautés pour justifier une écriture NVS
    /// (on limite l'usure de la flash).
    pub fn should_persist(&self) -> bool {
        self.dirty_presses >= 50
    }

    pub fn mark_persisted(&mut self) {
        self.dirty_presses = 0;
    }
}
