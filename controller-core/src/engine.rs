//! Moteur principal : transforme l'état physique brut (boutons + stick)
//! en état logique Switch, en appliquant couches, mapping, macros et turbo.

use crate::buttons::PhysicalInput;
use crate::config::{Config, StickTarget};
use crate::macros_engine::MacroEngine;
use crate::report::{SwitchState, STICK_CENTER};
use crate::stats::{Stats, StatsTracker};
use crate::turbo::TurboEngine;

/// Entrées brutes d'un tick.
#[derive(Debug, Clone, Copy)]
pub struct InputFrame {
    /// Bitmask `PhysicalInput::mask` des entrées pressées (débouncées).
    pub physical: u16,
    /// Joystick physique, -1000..=1000 par axe (0 = centre).
    pub stick_x: i16,
    pub stick_y: i16,
    /// Horloge monotone en millisecondes.
    pub now_ms: u32,
}

/// Résultat d'un tick.
#[derive(Debug, Clone, Copy)]
pub struct EngineOutput {
    pub state: SwitchState,
    /// Un bouton a été enfoncé ce tick (pour clic haptique / LED React).
    pub press_edge: bool,
    /// Le turbo vient d'être basculé sur un bouton (feedback à donner).
    pub turbo_toggled: Option<bool>,
}

pub struct Engine {
    config: Config,
    turbo: TurboEngine,
    macros: MacroEngine,
    stats: StatsTracker,
    prev_physical: u16,
    /// Boutons déjà basculés pendant le maintien courant de TurboMod.
    turbo_latch: u16,
}

impl Engine {
    pub fn new(config: Config, initial_stats: Stats) -> Self {
        let turbo = TurboEngine::new(&config.turbo);
        Self {
            config,
            turbo,
            macros: MacroEngine::new(),
            stats: StatsTracker::new(initial_stats),
            prev_physical: 0,
            turbo_latch: 0,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Remplace la configuration (reçue de l'app iPhone).
    pub fn set_config(&mut self, config: Config) {
        self.turbo.apply_config(&config.turbo);
        // La liste des macros change : une lecture en cours indexerait
        // l'ancienne liste (panic possible si elle a rétréci).
        self.macros = MacroEngine::new();
        self.config = config;
    }

    /// Config avec le masque turbo courant (modifié à la volée), pour la
    /// persistance.
    pub fn config_for_save(&self) -> Config {
        let mut c = self.config.clone();
        c.turbo.enabled_mask = self.turbo.enabled_mask();
        c
    }

    pub fn stats(&self) -> &Stats {
        self.stats.stats()
    }

    pub fn stats_mut(&mut self) -> &mut StatsTracker {
        &mut self.stats
    }

    pub fn tick(&mut self, frame: InputFrame) -> EngineOutput {
        let physical = frame.physical;
        let rising = physical & !self.prev_physical;

        self.stats.tick(physical, frame.now_ms);

        let shift_held = physical & PhysicalInput::ShiftMod.mask() != 0;
        let turbo_mod_held = physical & PhysicalInput::TurboMod.mask() != 0;

        // Bascule turbo : TurboMod maintenu + front montant d'un bouton mappé.
        let mut turbo_toggled = None;
        if turbo_mod_held {
            let candidates = rising
                & !(PhysicalInput::TurboMod.mask() | PhysicalInput::ShiftMod.mask())
                & !self.turbo_latch;
            if candidates != 0 {
                let layer = if shift_held {
                    &self.config.layer_shift
                } else {
                    &self.config.layer_normal
                };
                for (i, mapped) in layer.iter().enumerate() {
                    if candidates & (1 << i) != 0 {
                        if let Some(btn) = mapped {
                            turbo_toggled = Some(self.turbo.toggle(btn.mask()));
                            self.turbo_latch |= 1 << i;
                        }
                    }
                }
            }
        } else {
            self.turbo_latch = 0;
        }

        // Macros (elles masquent les entrées de leur accord déclencheur).
        // Pendant le maintien de TurboMod, les accords servent à configurer
        // le turbo : pas de nouveau déclenchement de macro.
        let was_playing = self.macros.is_playing();
        let macro_out =
            self.macros
                .tick(&self.config.macros, physical, frame.now_ms, !turbo_mod_held);
        if !was_playing && self.macros.is_playing() {
            self.stats.on_macro_fired();
        }

        // Mapping couche → boutons logiques.
        let layer = if shift_held {
            &self.config.layer_shift
        } else {
            &self.config.layer_normal
        };
        let effective = physical
            & !macro_out.suppress_physical
            & !(PhysicalInput::TurboMod.mask() | PhysicalInput::ShiftMod.mask());
        // Pendant le maintien de TurboMod, les boutons servent à configurer
        // le turbo : on n'envoie pas leurs appuis à la console.
        let effective = if turbo_mod_held { 0 } else { effective };

        let mut logical: u32 = 0;
        for (i, mapped) in layer.iter().enumerate() {
            if effective & (1 << i) != 0 {
                if let Some(btn) = mapped {
                    logical |= btn.mask();
                }
            }
        }

        // Turbo puis injection des macros (les macros ne subissent pas le
        // turbo : leur timing est déjà défini par leurs étapes).
        let logical = self.turbo.apply(logical, frame.now_ms) | macro_out.buttons_mask;

        // Joystick : zone morte puis routage vers le stick choisi.
        let (sx, sy) = apply_deadzone(frame.stick_x, frame.stick_y, self.config.stick_deadzone);
        let target = if shift_held {
            self.config.stick_shift
        } else {
            self.config.stick_normal
        };
        let mut state = SwitchState::centered();
        state.buttons = logical;
        let (ax, ay) = (axis_to_12bit(sx), axis_to_12bit(sy));
        match target {
            StickTarget::Left => {
                state.lx = ax;
                state.ly = ay;
            }
            StickTarget::Right => {
                state.rx = ax;
                state.ry = ay;
            }
        }

        self.prev_physical = physical;

        // Les modificateurs seuls ne comptent pas comme un appui (pas de
        // clic haptique ni de flash LED quand on prend TURBO ou SHIFT).
        let modifiers = PhysicalInput::TurboMod.mask() | PhysicalInput::ShiftMod.mask();

        EngineOutput {
            state,
            press_edge: rising & !modifiers != 0,
            turbo_toggled,
        }
    }
}

/// Zone morte radiale approchée (par axe), entrée/sortie en -1000..=1000.
fn apply_deadzone(x: i16, y: i16, deadzone: u16) -> (i16, i16) {
    let dz = deadzone as i32;
    let scale = |v: i16| -> i16 {
        let v = v.clamp(-1000, 1000) as i32;
        if v.abs() <= dz {
            0
        } else {
            // Re-normalise pour garder toute la course après la zone morte.
            let sign = if v < 0 { -1 } else { 1 };
            let out = (v.abs() - dz) * 1000 / (1000 - dz);
            (sign * out) as i16
        }
    };
    (scale(x), scale(y))
}

/// -1000..=1000 → 12 bits centrés sur 0x800.
fn axis_to_12bit(v: i16) -> u16 {
    let v = v.clamp(-1000, 1000) as i32;
    (STICK_CENTER as i32 + v * 0x7FF / 1000).clamp(0, 0xFFF) as u16
}
