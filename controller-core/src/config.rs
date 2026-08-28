//! Configuration persistante de la manette : mapping des boutons,
//! turbo, macros, LEDs, haptique. Sérialisée en `postcard` dans la NVS
//! de l'ESP32 et échangée telle quelle avec l'application iPhone.

use crate::buttons::{PhysicalInput, SwitchButton, NUM_PHYSICAL};
use crate::macros_engine::MacroDef;
use heapless::Vec;
use serde::{Deserialize, Serialize};

/// Version du format de configuration (pour migrations futures).
pub const CONFIG_VERSION: u8 = 1;

/// Nombre maximal de macros stockées.
pub const MAX_MACROS: usize = 8;

/// Vers quel stick logique le joystick physique unique est envoyé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StickTarget {
    Left,
    Right,
}

/// Mode d'éclairage du bandeau WS2812B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedMode {
    Off,
    /// Couleur fixe.
    Solid,
    /// Respiration lente sur la couleur choisie.
    Breathe,
    /// Arc-en-ciel défilant.
    Rainbow,
    /// Flash à chaque appui de bouton.
    React,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedConfig {
    pub mode: LedMode,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Luminosité globale 0..=255.
    pub brightness: u8,
}

impl Default for LedConfig {
    fn default() -> Self {
        Self {
            mode: LedMode::Breathe,
            r: 0,
            g: 120,
            b: 255,
            brightness: 80,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HapticConfig {
    pub enabled: bool,
    /// Intensité 0..=127 (registre DRV2605).
    pub strength: u8,
    /// Clic haptique local à chaque appui (en plus du rumble console).
    pub click_on_press: bool,
}

impl Default for HapticConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strength: 90,
            click_on_press: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurboConfig {
    /// Fréquence des rafales en Hz (1..=30).
    pub rate_hz: u8,
    /// Boutons logiques avec turbo actif (bitmask `SwitchButton::mask`).
    pub enabled_mask: u32,
}

impl Default for TurboConfig {
    fn default() -> Self {
        Self {
            rate_hz: 12,
            enabled_mask: 0,
        }
    }
}

/// Mapping d'une couche : pour chaque entrée physique, un bouton Switch
/// (ou `None` si l'entrée est inerte sur cette couche).
pub type LayerMap = [Option<SwitchButton>; NUM_PHYSICAL];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub version: u8,
    /// Couche normale.
    pub layer_normal: LayerMap,
    /// Couche active tant que `ShiftMod` est maintenu.
    pub layer_shift: LayerMap,
    /// Cible du joystick sur la couche normale.
    pub stick_normal: StickTarget,
    /// Cible du joystick sur la couche shift.
    pub stick_shift: StickTarget,
    pub turbo: TurboConfig,
    pub macros: Vec<MacroDef, MAX_MACROS>,
    pub leds: LedConfig,
    pub haptics: HapticConfig,
    /// Zone morte du joystick (0..=1000, en millièmes de la course).
    pub stick_deadzone: u16,
}

impl Default for Config {
    fn default() -> Self {
        use PhysicalInput as P;
        use SwitchButton as S;

        let mut normal: LayerMap = [None; NUM_PHYSICAL];
        normal[P::FaceTop as usize] = Some(S::X);
        normal[P::FaceRight as usize] = Some(S::A);
        normal[P::FaceBottom as usize] = Some(S::B);
        normal[P::FaceLeft as usize] = Some(S::Y);
        normal[P::IndexUpper as usize] = Some(S::R);
        normal[P::IndexLower as usize] = Some(S::Zr);
        normal[P::MiddleUpper as usize] = Some(S::L);
        normal[P::MiddleLower as usize] = Some(S::Zl);
        normal[P::Palm as usize] = Some(S::RStick);
        normal[P::StickClick as usize] = Some(S::LStick);
        normal[P::Plus as usize] = Some(S::Plus);
        normal[P::Minus as usize] = Some(S::Minus);
        normal[P::Home as usize] = Some(S::Home);
        normal[P::Capture as usize] = Some(S::Capture);
        // TurboMod et ShiftMod ne sont pas mappés : ce sont des modificateurs.

        // Couche shift : les 4 boutons du pouce deviennent le D-pad,
        // le joystick devient le stick droit (caméra).
        let mut shift: LayerMap = [None; NUM_PHYSICAL];
        shift[P::FaceTop as usize] = Some(S::DpadUp);
        shift[P::FaceRight as usize] = Some(S::DpadRight);
        shift[P::FaceBottom as usize] = Some(S::DpadDown);
        shift[P::FaceLeft as usize] = Some(S::DpadLeft);
        shift[P::IndexUpper as usize] = Some(S::R);
        shift[P::IndexLower as usize] = Some(S::Zr);
        shift[P::MiddleUpper as usize] = Some(S::L);
        shift[P::MiddleLower as usize] = Some(S::Zl);

        Self {
            version: CONFIG_VERSION,
            layer_normal: normal,
            layer_shift: shift,
            stick_normal: StickTarget::Left,
            stick_shift: StickTarget::Right,
            turbo: TurboConfig::default(),
            macros: Vec::new(),
            leds: LedConfig::default(),
            haptics: HapticConfig::default(),
            stick_deadzone: 80,
        }
    }
}

impl Config {
    /// Sérialise la config en octets `postcard` (pour NVS ou BLE).
    pub fn to_bytes<'a>(&self, buf: &'a mut [u8]) -> Result<&'a mut [u8], postcard::Error> {
        postcard::to_slice(self, buf)
    }

    /// Désérialise, en refusant les versions inconnues.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        let cfg: Config = postcard::from_bytes(bytes)?;
        if cfg.version != CONFIG_VERSION {
            return Err(postcard::Error::DeserializeBadEncoding);
        }
        Ok(cfg)
    }
}
