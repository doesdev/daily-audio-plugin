//! The host's transport from a VST3 ProcessContext. Each field counts
//! only when the host marks it valid.

use kit::Transport;
use vst3::Steinberg::Vst::IProcessContextRequirements_::Flags_ as Need;
use vst3::Steinberg::Vst::ProcessContext;
use vst3::Steinberg::Vst::ProcessContext_::StatesAndFlags_ as State;

/// What getProcessContextRequirements asks the host for.
pub const NEEDS: u32 =
    (Need::kNeedProjectTimeMusic | Need::kNeedTempo | Need::kNeedTransportState) as u32;

pub fn read(ctx: Option<&ProcessContext>) -> Transport {
    let Some(ctx) = ctx else {
        return Transport::default();
    };

    let has = |flag: u32| ctx.state & flag != 0;
    let playing = has(State::kPlaying as u32);
    Transport {
        playing,
        tempo: has(State::kTempoValid as u32)
            .then_some(ctx.tempo)
            .filter(|t| t.is_finite() && *t > 0.0),
        beats: has(State::kProjectTimeMusicValid as u32)
            .then_some(ctx.projectTimeMusic)
            .filter(|b| b.is_finite()),
        sample: playing.then_some(ctx.projectTimeSamples),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(state: u32) -> ProcessContext {
        let mut ctx: ProcessContext = unsafe { std::mem::zeroed() };
        ctx.state = state;
        ctx.projectTimeSamples = 456_000;
        ctx.projectTimeMusic = 19.0;
        ctx.tempo = 120.0;
        ctx
    }

    #[test]
    fn reads_what_the_host_marks_valid() {
        let all = (State::kTempoValid | State::kProjectTimeMusicValid) as u32;
        let rolling = Transport {
            playing: true,
            tempo: Some(120.0),
            beats: Some(19.0),
            sample: Some(456_000),
        };
        let stopped = Transport {
            playing: false,
            sample: None,
            ..rolling
        };
        let cases = [
            (
                "rolling",
                Some(context(all | State::kPlaying as u32)),
                rolling,
            ),
            ("stopped", Some(context(all)), stopped),
            ("nothing valid", Some(context(0)), Transport::default()),
            ("no context", None, Transport::default()),
        ];

        for (name, ctx, want) in cases {
            assert_eq!(read(ctx.as_ref()), want, "{name}");
        }
    }
}
