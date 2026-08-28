//! Construction du rapport d'entrée "standard full" (0x30) du
//! Pro Controller Switch, et du rapport HID gamepad générique pour le
//! mode PC (Windows/macOS).

use crate::buttons::SwitchButton;

/// État logique complet envoyé à la console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SwitchState {
    /// Bitmask `SwitchButton::mask`.
    pub buttons: u32,
    /// Sticks en 12 bits, centre = 0x800.
    pub lx: u16,
    pub ly: u16,
    pub rx: u16,
    pub ry: u16,
}

pub const STICK_CENTER: u16 = 0x800;

impl SwitchState {
    pub fn centered() -> Self {
        Self {
            buttons: 0,
            lx: STICK_CENTER,
            ly: STICK_CENTER,
            rx: STICK_CENTER,
            ry: STICK_CENTER,
        }
    }

    #[inline]
    fn has(&self, b: SwitchButton) -> bool {
        self.buttons & b.mask() != 0
    }
}

/// Corps du rapport 0x30 (sans l'ID de rapport) : 11 octets utiles
/// suivis des données IMU (laissées à zéro ici).
pub const INPUT_REPORT_LEN: usize = 48;

/// Remplit `out` (>= 48 octets) au format "standard full input report".
///
/// Format documenté par le reverse engineering communautaire
/// (dekuNukem/Nintendo_Switch_Reverse_Engineering) :
/// - octet 0 : timer 1 octet qui s'incrémente
/// - octet 1 : niveau batterie (poids fort) | type de connexion
/// - octets 2..=4 : boutons
/// - octets 5..=7 : stick gauche (2 × 12 bits)
/// - octets 8..=10 : stick droit
/// - octet 11 : état du vibreur
pub fn pack_standard_report(state: &SwitchState, timer: u8, battery_level: u8, out: &mut [u8]) {
    assert!(out.len() >= INPUT_REPORT_LEN);
    for b in out[..INPUT_REPORT_LEN].iter_mut() {
        *b = 0;
    }

    out[0] = timer;
    // Batterie sur le quartet haut (8 = pleine, par pas de 2), connexion
    // 0x1 = manette Pro sur le quartet bas... 0xE = pleine + BT.
    out[1] = ((battery_level & 0x0F) << 4) | 0x0E;

    // Octet 2 : Y X B A SR SL R ZR (bits 0..7)
    let mut b2 = 0u8;
    if state.has(SwitchButton::Y) {
        b2 |= 0x01;
    }
    if state.has(SwitchButton::X) {
        b2 |= 0x02;
    }
    if state.has(SwitchButton::B) {
        b2 |= 0x04;
    }
    if state.has(SwitchButton::A) {
        b2 |= 0x08;
    }
    if state.has(SwitchButton::R) {
        b2 |= 0x40;
    }
    if state.has(SwitchButton::Zr) {
        b2 |= 0x80;
    }
    out[2] = b2;

    // Octet 3 : Minus Plus RStick LStick Home Capture -- ChargingGrip
    let mut b3 = 0u8;
    if state.has(SwitchButton::Minus) {
        b3 |= 0x01;
    }
    if state.has(SwitchButton::Plus) {
        b3 |= 0x02;
    }
    if state.has(SwitchButton::RStick) {
        b3 |= 0x04;
    }
    if state.has(SwitchButton::LStick) {
        b3 |= 0x08;
    }
    if state.has(SwitchButton::Home) {
        b3 |= 0x10;
    }
    if state.has(SwitchButton::Capture) {
        b3 |= 0x20;
    }
    out[3] = b3;

    // Octet 4 : Down Up Right Left SR SL L ZL
    let mut b4 = 0u8;
    if state.has(SwitchButton::DpadDown) {
        b4 |= 0x01;
    }
    if state.has(SwitchButton::DpadUp) {
        b4 |= 0x02;
    }
    if state.has(SwitchButton::DpadRight) {
        b4 |= 0x04;
    }
    if state.has(SwitchButton::DpadLeft) {
        b4 |= 0x08;
    }
    if state.has(SwitchButton::L) {
        b4 |= 0x40;
    }
    if state.has(SwitchButton::Zl) {
        b4 |= 0x80;
    }
    out[4] = b4;

    // Sticks : 12 bits little-endian empaquetés sur 3 octets.
    out[5] = (state.lx & 0xFF) as u8;
    out[6] = (((state.lx >> 8) & 0x0F) as u8) | (((state.ly & 0x0F) as u8) << 4);
    out[7] = ((state.ly >> 4) & 0xFF) as u8;
    out[8] = (state.rx & 0xFF) as u8;
    out[9] = (((state.rx >> 8) & 0x0F) as u8) | (((state.ry & 0x0F) as u8) << 4);
    out[10] = ((state.ry >> 4) & 0xFF) as u8;

    // Octet 11 : vibrator input report (0x0B ~ valeur usuelle au repos).
    out[11] = 0x0B;
    // Octets 12..47 : IMU à zéro (accéléromètre/gyro non implémentés).
}

/// Rapport HID gamepad générique (mode PC) : 16 boutons + 4 axes 8 bits.
/// Correspond au report descriptor de `firmware/src/pc_hid.rs`.
pub const PC_REPORT_LEN: usize = 6;

pub fn pack_pc_report(state: &SwitchState, out: &mut [u8]) {
    assert!(out.len() >= PC_REPORT_LEN);
    // 18 boutons logiques → 16 bits (Home/Capture repliés sur 13/14).
    let mut bits: u16 = (state.buttons & 0xFFFF) as u16;
    if state.buttons & SwitchButton::DpadLeft.mask() != 0 {
        bits |= 1 << 14;
    }
    if state.buttons & SwitchButton::DpadRight.mask() != 0 {
        bits |= 1 << 15;
    }
    out[0] = (bits & 0xFF) as u8;
    out[1] = (bits >> 8) as u8;
    out[2] = (state.lx >> 4) as u8;
    out[3] = (state.ly >> 4) as u8;
    out[4] = (state.rx >> 4) as u8;
    out[5] = (state.ry >> 4) as u8;
}
