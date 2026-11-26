//! A small plugin loaded through its VST3 factory, the way a host
//! loads it: what the kit's VST3 side must do for any plugin.

use kit::{Block, Event, Info, Param, Plugin, Role, level, list, meter, range};
use kit_vst3::testing::{Host, conforms};

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
fn conforms_as_a_vst3() {
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
fn notes_and_meters_come_back_out() {
    let mut host = Host::load::<Gain>(48_000.0, 64);
    (0..2).for_each(|ch| host.input(ch).fill(0.5));
    host.send(Event {
        offset: 9,
        data: [0x90, 60, 100],
    });
    host.block(64);

    assert_eq!(host.sent().iter().map(|e| e.1).collect::<Vec<_>>(), [9]);
    let reported = host.reported();
    assert_eq!(reported.len(), 1);
    let (id, value) = reported[0];
    assert_eq!(id, 12);
    assert!((value - level::to_fraction(0.5)).abs() < 1e-9, "{value}");

    host.block(64);
    assert!(host.sent().is_empty(), "the note is not sent twice");
    assert!(host.reported().is_empty(), "a meter that has not moved");
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
fn a_learned_controller_moves_its_setting_and_the_host_is_told() {
    let spec = kit::Spec::of::<Gain>();
    let mut host = Host::load::<Gain>(48_000.0, 64);
    // Gain (id 10) learned controller 74 in some earlier session.
    let learnt: Vec<Option<u8>> = spec
        .params
        .iter()
        .map(|p| (p.id == 10).then_some(74))
        .collect();
    let saved = kit::state::encode_learnt(&spec, &spec.defaults(), &learnt);
    assert!(host.restore(&saved));

    // A controller no setting has learned moves nothing.
    host.set(kit_vst3::CC_PARAMS + 20, 1.0);
    host.block(64);
    assert!(host.reported().iter().all(|&(id, _)| id != 10));

    // The host turns controller 74 into its stand-in parameter; the
    // setting follows, and the host hears where it went.
    host.set(kit_vst3::CC_PARAMS + 74, 1.0);
    host.block(64);
    assert!(
        host.reported().contains(&(10, 1.0)),
        "{:?}",
        host.reported()
    );

    // And what was learned is saved again.
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
