//! A small plugin made through the CLAP entry, the way a host
//! loads it: what the kit's CLAP side must do for any plugin.

use kit::{Block, Event, Info, Param, Plugin, Role, level, list, meter, range};
use kit_clap::testing::{Host, Out, conforms};

const GAIN: usize = 0;
const INVERT: usize = 1;

// Adds whatever is on its sidechain, so a test can see it arrive.
// Scales what it hears, passes each note through, meters its output.
struct Gain;

impl Plugin for Gain {
    const INFO: &'static Info = &Info {
        name: "Gain",
        vendor: "kit",
        url: "https://example.invalid",
        description: "Louder or quieter",
        version: "0.1.0",
        clap_id: "dev.kit.gain",
        vst3_processor: [0x1111_0001, 0x2222_0002, 0x3333_0003, 0x4444_0004],
        vst3_controller: [0x5555_0005, 0x6666_0006, 0x7777_0007, 0x8888_0008],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: true,
        midi_out: true,
        size: (480, 300),
        min_size: (320, 200),
    };
    const PARAMS: &'static [Param] = &[
        range(10, "gain", "Gain", "Level", (0.0, 2.0, 1.0), ""),
        list(11, "invert", "Invert", "Level", &["Off", "On"], 0),
    ];
    const METERS: &'static [Param] = &[meter(12, "out", "Output")];
    const KEY_IN: u32 = 2;

    fn new() -> Self {
        Self
    }

    fn prepare(&mut self, _rate: f32, _max_block: usize) {}

    fn process(&mut self, block: &mut Block) {
        let sign = if block.on(INVERT) { -1.0 } else { 1.0 };
        let gain = block.value(GAIN) * sign;
        let mut peak = 0.0;
        for ch in 0..block.outputs() {
            let (input, key, output) = block.keyed_io(ch);
            output
                .iter_mut()
                .zip(input)
                .zip(key)
                .for_each(|((o, i), k)| *o = i * gain + k);
            peak = level::peak(output).max(peak);
        }
        for i in 0..block.events.len() {
            let e = block.events[i];
            block.send(e.offset as usize, e.data);
        }
        block.meter(0, level::to_fraction(peak));
    }
}

#[test]
fn conforms_as_a_clap() {
    conforms::<Gain>();
}

#[test]
fn audio_follows_automation_by_host_id() {
    let mut host = Host::load::<Gain>(48_000.0, 64);
    let ramp: Vec<f32> = (0..200).map(|i| i as f32 / 200.0).collect();

    let plain = host.render(&[ramp.clone(), ramp.clone()], 200, 64);
    assert_eq!(plain[0], ramp, "unity at the default");

    host.set(10, 0.25);
    host.set(11, 1.0);
    let moved = host.render(std::slice::from_ref(&ramp), 200, 64);
    let want: Vec<f32> = ramp.iter().map(|s| s * -0.5).collect();
    assert_eq!(moved[1], want, "a mono input, halved and inverted, on both");
}

#[test]
fn notes_come_back_out_once() {
    let mut host = Host::load::<Gain>(48_000.0, 64);
    host.send(Event {
        offset: 9,
        data: [0x90, 60, 100],
    });
    host.block(64);
    assert_eq!(host.sent(), [Out::Midi(9, [0x90, 60, 100])]);

    host.block(64);
    assert!(host.sent().is_empty(), "the note is not sent twice");
}

#[test]
fn takes_more_than_the_promised_block() {
    // Promised 16, handed 64: in pieces, each read before written.
    let mut host = Host::load::<Gain>(48_000.0, 64);
    let mut small = Host::load_promised::<Gain>(48_000.0, 16, 64);
    let ramp: Vec<f32> = (0..64).map(|i| i as f32).collect();
    host.set(10, 1.0);
    small.set(10, 1.0);

    let want = host.render(std::slice::from_ref(&ramp), 64, 64);
    let got = small.render(std::slice::from_ref(&ramp), 64, 64);
    assert_eq!(got, want);
}

#[test]
fn a_learned_controller_moves_its_setting() {
    let spec = kit::Spec::of::<Gain>();
    let mut host = Host::load::<Gain>(48_000.0, 64);
    let learnt: Vec<Option<u8>> = spec
        .params
        .iter()
        .map(|p| (p.id == 10).then_some(74))
        .collect();
    let saved = kit::state::encode_learnt(&spec, &spec.defaults(), &learnt);
    assert!(host.restore(&saved));
    let before = host.value(10);

    // A controller no setting has learned moves nothing.
    host.send(Event {
        offset: 0,
        data: [0xB0, 20, 127],
    });
    host.block(64);
    assert_eq!(host.value(10), before);

    host.send(Event {
        offset: 0,
        data: [0xB3, 74, 127],
    });
    host.block(64);
    // All the way up: Gain runs to 2, and CLAP speaks in plain values.
    assert_eq!(host.value(10), Some(2.0));
    assert_eq!(kit::state::learnt(&spec, &host.save()), learnt);
}

#[test]
fn a_sidechain_arrives_on_its_own_input_and_is_silent_when_not_fed() {
    let mut host = Host::load::<Gain>(48_000.0, 64);
    let ramp: Vec<f32> = (0..200).map(|i| i as f32 / 200.0).collect();
    let key = vec![0.25_f32; 200];

    // Two channels in and nothing else: the sidechain is silence, and
    // the second input channel is not mistaken for it.
    let alone = host.render(&[ramp.clone(), ramp.clone()], 200, 64);
    assert_eq!(alone[1], ramp);

    // Channels past the plugin's own input are its sidechain.
    let fed = host.render(&[ramp.clone(), ramp.clone(), key.clone(), key], 200, 64);
    let want: Vec<f32> = ramp.iter().map(|s| s + 0.25).collect();
    assert_eq!(fed[0], want);
    assert_eq!(fed[1], want);
}

// Another instance on the machine, listening: what it hears of this
// one's track is what a person linking would be shown.
fn heard_track(of: &str) -> impl Fn() -> Option<String> {
    use std::sync::Arc;
    let spec = kit::Spec::of::<Gain>();
    let shared = Arc::new(kit::Shared::new(&spec));
    let running = kit::link::start(&shared, spec);
    assert!(running.is_some(), "a port to link on");
    let of = of.to_string();
    move || {
        let _held = &running;
        (0..600).find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(5));
            let peers = shared.scope().links.peers();
            peers.into_iter().find(|p| p.track == of).map(|p| p.track)
        })
    }
}

#[test]
fn the_track_a_host_names_is_what_other_instances_hear() {
    let host = Host::load::<Gain>(48_000.0, 64);
    let first = heard_track("Drum Bus");
    let renamed = heard_track("Kit");

    host.name_track("Drum Bus");
    assert_eq!(first().as_deref(), Some("Drum Bus"));
    // And when the track is renamed.
    host.name_track("Kit");
    assert_eq!(renamed().as_deref(), Some("Kit"));
}
