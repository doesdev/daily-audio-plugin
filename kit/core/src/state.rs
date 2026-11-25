//! What a host saves with a project: every setting by its id, so a
//! later build that adds, drops or reorders settings still loads it.
//!
//! `KIT1`, a count, then that many (id: u32, value: f64) pairs, all
//! little endian. Values are normalized.
//!
//! A setting that has learned a MIDI controller has a second pair: its
//! id with `LEARNT` set, and the controller's number as the value. A
//! build from before MIDI learn reads past those as ids it does not
//! know.

use crate::Spec;

const MAGIC: &[u8; 4] = b"KIT1";
const PAIR: usize = 12;

/// Set in the id of a pair that holds a setting's controller. No
/// setting's own id may have it.
pub const LEARNT: u32 = 0x4C00_0000;

pub fn encode(spec: &Spec, values: &[f64]) -> Vec<u8> {
    encode_learnt(spec, values, &[])
}

/// As `encode`, with the controller each setting has learned.
pub fn encode_learnt(spec: &Spec, values: &[f64], ccs: &[Option<u8>]) -> Vec<u8> {
    let learnt = spec
        .params
        .iter()
        .zip(ccs)
        .filter_map(|(p, cc)| Some((p.id | LEARNT, f64::from((*cc)?))));
    let pairs: Vec<(u32, f64)> = spec
        .params
        .iter()
        .zip(values)
        .map(|(p, &value)| (p.id, value))
        .chain(learnt)
        .collect();

    let mut out = Vec::with_capacity(8 + pairs.len() * PAIR);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    for (id, value) in pairs {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// The controller each setting had learned when `bytes` was saved:
/// none, for a state that is not one or says nothing of it.
pub fn learnt(spec: &Spec, bytes: &[u8]) -> Vec<Option<u8>> {
    let mut ccs = vec![None; spec.params.len()];
    let pairs = bytes
        .strip_prefix(MAGIC)
        .and_then(|rest| rest.get(4..))
        .unwrap_or(&[]);
    for pair in pairs.as_chunks::<PAIR>().0 {
        let id = u32::from_le_bytes([pair[0], pair[1], pair[2], pair[3]]);
        let mut value = [0; 8];
        value.copy_from_slice(&pair[4..]);
        let cc = f64::from_le_bytes(value);
        let index = Some(id)
            .filter(|id| id & LEARNT != 0)
            .and_then(|id| spec.setting_of(id & !LEARNT));
        if let Some(index) = index.filter(|_| (0.0..128.0).contains(&cc)) {
            ccs[index] = Some(cc as u8);
        }
    }
    ccs
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

    #[test]
    fn what_a_setting_has_learned_is_saved_with_it_and_comes_back() {
        let spec = spec(&[MIX, MODE, TONE]);
        let bytes = encode_learnt(&spec, &[0.8, 1.0, 0.1], &[Some(74), None, Some(0)]);

        assert_eq!(learnt(&spec, &bytes), [Some(74), None, Some(0)]);
        // The settings themselves read as they always did.
        assert_eq!(decode(&spec, &bytes), Some(vec![0.8, 1.0, 0.1]));
    }

    #[test]
    fn a_state_from_before_learning_has_learned_nothing() {
        let spec = spec(&[MIX, MODE]);

        assert_eq!(learnt(&spec, &encode(&spec, &[0.8, 1.0])), [None, None]);
        assert_eq!(learnt(&spec, b"junk"), [None, None]);
        assert_eq!(learnt(&spec, b""), [None, None]);
    }

    #[test]
    fn a_controller_moves_the_setting_that_learned_it_as_a_gesture() {
        use crate::{BEGIN, END, SET, Shared};
        let spec = spec(&[MIX, MODE]);
        let shared = Shared::new(&spec);

        // Nothing has learned it: nothing moves.
        shared.controller(74, 127);
        assert_eq!(shared.value(0), Some(0.5));

        shared.scope().learn.listen(Some(0));
        shared.controller(74, 127);
        assert_eq!(shared.value(0), Some(1.0));
        assert_eq!(shared.take_edit(0), BEGIN | SET | END);
        shared.controller(74, 0);
        assert_eq!(shared.value(0), Some(0.0));

        // And it is what gets saved, and put back.
        assert_eq!(shared.learnt(), [Some(74), None]);
        let other = Shared::new(&spec);
        other.set_learnt(&shared.learnt());
        other.controller(74, 127);
        assert_eq!(other.value(0), Some(1.0));
    }
}
