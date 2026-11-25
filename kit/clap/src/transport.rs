//! The host's transport from a CLAP process call, as it stands at the
//! block's first sample. Each field counts only when its flag says the
//! host has it.

use clap_sys::events::*;
use clap_sys::fixedpoint::{CLAP_BEATTIME_FACTOR, CLAP_SECTIME_FACTOR};
use kit::Transport;

pub fn read(t: Option<&clap_event_transport>, rate: f32) -> Transport {
    let Some(t) = t else {
        return Transport::default();
    };

    let has = |flag: u32| t.flags & flag != 0;
    let playing = has(CLAP_TRANSPORT_IS_PLAYING);
    let seconds = t.song_pos_seconds as f64 / CLAP_SECTIME_FACTOR as f64;
    Transport {
        playing,
        tempo: has(CLAP_TRANSPORT_HAS_TEMPO)
            .then_some(t.tempo)
            .filter(|t| t.is_finite() && *t > 0.0),
        beats: has(CLAP_TRANSPORT_HAS_BEATS_TIMELINE)
            .then_some(t.song_pos_beats as f64 / CLAP_BEATTIME_FACTOR as f64),
        sample: (playing && has(CLAP_TRANSPORT_HAS_SECONDS_TIMELINE))
            .then(|| (seconds * rate as f64).round() as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transport(flags: u32) -> clap_event_transport {
        let mut t: clap_event_transport = unsafe { std::mem::zeroed() };
        t.flags = flags;
        t.tempo = 90.0;
        t.song_pos_beats = (13.5 * CLAP_BEATTIME_FACTOR as f64) as i64;
        t.song_pos_seconds = 9 * CLAP_SECTIME_FACTOR;
        t
    }

    #[test]
    fn reads_what_the_host_says_it_has() {
        let all = CLAP_TRANSPORT_HAS_TEMPO
            | CLAP_TRANSPORT_HAS_BEATS_TIMELINE
            | CLAP_TRANSPORT_HAS_SECONDS_TIMELINE;
        let rolling = Transport {
            playing: true,
            tempo: Some(90.0),
            beats: Some(13.5),
            sample: Some(432_000),
        };
        let stopped = Transport {
            playing: false,
            sample: None,
            ..rolling
        };
        let cases = [
            (
                "rolling",
                Some(transport(all | CLAP_TRANSPORT_IS_PLAYING)),
                rolling,
            ),
            ("stopped", Some(transport(all)), stopped),
            ("nothing", Some(transport(0)), Transport::default()),
            ("no transport", None, Transport::default()),
        ];

        for (name, t, want) in cases {
            assert_eq!(read(t.as_ref(), 48_000.0), want, "{name}");
        }
    }
}
