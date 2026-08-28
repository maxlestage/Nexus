//! Retour haptique via DRV2605L (I2C) pilotant un moteur ERM ou LRA.
//!
//! - Rumble console : la Switch envoie du "HD rumble" ; on le convertit en
//!   amplitude 0..255 (voir `controller_core::procon::decode_rumble`) et on
//!   pilote le DRV2605 en mode RTP (temps réel).
//! - Clic local : petits effets de la bibliothèque interne du DRV2605
//!   (confirmation d'appui, bascule turbo, identification).

use esp_idf_hal::delay::BLOCK;
use esp_idf_hal::i2c::I2cDriver;

pub const DRV2605_ADDR: u8 = 0x5A;

// Registres du DRV2605.
const REG_STATUS: u8 = 0x00;
const REG_MODE: u8 = 0x01;
const REG_RTP_INPUT: u8 = 0x02;
const REG_LIBRARY: u8 = 0x03;
const REG_WAVESEQ1: u8 = 0x04;
const REG_WAVESEQ2: u8 = 0x05;
const REG_GO: u8 = 0x0C;
const REG_RATED_VOLTAGE: u8 = 0x16;
const REG_OVERDRIVE_CLAMP: u8 = 0x17;
const REG_FEEDBACK: u8 = 0x1A;

const MODE_INTERNAL_TRIGGER: u8 = 0x00;
const MODE_RTP: u8 = 0x05;

/// Quelques effets utiles de la bibliothèque n°1 (ERM).
pub mod effects {
    /// Clic net.
    pub const STRONG_CLICK: u8 = 1;
    /// Clic doux.
    pub const SOFT_CLICK: u8 = 3;
    /// Double clic (bascule turbo ON).
    pub const DOUBLE_CLICK: u8 = 10;
    /// Bourdonnement court (bascule turbo OFF).
    pub const SHORT_BUZZ: u8 = 13;
    /// Rampe montante (identification depuis l'app).
    pub const RAMP_UP: u8 = 82;
}

pub struct Haptics<'d> {
    i2c: I2cDriver<'d>,
    enabled: bool,
    strength: u8,
    last_rtp: u8,
}

impl<'d> Haptics<'d> {
    pub fn new(i2c: I2cDriver<'d>) -> anyhow::Result<Self> {
        let mut h = Self { i2c, enabled: true, strength: 90, last_rtp: 0 };
        h.init()?;
        Ok(h)
    }

    fn write_reg(&mut self, reg: u8, val: u8) -> anyhow::Result<()> {
        self.i2c.write(DRV2605_ADDR, &[reg, val], BLOCK)?;
        Ok(())
    }

    fn init(&mut self) -> anyhow::Result<()> {
        // Sort du standby, moteur ERM, bibliothèque d'effets 1.
        self.write_reg(REG_MODE, MODE_INTERNAL_TRIGGER)?;
        self.write_reg(REG_FEEDBACK, 0x36)?; // ERM, boucle ouverte
        self.write_reg(REG_LIBRARY, 0x01)?;
        self.write_reg(REG_RATED_VOLTAGE, 0x90)?; // ~3 V nominal
        self.write_reg(REG_OVERDRIVE_CLAMP, 0xA4)?;
        let _ = self.i2c.write_read(DRV2605_ADDR, &[REG_STATUS], &mut [0u8; 1], BLOCK);
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if !enabled {
            // Couper le moteur AVANT de désactiver : `rumble` ne fait rien
            // quand `enabled` est déjà à false, et le registre RTP resterait
            // sinon à sa dernière valeur — vibration sans fin.
            let _ = self.rumble(0);
        }
        self.enabled = enabled;
    }

    pub fn set_strength(&mut self, strength: u8) {
        self.strength = strength.min(127);
    }

    /// Joue un effet de la bibliothèque interne (clic, double clic...).
    pub fn play_effect(&mut self, effect: u8) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        self.write_reg(REG_MODE, MODE_INTERNAL_TRIGGER)?;
        self.write_reg(REG_WAVESEQ1, effect & 0x7F)?;
        self.write_reg(REG_WAVESEQ2, 0)?;
        self.write_reg(REG_GO, 1)?;
        Ok(())
    }

    /// Vibration continue proportionnelle (rumble console), 0 = stop.
    pub fn rumble(&mut self, amplitude: u8) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        // Met à l'échelle par la force configurée dans l'app.
        let scaled = (amplitude as u16 * (128 + self.strength as u16) / 255).min(255) as u8;
        if scaled == self.last_rtp {
            return Ok(());
        }
        self.last_rtp = scaled;
        if scaled == 0 {
            self.write_reg(REG_RTP_INPUT, 0)?;
            self.write_reg(REG_MODE, MODE_INTERNAL_TRIGGER)?;
        } else {
            self.write_reg(REG_MODE, MODE_RTP)?;
            self.write_reg(REG_RTP_INPUT, scaled)?;
        }
        Ok(())
    }
}
