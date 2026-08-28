//! Protocole applicatif du Pro Controller (par-dessus HID) : réponses aux
//! "subcommands" que la Switch envoie pendant l'appairage et le jeu.
//!
//! Basé sur le reverse engineering communautaire
//! (dekuNukem/Nintendo_Switch_Reverse_Engineering). Tout est en fonctions
//! pures pour être testable sans Bluetooth.

use crate::report::{pack_standard_report, SwitchState};

/// Adresse MAC annoncée dans les réponses (renseignée par le firmware).
#[derive(Debug, Clone, Copy)]
pub struct ProconIdentity {
    pub mac: [u8; 6],
}

/// Événements extraits d'un rapport de sortie de la console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HostEffects {
    /// Amplitude de vibration 0..=255 décodée du rumble HD (approximation
    /// suffisante pour piloter un DRV2605).
    pub rumble_amplitude: Option<u8>,
    /// Numéro de joueur reçu via "player lights" (1..=4).
    pub player_number: Option<u8>,
}

/// Réponse à renvoyer à la console.
pub struct Reply {
    pub data: [u8; 64],
    pub len: usize,
    /// ID de rapport HID (0x21 pour les acks de subcommand).
    pub report_id: u8,
}

pub struct ProconProtocol {
    identity: ProconIdentity,
    /// Mode de rapport demandé par la console (0x30 = full à 60 Hz).
    pub input_mode: u8,
    pub imu_enabled: bool,
    pub vibration_enabled: bool,
}

impl ProconProtocol {
    pub fn new(identity: ProconIdentity) -> Self {
        Self {
            identity,
            input_mode: 0x3F,
            imu_enabled: false,
            vibration_enabled: false,
        }
    }

    /// Traite un rapport de sortie reçu de la Switch.
    ///
    /// `report` commence à l'ID de rapport (0x01 = rumble + subcommand,
    /// 0x10 = rumble seul). Retourne l'éventuelle réponse et les effets
    /// (rumble, lights) à appliquer côté matériel.
    pub fn handle_output_report(
        &mut self,
        report: &[u8],
        state: &SwitchState,
        timer: u8,
        battery: u8,
    ) -> (Option<Reply>, HostEffects) {
        let mut fx = HostEffects::default();
        if report.is_empty() {
            return (None, fx);
        }
        match report[0] {
            0x10 => {
                // Rumble seul : octets 2..=9 (2 × 4 octets HD rumble).
                if report.len() >= 10 {
                    fx.rumble_amplitude = Some(decode_rumble(&report[2..10]));
                }
                (None, fx)
            }
            0x01 => {
                if report.len() >= 10 {
                    fx.rumble_amplitude = Some(decode_rumble(&report[2..10]));
                }
                if report.len() < 11 {
                    return (None, fx);
                }
                let subcmd = report[10];
                let args = &report[11..];
                let reply = self.handle_subcommand(subcmd, args, state, timer, battery, &mut fx);
                (Some(reply), fx)
            }
            _ => (None, fx),
        }
    }

    fn handle_subcommand(
        &mut self,
        subcmd: u8,
        args: &[u8],
        state: &SwitchState,
        timer: u8,
        battery: u8,
        fx: &mut HostEffects,
    ) -> Reply {
        let mut r = Reply {
            data: [0u8; 64],
            len: 49,
            report_id: 0x21,
        };
        // Préambule : timer + batterie + état courant des boutons/sticks.
        let mut std_report = [0u8; 48];
        pack_standard_report(state, timer, battery, &mut std_report);
        r.data[..12].copy_from_slice(&std_report[..12]);

        // ACK par défaut : 0x80 | présence de données, puis l'ID du subcommand.
        r.data[13] = subcmd;

        match subcmd {
            // Get device info.
            0x02 => {
                r.data[12] = 0x82;
                let m = &self.identity.mac;
                let info: [u8; 12] = [
                    0x04, 0x21, // version firmware
                    0x03, // type : Pro Controller
                    0x02, // constante
                    m[5], m[4], m[3], m[2], m[1], m[0], // MAC inversée
                    0x03, // constante
                    0x02, // couleurs : stockées en SPI
                ];
                r.data[14..14 + info.len()].copy_from_slice(&info);
            }
            // Set input report mode.
            0x03 => {
                if let Some(&mode) = args.first() {
                    self.input_mode = mode;
                }
                r.data[12] = 0x80;
            }
            // Trigger buttons elapsed time.
            0x04 => {
                r.data[12] = 0x83;
            }
            // Set shipment low power state.
            0x08 => {
                r.data[12] = 0x80;
            }
            // SPI flash read : calibration usine, couleurs, etc.
            0x10 => {
                if args.len() >= 5 {
                    let addr = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                    let len = (args[4] as usize).min(0x1D);
                    r.data[12] = 0x90;
                    r.data[14..19].copy_from_slice(&args[..5]);
                    spi_flash_read(addr, &mut r.data[19..19 + len]);
                } else {
                    r.data[12] = 0x80;
                }
            }
            // Set NFC/IR MCU config / state.
            0x21 | 0x22 => {
                r.data[12] = 0xA0;
            }
            // Set player lights.
            0x30 => {
                if let Some(&bits) = args.first() {
                    // bits 0..3 = LEDs joueur fixes.
                    fx.player_number = Some(match bits & 0x0F {
                        0x01 => 1,
                        0x03 => 2,
                        0x07 => 3,
                        0x0F => 4,
                        _ => 1,
                    });
                }
                r.data[12] = 0x80;
            }
            // Home light.
            0x38 => {
                r.data[12] = 0x80;
            }
            // Enable IMU.
            0x40 => {
                self.imu_enabled = args.first().copied().unwrap_or(0) != 0;
                r.data[12] = 0x80;
            }
            // Enable vibration.
            0x48 => {
                self.vibration_enabled = args.first().copied().unwrap_or(0) != 0;
                r.data[12] = 0x80;
            }
            // Subcommand inconnu : ack simple.
            _ => {
                r.data[12] = 0x80;
            }
        }
        r
    }
}

