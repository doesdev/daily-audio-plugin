//! What a host saves with a project: every setting by its id, so a
//! later build that adds, drops or reorders settings still loads it.
//!
//! `KIT1`, a count, then that many (id: u32, value: f64) pairs, all
//! little endian. Values are normalized.

use crate::Spec;

const MAGIC: &[u8; 4] = b"KIT1";
const PAIR: usize = 12;

pub fn encode(spec: &Spec, values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + spec.params.len() * PAIR);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(spec.params.len() as u32).to_le_bytes());
    for (p, value) in spec.params.iter().zip(values) {
        out.extend_from_slice(&p.id.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// Every setting as `bytes` has it: one the state does not mention is
/// at its default, and one this build does not know is ignored. `None`
/// if the bytes are not a state at all.
pub fn decode(spec: &Spec, bytes: &[u8]) -> Option<Vec<f64>> {
    let rest = bytes.strip_prefix(MAGIC)?;
    let (count, rest) = rest.split_at_checked(4)?;
    let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
    let pairs = rest.get(..count.checked_mul(PAIR)?)?;

    let mut values = spec.defaults();
    for pair in pairs.as_chunks::<PAIR>().0 {
        let id = u32::from_le_bytes(pair[..4].try_into().ok()?);
        let value = f64::from_le_bytes(pair[4..].try_into().ok()?);
        if let Some(index) = spec.setting_of(id).filter(|_| value.is_finite()) {
            values[index] = value.clamp(0.0, 1.0);
        }
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Info, Param, Role, list, range};

    const INFO: Info = Info {
        name: "",
        vendor: "",
        url: "",
        description: "",
        version: "",
        clap_id: "a.b",
        vst3_processor: [0; 4],
        vst3_controller: [1; 4],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (1, 1),
        min_size: (1, 1),
    };
    const MIX: Param = range(0, "mix", "Mix", "", (0.0, 1.0, 0.5), "");
    const MODE: Param = list(7, "mode", "Mode", "", &["A", "B", "C"], 1);
    const TONE: Param = range(3, "tone", "Tone", "", (0.0, 1.0, 0.25), "");

    fn spec(params: &'static [Param]) -> Spec {
        Spec {
            info: &INFO,
            params,
            meters: &[],
            assets: &[],
        }
    }

    #[test]
    fn round_trips() {
        let spec = spec(&[MIX, MODE]);
        let bytes = encode(&spec, &[0.8, 1.0]);

        assert_eq!(decode(&spec, &bytes), Some(vec![0.8, 1.0]));
    }

    #[test]
    fn a_later_build_reads_an_older_state() {
        let old = encode(&spec(&[MIX, MODE]), &[0.8, 1.0]);
        // Reordered, one added, one dropped.
        let cases: [(&'static [Param], Vec<f64>); 3] = [
            (&[MODE, MIX], vec![1.0, 0.8]),
            (&[MIX, MODE, TONE], vec![0.8, 1.0, 0.25]),
            (&[TONE, MIX], vec![0.25, 0.8]),
        ];

        for (params, want) in cases {
            assert_eq!(decode(&spec(params), &old), Some(want));
        }
    }

    #[test]
    fn refuses_what_is_not_a_state_and_bounds_what_is() {
        let spec = spec(&[MIX]);
        let mut wild = encode(&spec, &[7.0]);

        assert_eq!(decode(&spec, &wild), Some(vec![1.0]), "clamped");
        wild[8..12].copy_from_slice(&0u32.to_le_bytes());
        wild[12..].copy_from_slice(&f64::NAN.to_le_bytes());
        assert_eq!(decode(&spec, &wild), Some(vec![0.5]), "not a number");
        for bytes in [&b""[..], b"KIT", b"NOPE\0\0\0\0", &wild[..10]] {
            assert_eq!(decode(&spec, bytes), None, "{bytes:?}");
        }
    }
}
