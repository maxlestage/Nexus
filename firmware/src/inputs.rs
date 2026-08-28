//! Lecture des 16 boutons (GPIO, actifs à l'état bas) et du joystick
//! analogique, avec débounce logiciel.
//!
//! Brochage complet : voir `docs/WIRING.md`.

use controller_core::buttons::{PhysicalInput, NUM_PHYSICAL};
use esp_idf_hal::gpio::{AnyIOPin, Input, PinDriver, Pull};

/// Fenêtre de débounce : un bouton doit être stable pendant N ticks (1 kHz).
const DEBOUNCE_TICKS: u8 = 5;

pub struct Buttons<'d> {
    pins: [PinDriver<'d, AnyIOPin, Input>; NUM_PHYSICAL],
    integrators: [u8; NUM_PHYSICAL],
    stable: u16,
}

impl<'d> Buttons<'d> {
    /// `pins[i]` correspond à `PhysicalInput::ALL[i]`.
    pub fn new(pins: [AnyIOPin; NUM_PHYSICAL]) -> anyhow::Result<Self> {
        let mut drivers: heapless::Vec<PinDriver<'d, AnyIOPin, Input>, NUM_PHYSICAL> =
            heapless::Vec::new();
        for (i, pin) in pins.into_iter().enumerate() {
            let mut d = PinDriver::input(pin)?;
            // ShiftMod est sur GPIO39 (entrée seule) : pas de pull-up
            // interne, résistance externe obligatoire (docs/WIRING.md).
            if PhysicalInput::ALL[i] != PhysicalInput::ShiftMod {
                d.set_pull(Pull::Up)?;
            }
            drivers.push(d).map_err(|_| anyhow::anyhow!("trop de pins"))?;
        }
        let pins = drivers
            .into_array()
            .map_err(|_| anyhow::anyhow!("il faut exactement 16 pins"))?;
        Ok(Self { pins, integrators: [0; NUM_PHYSICAL], stable: 0 })
    }

    /// À appeler à 1 kHz. Retourne le bitmask débouncé
    /// (`PhysicalInput::mask`).
    pub fn scan(&mut self) -> u16 {
        for i in 0..NUM_PHYSICAL {
            let pressed = self.pins[i].is_low(); // actif bas
            let integ = &mut self.integrators[i];
            if pressed {
                if *integ < DEBOUNCE_TICKS {
                    *integ += 1;
                    if *integ == DEBOUNCE_TICKS {
                        self.stable |= 1 << i;
                    }
                }
            } else if *integ > 0 {
                *integ -= 1;
                if *integ == 0 {
                    self.stable &= !(1 << i);
                }
            }
        }
        self.stable
    }

    /// Lecture immédiate (avant débounce), pour la détection du mode au boot.
    pub fn raw_is_pressed(&self, p: PhysicalInput) -> bool {
        self.pins[p as usize].is_low()
    }
}

/// Joystick analogique : convertit les lectures ADC brutes (0..=4095,
/// X = GPIO34, Y = GPIO35) en axes -1000..=1000 avec auto-calibration du
/// centre au démarrage. La possession des canaux ADC reste dans `main.rs` ;
/// ce type ne fait que la conversion.
pub struct StickScaler {
    center_x: i32,
    center_y: i32,
}

impl StickScaler {
    /// `center_*` : moyenne de lectures au boot, stick au repos.
    pub fn new(center_x: u16, center_y: u16) -> Self {
        Self { center_x: center_x as i32, center_y: center_y as i32 }
    }

    /// Axes en -1000..=1000 (0 = centre), Y vers le haut.
    pub fn scale(&self, raw_x: u16, raw_y: u16) -> (i16, i16) {
        let to_axis = |raw: u16, center: i32| -> i16 {
            let d = raw as i32 - center;
            let span_pos = (4095 - center).max(1);
            let span_neg = center.max(1);
            let v = if d >= 0 { d * 1000 / span_pos } else { d * 1000 / span_neg };
            v.clamp(-1000, 1000) as i16
        };
        (to_axis(raw_x, self.center_x), -to_axis(raw_y, self.center_y))
    }
}