/// Décodage approché de l'amplitude HD rumble : la Switch encode
/// fréquence + amplitude sur 4 octets par actionneur ; l'octet 3 (amFm)
/// porte l'amplitude principale. `[0x00, 0x01, 0x40, 0x40]` = neutre.
fn decode_rumble(rumble: &[u8]) -> u8 {
    if rumble.len() < 8 {
        return 0;
    }
    let amp_of = |quad: &[u8]| -> u8 {
        // Neutre exact → 0.
        if quad == [0x00, 0x01, 0x40, 0x40] {
            return 0;
        }
        // L'amplitude haute est encodée dans l'octet 1 (bits 1..), la basse
        // dans l'octet 3 ; on prend le max, remis grossièrement sur 0..255.
        let hi = quad[1].saturating_sub(0x01);
        let lo = if quad[3] >= 0x40 {
            (quad[3] - 0x40).saturating_mul(2)
        } else {
            0
        };
        hi.max(lo)
    };
    let left = amp_of(&rumble[0..4]);
    let right = amp_of(&rumble[4..8]);
    let m = left.max(right);
    // Étale sur 0..255 (l'encodage brut plafonne vers ~0xC8).
    m.saturating_mul(2)
}

/// Émule la lecture de la flash SPI du Pro Controller : on sert des
/// données de calibration d'usine neutres et "pas de calibration
/// utilisateur" (0xFF).
fn spi_flash_read(addr: u32, out: &mut [u8]) {
    for b in out.iter_mut() {
        *b = 0xFF;
    }
    match addr {
        // Numéro de série : 0xFF = absent.
        0x6000..=0x600F => {}
        // Calibration usine IMU.
        0x6020..=0x603C => {
            for b in out.iter_mut() {
                *b = 0;
            }
        }
        // Calibration usine des sticks : centre 0x800, course ±0x600.
        // 9 octets par stick : [max_dx, centre, min_dx] empaquetés 12 bits.
        0x603D => {
            const CAL: [u8; 18] = [
                // Stick gauche : above_center(0x600,0x600), center(0x800,0x800), below(0x600,0x600)
                0x00, 0x06, 0x60, 0x00, 0x08, 0x80, 0x00, 0x06, 0x60,
                // Stick droit : center, below, above (ordre différent côté droit)
                0x00, 0x08, 0x80, 0x00, 0x06, 0x60, 0x00, 0x06, 0x60,
            ];
            let n = out.len().min(CAL.len());
            out[..n].copy_from_slice(&CAL[..n]);
        }
        // Couleurs de la manette (corps, boutons) : gris/cyan.
        0x6050..=0x605C => {
            let colors: [u8; 12] = [
                0x32, 0x32, 0x32, // corps
                0x00, 0xB7, 0xEB, // boutons
                0xFF, 0xFF, 0xFF, // poignée gauche
                0xFF, 0xFF, 0xFF, // poignée droite
            ];
            let off = (addr - 0x6050) as usize;
            for (i, b) in out.iter_mut().enumerate() {
                if off + i < colors.len() {
                    *b = colors[off + i];
                }
            }
        }
        // Paramètres sticks (deadzone usine...).
        0x6080..=0x6097 => {
            for b in out.iter_mut() {
                *b = 0;
            }
        }
        // Calibration utilisateur : 0xFF partout = absente (déjà rempli).
        0x8010..=0x8040 => {}
        _ => {}
    }
}
