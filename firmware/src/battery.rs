//! Surveillance batterie : pont diviseur (2 × 100 kΩ) entre B+ et GPIO36
//! (ADC1_CH0). La tension lue est la moitié de la tension batterie ; l'état
//! « en charge » est déduit de la tension (> 4,25 V), la broche CHRG du
//! TP4056 n'est pas câblée.

/// Convertit une lecture ADC (atténuation 11 dB, 0..=4095 ≈ 0..=3,55 V)
/// en millivolts batterie (× 2 à cause du pont diviseur).
pub fn adc_to_battery_mv(raw: u16) -> u16 {
    let pin_mv = raw as u32 * 3550 / 4095;
    (pin_mv * 2) as u16
}

/// Courbe de décharge Li-ion approchée → pourcentage 0..=100.
pub fn mv_to_percent(mv: u16) -> u8 {
    const TABLE: [(u16, u8); 9] = [
        (4200, 100),
        (4050, 90),
        (3950, 80),
        (3850, 65),
        (3780, 50),
        (3700, 35),
        (3600, 20),
        (3450, 8),
        (3300, 0),
    ];
    if mv >= TABLE[0].0 {
        return 100;
    }
    for w in TABLE.windows(2) {
        let (hi_mv, hi_pct) = w[0];
        let (lo_mv, lo_pct) = w[1];
        if mv >= lo_mv {
            let span = (hi_mv - lo_mv) as u32;
            let d = (mv - lo_mv) as u32;
            return (lo_pct as u32 + d * (hi_pct - lo_pct) as u32 / span) as u8;
        }
    }
    0
}

/// Niveau batterie au format Pro Controller (0, 2, 4, 6, 8).
pub fn percent_to_procon_level(percent: u8) -> u8 {
    match percent {
        0..=10 => 0,
        11..=30 => 2,
        31..=55 => 4,
        56..=80 => 6,
        _ => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_monotonic() {
        assert_eq!(mv_to_percent(4200), 100);
        assert_eq!(mv_to_percent(3300), 0);
        assert!(mv_to_percent(3800) > mv_to_percent(3600));
    }
}
