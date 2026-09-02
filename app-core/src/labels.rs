//! Libellés affichés à l'utilisateur. Ils vivent en Rust : Swift ne
//! contient aucun texte d'interface.

use controller_core::buttons::{PhysicalInput, SwitchButton};

pub fn physical(input: PhysicalInput) -> &'static str {
    use PhysicalInput::*;
    match input {
        FaceTop => "Pouce · haut",
        FaceRight => "Pouce · droite",
        FaceBottom => "Pouce · bas",
        FaceLeft => "Pouce · gauche",
        IndexUpper => "Index · gâchette haute",
        IndexLower => "Index · gâchette basse",
        MiddleUpper => "Majeur · gâchette haute",
        MiddleLower => "Majeur · gâchette basse",
        Palm => "Paume",
        StickClick => "Clic du stick",
        Plus => "Bouton +",
        Minus => "Bouton −",
        Home => "Home",
        Capture => "Capture",
        TurboMod => "Modificateur TURBO",
        ShiftMod => "Modificateur SHIFT",
    }
}

pub fn switch_button(button: SwitchButton) -> &'static str {
    use SwitchButton::*;
    match button {
        A => "A",
        B => "B",
        X => "X",
        Y => "Y",
        L => "L",
        R => "R",
        Zl => "ZL",
        Zr => "ZR",
        Plus => "+",
        Minus => "−",
        LStick => "Clic stick gauche",
        RStick => "Clic stick droit",
        Home => "Home",
        Capture => "Capture",
        DpadUp => "Croix ↑",
        DpadDown => "Croix ↓",
        DpadLeft => "Croix ←",
        DpadRight => "Croix →",
    }
}

/// Nom stable utilisé dans les identifiants d'action (jamais affiché).
pub fn switch_button_key(button: SwitchButton) -> &'static str {
    use SwitchButton::*;
    match button {
        A => "A",
        B => "B",
        X => "X",
        Y => "Y",
        L => "L",
        R => "R",
        Zl => "Zl",
        Zr => "Zr",
        Plus => "Plus",
        Minus => "Minus",
        LStick => "LStick",
        RStick => "RStick",
        Home => "Home",
        Capture => "Capture",
        DpadUp => "DpadUp",
        DpadDown => "DpadDown",
        DpadLeft => "DpadLeft",
        DpadRight => "DpadRight",
    }
}

pub fn switch_button_from_key(key: &str) -> Option<SwitchButton> {
    SwitchButton::DISPLAY_ORDER
        .iter()
        .copied()
        .find(|b| switch_button_key(*b) == key)
}

/// Durée de jeu en français, arrondie à la minute.
pub fn duration(seconds: u32) -> String {
    let (h, m) = (seconds / 3600, (seconds % 3600) / 60);
    if h > 0 {
        format!("{h} h {m:02} min")
    } else {
        format!("{m} min")
    }
}
