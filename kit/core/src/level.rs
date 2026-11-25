//! Loudness, as gain, as decibels and as the fraction a meter shows.

/// The quietest a meter shows; anything under reads as silent.
pub const FLOOR_DB: f32 = -60.0;

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub fn gain_to_db(gain: f32) -> f32 {
    20.0 * gain.max(1e-9).log10()
}

/// A linear peak as a meter's fraction: 0 at the floor, 1 at full scale.
pub fn to_fraction(peak: f32) -> f64 {
    ((gain_to_db(peak) - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0) as f64
}

/// The decibels a meter's fraction stands for.
pub fn fraction_to_db(fraction: f64) -> f32 {
    FLOOR_DB + fraction.clamp(0.0, 1.0) as f32 * -FLOOR_DB
}

/// The loudest sample of a block.
pub fn peak(block: &[f32]) -> f32 {
    block.iter().fold(0.0, |m, s| m.max(s.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_and_decibels_round_trip() {
        for db in [-60.0, -18.0, -6.0, 0.0, 6.0] {
            let back = gain_to_db(db_to_gain(db));
            assert!((back - db).abs() < 1e-4, "{db}: {back}");
        }
    }

    #[test]
    fn a_meter_runs_from_the_floor_to_full_scale() {
        let cases = [(0.0, 0.0), (1.0, 1.0), (2.0, 1.0), (0.001, 0.0)];

        for (peak, want) in cases {
            assert!((to_fraction(peak) - want).abs() < 1e-6, "{peak}");
        }
        assert!((to_fraction(db_to_gain(-30.0)) - 0.5).abs() < 1e-4);
        assert!((fraction_to_db(0.5) - -30.0).abs() < 1e-4);
    }
}
