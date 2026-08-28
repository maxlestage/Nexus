//! Macros : une combinaison physique (accord de boutons) déclenche une
//! séquence de boutons logiques minutée.
//!
//! Exemple : `FaceRight + FaceBottom` (A+B physiques) → appui `X` 50 ms.

use crate::buttons::PhysicalInput;
use heapless::Vec;
use serde::{Deserialize, Serialize};

pub const MAX_STEPS: usize = 16;

/// Une étape de macro : boutons logiques maintenus pendant `duration_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroStep {
    /// Bitmask `SwitchButton::mask` (0 = relâcher tout : pause).
    pub buttons_mask: u32,
    pub duration_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroDef {
    /// Accord physique déclencheur (bitmask `PhysicalInput::mask`).
    /// La macro part quand TOUTES ces entrées sont pressées ensemble.
    pub trigger_mask: u16,
    pub steps: Vec<MacroStep, MAX_STEPS>,
}

impl MacroDef {
    /// Petit constructeur pratique pour "A+B = X" et compagnie.
    pub fn chord_to_button(chord: &[PhysicalInput], out_mask: u32, hold_ms: u16) -> Self {
        let mut trigger = 0u16;
        for p in chord {
            trigger |= p.mask();
        }
        let mut steps = Vec::new();
        let _ = steps.push(MacroStep {
            buttons_mask: out_mask,
            duration_ms: hold_ms,
        });
        Self {
            trigger_mask: trigger,
            steps,
        }
    }
}

/// État d'exécution des macros.
pub struct MacroEngine {
    /// Index de la macro en cours et étape courante.
    playing: Option<Playing>,
    /// Mémorise l'état physique précédent pour détecter le front montant
    /// de l'accord complet.
    prev_physical: u16,
}

struct Playing {
    macro_idx: usize,
    step_idx: usize,
    step_started_ms: u32,
}

pub struct MacroOutput {
    /// Boutons logiques injectés par la macro.
    pub buttons_mask: u32,
    /// Entrées physiques à masquer (celles de l'accord déclencheur),
    /// pour ne pas envoyer en plus leurs mappings normaux.
    pub suppress_physical: u16,
}

impl MacroEngine {
    pub fn new() -> Self {
        Self {
            playing: None,
            prev_physical: 0,
        }
    }

    /// Avance l'état des macros. À appeler à chaque tick (1 kHz typ.).
    pub fn tick(&mut self, macros: &[MacroDef], physical: u16, now_ms: u32) -> MacroOutput {
        // Macro en cours : on la déroule jusqu'au bout.
        if let Some(p) = &mut self.playing {
            let def = &macros[p.macro_idx];
            let step = &def.steps[p.step_idx];
            if now_ms.wrapping_sub(p.step_started_ms) >= step.duration_ms as u32 {
                p.step_idx += 1;
                p.step_started_ms = now_ms;
                if p.step_idx >= def.steps.len() {
                    let suppress = def.trigger_mask;
                    self.playing = None;
                    self.prev_physical = physical;
                    // Dernier tick : on continue de masquer l'accord tant
                    // qu'il est physiquement maintenu.
                    return MacroOutput {
                        buttons_mask: 0,
                        suppress_physical: suppress & physical,
                    };
                }
            }
            let def = &macros[p.macro_idx];
            let out = def.steps[p.step_idx].buttons_mask;
            self.prev_physical = physical;
            return MacroOutput {
                buttons_mask: out,
                suppress_physical: def.trigger_mask,
            };
        }

        // Détection d'un nouvel accord complet (front montant).
        let newly_pressed = physical & !self.prev_physical;
        if newly_pressed != 0 {
            for (idx, def) in macros.iter().enumerate() {
                if def.trigger_mask != 0
                    && physical & def.trigger_mask == def.trigger_mask
                    && newly_pressed & def.trigger_mask != 0
                    && !def.steps.is_empty()
                {
                    self.playing = Some(Playing {
                        macro_idx: idx,
                        step_idx: 0,
                        step_started_ms: now_ms,
                    });
                    self.prev_physical = physical;
                    return MacroOutput {
                        buttons_mask: def.steps[0].buttons_mask,
                        suppress_physical: def.trigger_mask,
                    };
                }
            }
        }

        self.prev_physical = physical;
        MacroOutput {
            buttons_mask: 0,
            suppress_physical: 0,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing.is_some()
    }
}

impl Default for MacroEngine {
    fn default() -> Self {
        Self::new()
    }
}
