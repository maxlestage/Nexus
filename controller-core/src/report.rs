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

/// Rapport HID gamepad générique (mode PC) : 20 boutons (18 utilisés,
/// 2 de bourrage) + 4 axes 8 bits. Correspond au report descriptor de
/// `firmware/src/bt/pc_hid.rs`.
pub const PC_REPORT_LEN: usize = 7;

pub fn pack_pc_report(state: &SwitchState, out: &mut [u8]) {
    assert!(out.len() >= PC_REPORT_LEN);
    // Les 18 boutons logiques passent tels quels : le bouton HID n de l'hôte
    // correspond au `SwitchButton` d'indice n-1, croix comprise (bits 14..17).
    let bits = state.buttons & 0x3FFFF;
    out[0] = (bits & 0xFF) as u8;
    out[1] = ((bits >> 8) & 0xFF) as u8;
    out[2] = ((bits >> 16) & 0x03) as u8;
    out[3] = (state.lx >> 4) as u8;
    out[4] = (state.ly >> 4) as u8;
    out[5] = (state.rx >> 4) as u8;
    out[6] = (state.ry >> 4) as u8;
}

/// Rapport d'entrée court 0x3F (11 octets), envoyé tant que la console n'a
/// pas demandé le mode 0x30 : l'écran « Changer le style/l'ordre » s'en sert
/// pour afficher les appuis pendant l'appairage.
///
/// Format (dekuNukem) : 2 octets de boutons, 1 octet de hat, puis les deux
/// sticks en 16 bits little-endian.
pub const SHORT_REPORT_LEN: usize = 11;

pub fn pack_short_report(state: &SwitchState, out: &mut [u8]) {
    assert!(out.len() >= SHORT_REPORT_LEN);
    for b in out[..SHORT_REPORT_LEN].iter_mut() {
        *b = 0;
    }

    let mut b0 = 0u8;
    if state.has(SwitchButton::B) {
        b0 |= 0x01;
    }
    if state.has(SwitchButton::A) {
        b0 |= 0x02;
    }
    if state.has(SwitchButton::Y) {
        b0 |= 0x04;
    }
    if state.has(SwitchButton::X) {
        b0 |= 0x08;
    }
    if state.has(SwitchButton::L) {
        b0 |= 0x10;
    }
    if state.has(SwitchButton::R) {
        b0 |= 0x20;
    }
    if state.has(SwitchButton::Zl) {
        b0 |= 0x40;
    }
    if state.has(SwitchButton::Zr) {
        b0 |= 0x80;
    }
    out[0] = b0;

    let mut b1 = 0u8;
    if state.has(SwitchButton::Minus) {
        b1 |= 0x01;
    }
    if state.has(SwitchButton::Plus) {
        b1 |= 0x02;
    }
    if state.has(SwitchButton::LStick) {
        b1 |= 0x04;
    }
    if state.has(SwitchButton::RStick) {
        b1 |= 0x08;
    }
    if state.has(SwitchButton::Home) {
        b1 |= 0x10;
    }
    if state.has(SwitchButton::Capture) {
        b1 |= 0x20;
    }
    out[1] = b1;

    // Hat : 0 = haut, tourne dans le sens horaire, 8 = repos.
    let up = state.has(SwitchButton::DpadUp);
    let down = state.has(SwitchButton::DpadDown);
    let left = state.has(SwitchButton::DpadLeft);
    let right = state.has(SwitchButton::DpadRight);
    out[2] = match (up, right, down, left) {
        (true, false, false, false) => 0,
        (true, true, false, false) => 1,
        (false, true, false, false) => 2,
        (false, true, true, false) => 3,
        (false, false, true, false) => 4,
        (false, false, true, true) => 5,
        (false, false, false, true) => 6,
        (true, false, false, true) => 7,
        _ => 8,
    };

    // Sticks 12 bits recadrés sur 16 bits.
    let lx = state.lx << 4;
    let ly = state.ly << 4;
    let rx = state.rx << 4;
    let ry = state.ry << 4;
    out[3..5].copy_from_slice(&lx.to_le_bytes());
    out[5..7].copy_from_slice(&ly.to_le_bytes());
    out[7..9].copy_from_slice(&rx.to_le_bytes());
    out[9..11].copy_from_slice(&ry.to_le_bytes());
}
