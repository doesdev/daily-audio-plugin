//! MIDI 1.0 bytes as VST3 events. Notes become note events; CC and pitch
//! bend become legacy MIDI CC out events, VST3's only way to send bend.
//! A keyboard's notes come back the other way; VST3 hands controllers
//! in as parameters, not events, so only notes come in here.

use kit::Event as Midi;
use vst3::Steinberg::Vst::{
    ControllerNumbers_, Event, Event_, Event__type0, LegacyMIDICCOutEvent, NoteOffEvent,
    NoteOnEvent,
};

// The host assigns note ids; -1 says this event has none of its own.
const NO_NOTE_ID: i32 = -1;

pub fn to_event(midi: Midi) -> Option<Event> {
    let [status, a, b] = midi.data;
    let channel = (status & 0x0F) as i16;

    let (kind, body) = match status & 0xF0 {
        0x90 => (
            Event_::EventTypes_::kNoteOnEvent,
            Event__type0 {
                noteOn: NoteOnEvent {
                    channel,
                    pitch: a as i16,
                    tuning: 0.0,
                    velocity: b as f32 / 127.0,
                    length: 0,
                    noteId: NO_NOTE_ID,
                },
            },
        ),
        0x80 => (
            Event_::EventTypes_::kNoteOffEvent,
            Event__type0 {
                noteOff: NoteOffEvent {
                    channel,
                    pitch: a as i16,
                    velocity: 0.0,
                    noteId: NO_NOTE_ID,
                    tuning: 0.0,
                },
            },
        ),
        0xB0 => cc_out(a, channel, b, 0),
        0xE0 => cc_out(ControllerNumbers_::kPitchBend as u8, channel, a, b),
        _ => return None,
    };

    Some(Event {
        busIndex: 0,
        sampleOffset: midi.offset as i32,
        ppqPosition: 0.0,
        flags: Event_::EventFlags_::kIsLive as u16,
        r#type: kind as u16,
        __field0: body,
    })
}

/// A note event from the host's MIDI in as MIDI 1.0 bytes at its
/// sample offset; `None` for anything else.
pub fn from_event(ev: &Event) -> Option<Midi> {
    let offset = ev.sampleOffset.max(0) as u32;
    let seven = |v: f32| (v.clamp(0.0, 1.0) * 127.0).round() as u8;
    let data = unsafe {
        match ev.r#type as Event_::EventTypes {
            Event_::EventTypes_::kNoteOnEvent => {
                let n = ev.__field0.noteOn;
                // A velocity of 0 would read as an end.
                [
                    0x90 | (n.channel & 0x0F) as u8,
                    n.pitch.clamp(0, 127) as u8,
                    seven(n.velocity).max(1),
                ]
            }
            Event_::EventTypes_::kNoteOffEvent => {
                let n = ev.__field0.noteOff;
                [
                    0x80 | (n.channel & 0x0F) as u8,
                    n.pitch.clamp(0, 127) as u8,
                    0,
                ]
            }
            _ => return None,
        }
    };
    Some(Midi { offset, data })
}

fn cc_out(control: u8, channel: i16, value: u8, value2: u8) -> (Event_::EventTypes, Event__type0) {
    (
        Event_::EventTypes_::kLegacyMIDICCOutEvent,
        Event__type0 {
            midiCCOut: LegacyMIDICCOutEvent {
                controlNumber: control,
                channel: channel as i8,
                value: value as i8,
                value2: value2 as i8,
            },
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_round_trip() {
        let cases = [
            ([0x90, 60, 100], Some([0x90, 60, 100])),
            ([0x93, 72, 1], Some([0x93, 72, 1])),
            ([0x80, 60, 0], Some([0x80, 60, 0])),
            // A controller goes out, but does not come back as a note.
            ([0xB0, 1, 64], None),
            ([0xE0, 0, 64], None),
        ];

        for (data, want) in cases {
            let ev = to_event(Midi { offset: 7, data }).expect("an event");
            assert_eq!(ev.sampleOffset, 7);
            let back = from_event(&ev);
            assert_eq!(back.map(|m| m.data), want, "{data:?}");
            assert!(back.is_none_or(|m| m.offset == 7));
        }
        let none = to_event(Midi {
            offset: 0,
            data: [0xF8, 0, 0],
        });
        assert!(none.is_none(), "a clock tick is not sent");
    }
}
