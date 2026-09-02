//! Définition des entrées physiques (côté manette LEGO) et des boutons
//! logiques Switch (côté console).

use serde::{Deserialize, Serialize};

/// Nombre d'entrées physiques câblées sur l'ESP32.
pub const NUM_PHYSICAL: usize = 16;

/// Entrées physiques de la manette une main (usage main gauche).
///
/// La disposition est pensée pour une hémiplégie droite : tout est
/// atteignable avec le pouce, l'index et le majeur de la main gauche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PhysicalInput {
    /// Groupe de 4 boutons sous le pouce (haut).
    FaceTop = 0,
    /// Groupe pouce (droite).
    FaceRight = 1,
    /// Groupe pouce (bas).
    FaceBottom = 2,
    /// Groupe pouce (gauche).
    FaceLeft = 3,
    /// Gâchette index, rangée haute.
    IndexUpper = 4,
    /// Gâchette index, rangée basse.
    IndexLower = 5,
    /// Gâchette majeur, rangée haute.
    MiddleUpper = 6,
    /// Gâchette majeur, rangée basse.
    MiddleLower = 7,
    /// Bouton de paume (pression de la main).
    Palm = 8,
    /// Clic du joystick.
    StickClick = 9,
    /// Petit bouton `+`.
    Plus = 10,
    /// Petit bouton `-`.
    Minus = 11,
    /// Petit bouton Home.
    Home = 12,
    /// Petit bouton Capture.
    Capture = 13,
    /// Modificateur TURBO (maintenu + bouton = active le turbo dessus).
    TurboMod = 14,
    /// Modificateur SHIFT : bascule sur la couche 2 (stick droit + D-pad).
    ShiftMod = 15,
}

impl PhysicalInput {
    pub const ALL: [PhysicalInput; NUM_PHYSICAL] = [
        PhysicalInput::FaceTop,
        PhysicalInput::FaceRight,
        PhysicalInput::FaceBottom,
        PhysicalInput::FaceLeft,
        PhysicalInput::IndexUpper,
        PhysicalInput::IndexLower,
        PhysicalInput::MiddleUpper,
        PhysicalInput::MiddleLower,
        PhysicalInput::Palm,
        PhysicalInput::StickClick,
        PhysicalInput::Plus,
        PhysicalInput::Minus,
        PhysicalInput::Home,
        PhysicalInput::Capture,
        PhysicalInput::TurboMod,
        PhysicalInput::ShiftMod,
    ];

    #[inline]
    pub const fn mask(self) -> u16 {
        1u16 << (self as u8)
    }

    pub fn from_index(i: u8) -> Option<Self> {
        Self::ALL.get(i as usize).copied()
    }
}

/// Boutons logiques d'une manette Switch Pro Controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SwitchButton {
    Y = 0,
    X = 1,
    B = 2,
    A = 3,
    R = 4,
    Zr = 5,
    L = 6,
    Zl = 7,
    Minus = 8,
    Plus = 9,
    RStick = 10,
    LStick = 11,
    Home = 12,
    Capture = 13,
    DpadUp = 14,
    DpadDown = 15,
    DpadLeft = 16,
    DpadRight = 17,
}

pub const NUM_SWITCH_BUTTONS: usize = 18;

impl SwitchButton {
    /// Tous les boutons dans l'ordre d'affichage de l'application — à ne pas
    /// confondre avec l'ordre des variantes, qui donne la position des bits.
    pub const DISPLAY_ORDER: [SwitchButton; NUM_SWITCH_BUTTONS] = [
        SwitchButton::A,
        SwitchButton::B,
        SwitchButton::X,
        SwitchButton::Y,
        SwitchButton::L,
        SwitchButton::R,
        SwitchButton::Zl,
        SwitchButton::Zr,
        SwitchButton::Plus,
        SwitchButton::Minus,
        SwitchButton::LStick,
        SwitchButton::RStick,
        SwitchButton::Home,
        SwitchButton::Capture,
        SwitchButton::DpadUp,
        SwitchButton::DpadDown,
        SwitchButton::DpadLeft,
        SwitchButton::DpadRight,
    ];

    #[inline]
    pub const fn mask(self) -> u32 {
        1u32 << (self as u8)
    }

    pub fn from_index(i: u8) -> Option<Self> {
        const ALL: [SwitchButton; NUM_SWITCH_BUTTONS] = [
            SwitchButton::Y,
            SwitchButton::X,
            SwitchButton::B,
            SwitchButton::A,
            SwitchButton::R,
            SwitchButton::Zr,
            SwitchButton::L,
            SwitchButton::Zl,
            SwitchButton::Minus,
            SwitchButton::Plus,
            SwitchButton::RStick,
            SwitchButton::LStick,
            SwitchButton::Home,
            SwitchButton::Capture,
            SwitchButton::DpadUp,
            SwitchButton::DpadDown,
            SwitchButton::DpadLeft,
            SwitchButton::DpadRight,
        ];
        ALL.get(i as usize).copied()
    }
}
