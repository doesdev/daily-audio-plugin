//! Linking: a setting in one instance moves with the same setting in
//! others, across every DAP plugin a person has open. Bypass four
//! saturators together, or ride the Mix of six at once.
//!
//! Instances are separate libraries, and in some hosts separate
//! processes, so they have no memory in common. They talk over UDP on
//! this machine alone: each takes the first free port of a short
//! range, says once a second who it is and which settings it has, and
//! tells the others of a group when one of its linked settings moves.
//! Nothing here touches the audio thread: a thread of its own watches
//! `Shared` for moves and writes what it hears back into it, as an
//! editor would, so the host is told in the way it already is.
//!
//! A link is a group: a number two instances hold for the same key.
//! A setting with group 0 is linked to nothing. Groups are saved with
//! the project, so links come back with it, and a duplicated track is
//! linked as the one it was copied from is.

use crate::{BEGIN, END, SET, Shared};
use crate::{Kind, Spec};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How many settings can each be linked.
pub const LINKABLE: usize = 256;
/// The ports instances take, one each: so at most this many at once.
pub const FIRST_PORT: u16 = 47_310;
pub const PORTS: u16 = 64;

const MAGIC: &str = "DAP1";
// How often each says it is here, and how long one unheard is kept.
const SAYS: Duration = Duration::from_millis(1000);
const KEPT: Duration = Duration::from_millis(3500);
// How often moves are looked for: a linked control lags by this.
const TICK: Duration = Duration::from_millis(8);

/// Another instance, as it last described itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Peer {
    pub uid: u64,
    pub name: String,
    pub port: u16,
    /// The track it is on, as its host names it: empty if the host
    /// has not said.
    pub track: String,
    /// Each setting it has, by key, and the group that is in.
    pub keys: Vec<(String, u64)>,
}

/// What instances say to each other.
#[derive(Debug, Clone, PartialEq)]
pub enum Said {
    Here(Peer),
    /// A setting of `group` moved to `plain`, in its own units.
    Set {
        group: u64,
        key: String,
        plain: f64,
    },
    /// Put your `key` in `group`.
    Join {
        key: String,
        group: u64,
    },
    /// Take your `key` out of whatever group it is in.
    Leave {
        key: String,
    },
}

pub fn encode(said: &Said) -> String {
    match said {
        Said::Here(peer) => {
            let keys: Vec<String> = peer.keys.iter().map(|(k, g)| format!("{k}={g}")).collect();
            // The track comes last: an instance from before tracks
            // were told reads the rest and stops.
            format!(
                "{MAGIC}\there\t{}\t{}\t{}\t{}\t{}",
                peer.uid,
                peer.port,
                peer.name.replace(['\t', '\n'], " "),
                keys.join(","),
                peer.track.replace(['\t', '\n'], " ")
            )
        }
        Said::Set { group, key, plain } => format!("{MAGIC}\tset\t{group}\t{key}\t{plain}"),
        Said::Join { key, group } => format!("{MAGIC}\tjoin\t{key}\t{group}"),
        Said::Leave { key } => format!("{MAGIC}\tleave\t{key}"),
    }
}

/// `None` for anything that is not one of ours, whoever sent it.
pub fn decode(text: &str) -> Option<Said> {
    let mut parts = text.split('\t');
    if parts.next()? != MAGIC {
        return None;
    }
    let said = match parts.next()? {
        "here" => Said::Here(Peer {
            uid: parts.next()?.parse().ok()?,
            port: parts.next()?.parse().ok()?,
            name: parts.next()?.to_string(),
            keys: parts
                .next()?
                .split(',')
                .filter(|pair| !pair.is_empty())
                .map(|pair| {
                    let (key, group) = pair.split_once('=')?;
                    Some((key.to_string(), group.parse().ok()?))
                })
                .collect::<Option<_>>()?,
            // And one from before them says nothing of its track.
            track: parts.next().unwrap_or("").to_string(),
        }),
        "set" => Said::Set {
            group: parts.next()?.parse().ok()?,
            key: parts.next()?.to_string(),
            plain: parts.next()?.parse().ok().filter(|v: &f64| v.is_finite())?,
        },
        "join" => Said::Join {
            key: parts.next()?.to_string(),
            group: parts.next()?.parse().ok()?,
        },
        "leave" => Said::Leave {
            key: parts.next()?.to_string(),
        },
        _ => return None,
    };
    Some(said)
}

/// What the editor and the linking thread share: which group each
/// setting is in, who else is here, and what the editor has asked for.
pub struct Links {
    groups: Box<[AtomicU64]>,
    uid: AtomicU64,
    version: AtomicU64,
    peers: Mutex<Vec<Peer>>,
    track: Mutex<String>,
    asks: Mutex<Vec<(u64, Said)>>,
}

impl Default for Links {
    fn default() -> Self {
        Self {
            groups: (0..LINKABLE).map(|_| AtomicU64::new(0)).collect(),
            uid: AtomicU64::new(0),
            version: AtomicU64::new(0),
            peers: Mutex::default(),
            track: Mutex::default(),
            asks: Mutex::default(),
        }
    }
}

// A number no other instance will have picked: the time, stirred with
// where this one lives in memory. Kept to 52 bits so it survives a
// trip through a float, which is how states and pages carry numbers.
fn fresh(salt: usize) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let stirred = (now ^ (salt as u64).rotate_left(29)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    (stirred >> 12).max(1)
}

impl Links {
    fn changed(&self) {
        self.version.fetch_add(1, Ordering::AcqRel);
    }

    /// Moves whenever anything an editor shows of this does.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// This instance's own number, once its thread is running.
    pub fn uid(&self) -> u64 {
        self.uid.load(Ordering::Relaxed)
    }

    /// The group setting `index` is in: 0 for none.
    pub fn group(&self, index: usize) -> u64 {
        self.groups
            .get(index)
            .map_or(0, |g| g.load(Ordering::Relaxed))
    }

    pub fn set_group(&self, index: usize, group: u64) {
        if let Some(slot) = self.groups.get(index) {
            slot.store(group, Ordering::Relaxed);
            self.changed();
        }
    }

    /// The track this instance is on, as the host names it. The
    /// format's wrapper sets it when the host says; others hear of it
    /// the next time this instance says it is here.
    pub fn set_track(&self, name: &str) {
        let name = name.trim();
        if let Ok(mut track) = self.track.lock()
            && *track != name
        {
            *track = name.to_string();
            drop(track);
            self.changed();
        }
    }

    pub fn track(&self) -> String {
        self.track.lock().map(|t| t.clone()).unwrap_or_default()
    }

    /// Every other instance heard from lately.
    pub fn peers(&self) -> Vec<Peer> {
        self.peers.lock().map(|p| p.clone()).unwrap_or_default()
    }

    /// From the editor: link setting `index`, whose key is `key`, with
    /// the same setting of instance `peer`. This one's group is kept
    /// if it has one, so the peer joins whoever is already linked.
    pub fn tie(&self, index: usize, key: &str, peer: u64) {
        let group = match self.group(index) {
            0 => fresh(self as *const Self as usize + index),
            group => group,
        };
        self.set_group(index, group);
        self.ask(
            peer,
            Said::Join {
                key: key.to_string(),
                group,
            },
        );
    }

    /// From the editor: take instance `peer`'s `key` out of its group,
    /// or with no peer, this instance's own.
    pub fn untie(&self, index: usize, key: &str, peer: Option<u64>) {
        match peer {
            Some(peer) => self.ask(
                peer,
                Said::Leave {
                    key: key.to_string(),
                },
            ),
            None => self.set_group(index, 0),
        }
    }

    fn ask(&self, peer: u64, said: Said) {
        if let Ok(mut asks) = self.asks.lock() {
            asks.push((peer, said));
        }
    }
}

// The thread's own view of one instance.
struct Station {
    spec: Spec,
    shared: Weak<Shared>,
    socket: UdpSocket,
    port: u16,
    // Each setting as it was last seen or sent, so a move is noticed
    // and one that came from a peer is not sent back.
    last: Vec<f64>,
    seen: Vec<(u64, Instant)>,
    said: Instant,
}

fn to(port: u16) -> SocketAddrV4 {
    SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)
}

impl Station {
    fn send(&self, port: u16, said: &Said) {
        // A peer that has gone is found out by its silence, not here.
        let _ = self.socket.send_to(encode(said).as_bytes(), to(port));
    }

    fn here(&self, links: &Links) -> Said {
        Said::Here(Peer {
            uid: links.uid(),
            name: self.spec.info.name.to_string(),
            port: self.port,
            track: links.track(),
            keys: self
                .spec
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| (p.key.to_string(), links.group(i)))
                .collect(),
        })
    }

    fn announce(&mut self, links: &Links) {
        let here = self.here(links);
        (FIRST_PORT..FIRST_PORT + PORTS)
            .filter(|port| *port != self.port)
            .for_each(|port| self.send(port, &here));
        self.said = Instant::now();
    }

    // The plain value of setting `index` as a peer should take it: a
    // number in its own units, or a choice's place in its list.
    fn plain(&self, index: usize, norm: f64) -> f64 {
        self.spec.params[index].plain(norm)
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        self.spec.params.iter().position(|p| p.key == key)
    }

    fn hear(&mut self, said: Said, shared: &Shared, links: &Links) {
        match said {
            Said::Here(peer) if peer.uid != links.uid() => {
                let Ok(mut peers) = links.peers.lock() else {
                    return;
                };
                let known = peers.iter().position(|p| p.uid == peer.uid);
                let now = Instant::now();
                match self.seen.iter_mut().find(|(uid, _)| *uid == peer.uid) {
                    Some(seen) => seen.1 = now,
                    None => self.seen.push((peer.uid, now)),
                }
                match known {
                    Some(at) if peers[at] == peer => {}
                    Some(at) => {
                        peers[at] = peer;
                        links.changed();
                    }
                    None => {
                        // Someone new: say who this is at once, so the
                        // two know each other without waiting a turn.
                        let port = peer.port;
                        peers.push(peer);
                        peers.sort_by_key(|p| p.uid);
                        drop(peers);
                        links.changed();
                        let here = self.here(links);
                        self.send(port, &here);
                    }
                }
            }
            Said::Here(_) => {}
            Said::Set { group, key, plain } => {
                let Some(index) = self.index_of(&key) else {
                    return;
                };
                if group == 0 || links.group(index) != group {
                    return;
                }
                let norm = self.spec.params[index].normalized(plain);
                if (norm - self.last[index]).abs() < 1e-9 {
                    return;
                }
                self.last[index] = norm;
                shared.set(index, norm);
                shared.edit(index, BEGIN | SET | END);
            }
            Said::Join { key, group } => {
                if let Some(index) = self.index_of(&key) {
                    links.set_group(index, group);
                    self.announce(links);
                }
            }
            Said::Leave { key } => {
                if let Some(index) = self.index_of(&key) {
                    links.set_group(index, 0);
                    self.announce(links);
                }
            }
        }
    }

    // Tells the group of every linked setting that has moved.
    fn tell(&mut self, shared: &Shared, links: &Links) {
        let peers = links.peers();
        for index in 0..self.spec.params.len().min(LINKABLE) {
            let Some(norm) = shared.value(index) else {
                continue;
            };
            let moved = (norm - self.last[index]).abs() > 1e-9;
            self.last[index] = norm;
            let group = links.group(index);
            if !moved || group == 0 {
                continue;
            }
            self.send_value(&peers, index, group, norm);
        }
    }

    fn send_value(&self, peers: &[Peer], index: usize, group: u64, norm: f64) {
        let key = self.spec.params[index].key;
        let said = Said::Set {
            group,
            key: key.to_string(),
            plain: self.plain(index, norm),
        };
        peers
            .iter()
            .filter(|p| p.keys.iter().any(|(k, g)| k == key && *g == group))
            .for_each(|p| self.send(p.port, &said));
    }

    // Does what the editor asked: a peer that joins is sent where the
    // setting is now, so the two agree from the start.
    fn ask(&mut self, shared: &Shared, links: &Links) {
        let asks = match links.asks.lock() {
            Ok(mut asks) => std::mem::take(&mut *asks),
            Err(_) => return,
        };
        if asks.is_empty() {
            return;
        }
        let peers = links.peers();
        for (uid, said) in asks {
            let Some(peer) = peers.iter().find(|p| p.uid == uid) else {
                continue;
            };
            self.send(peer.port, &said);
            let Said::Join { key, group } = said else {
                continue;
            };
            let value = self
                .index_of(&key)
                .and_then(|i| Some((i, shared.value(i)?)));
            if let Some((index, norm)) = value {
                let now = Said::Set {
                    group,
                    key,
                    plain: self.plain(index, norm),
                };
                self.send(peer.port, &now);
            }
        }
        self.announce(links);
    }

    fn forget(&mut self, links: &Links) {
        let now = Instant::now();
        let before = self.seen.len();
        self.seen.retain(|(_, at)| now.duration_since(*at) < KEPT);
        if self.seen.len() == before {
            return;
        }
        if let Ok(mut peers) = links.peers.lock() {
            peers.retain(|p| self.seen.iter().any(|(uid, _)| *uid == p.uid));
        }
        links.changed();
    }
}

/// Linking, running for one instance. Dropping it stops the thread and
/// waits for it to end. An instance must hold this for as long as it
/// lives and no longer: a host may unload the plugin's library the
/// moment its last instance is gone, and a thread still running then
/// would be running code that is no longer there.
pub struct Running {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Starts linking for one instance. `None`, and nothing is linked, if
/// every port is taken or the machine will not give one.
pub fn start(shared: &Arc<Shared>, spec: Spec) -> Option<Running> {
    let (socket, port) = (FIRST_PORT..FIRST_PORT + PORTS)
        .find_map(|port| Some((UdpSocket::bind(to(port)).ok()?, port)))?;
    socket.set_read_timeout(Some(TICK)).ok()?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let weak = Arc::downgrade(shared);
    let links = &shared.scope().links;
    links
        .uid
        .store(fresh(Arc::as_ptr(shared) as usize), Ordering::Relaxed);
    let mut station = Station {
        last: shared.values(),
        spec,
        shared: weak,
        socket,
        port,
        seen: Vec::new(),
        said: Instant::now(),
    };
    let run = move || {
        let mut buf = [0u8; 8192];
        let mut first = true;
        loop {
            if stopped.load(Ordering::Acquire) {
                return;
            }
            let Some(shared) = station.shared.upgrade() else {
                return;
            };
            let scope = shared.scope();
            let links = &scope.links;
            if first || station.said.elapsed() >= SAYS {
                first = false;
                station.announce(links);
                station.forget(links);
            }
            // Waits up to a tick for the first, then takes what is
            // already there.
            let mut waited = false;
            while let Ok((n, from)) = station.socket.recv_from(&mut buf) {
                let ours = from.ip().is_loopback();
                let said = std::str::from_utf8(&buf[..n]).ok().and_then(decode);
                if let (true, Some(said)) = (ours, said) {
                    station.hear(said, &shared, links);
                }
                if !waited {
                    waited = true;
                    let _ = station.socket.set_nonblocking(true);
                }
            }
            if waited {
                let _ = station.socket.set_nonblocking(false);
            }
            station.ask(&shared, links);
            station.tell(&shared, links);
        }
    };
    let thread = std::thread::Builder::new()
        .name("dap-link".into())
        .spawn(run)
        .ok()?;
    Some(Running {
        stop,
        thread: Some(thread),
    })
}

/// Whether a setting may be linked at all: a meter is not a setting.
pub fn linkable(kind: &Kind) -> bool {
    !matches!(kind, Kind::Level | Kind::Fraction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Info, Param, Role, list, range};

    const INFO: Info = Info {
        name: "Linked",
        vendor: "",
        url: "",
        description: "",
        version: "",
        clap_id: "",
        vst3_processor: [0; 4],
        vst3_controller: [0; 4],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (1, 1),
        min_size: (1, 1),
    };

    static WIDE: [Param; 2] = [
        range(1, "drive", "Drive", "", (0.0, 24.0, 0.0), "dB"),
        list(2, "bypass", "Bypass", "", &["Off", "On"], 0),
    ];
    // The same keys in another plugin, with a narrower Drive.
    static NARROW: [Param; 2] = [
        list(7, "bypass", "Bypass", "", &["Off", "On"], 0),
        range(9, "drive", "Drive", "", (0.0, 12.0, 0.0), "dB"),
    ];

    fn spec(params: &'static [Param]) -> Spec {
        Spec {
            info: &INFO,
            params,
            meters: &[],
            assets: &[],
            key_in: 0,
            category: crate::Category::Other,
        }
    }

    // An instance, and its linking for as long as this is held.
    struct Instance(Arc<Shared>, #[allow(dead_code)] Option<Running>);

    impl std::ops::Deref for Instance {
        type Target = Shared;
        fn deref(&self) -> &Shared {
            &self.0
        }
    }

    fn instance(params: &'static [Param]) -> Instance {
        let shared = Arc::new(Shared::new(&spec(params)));
        let running = start(&shared, spec(params));
        assert!(running.is_some(), "a port to link on");
        Instance(shared, running)
    }

    // Waits up to three seconds for `done`: the threads are real.
    fn until(done: impl Fn() -> bool) -> bool {
        let from = Instant::now();
        while from.elapsed() < Duration::from_secs(3) {
            if done() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn what_is_said_survives_the_wire_and_what_is_not_ours_is_ignored() {
        let cases = [
            Said::Here(Peer {
                uid: 77,
                name: "Under tow".into(),
                port: 47_311,
                track: "Drum Bus".into(),
                keys: vec![("drive".into(), 0), ("bypass".into(), 901)],
            }),
            Said::Here(Peer {
                uid: 1,
                name: "Bare".into(),
                port: 1,
                track: String::new(),
                keys: vec![],
            }),
            Said::Set {
                group: 901,
                key: "drive".into(),
                plain: -12.5,
            },
            Said::Join {
                key: "bypass".into(),
                group: 4_503_599_627_370_495,
            },
            Said::Leave { key: "mix".into() },
        ];
        for said in cases {
            assert_eq!(decode(&encode(&said)), Some(said));
        }
        for junk in [
            "",
            "hello",
            "DAP1",
            "DAP1\tset\t1\tdrive\tNaN",
            "DAP1\tset\tx\tdrive\t1",
            "DAP2\tleave\tmix",
            "DAP1\there\t1\t2\tName\tdrive",
        ] {
            assert_eq!(decode(junk), None, "{junk:?}");
        }
        // An instance from before tracks were told is still heard.
        let old = decode("DAP1\there\t5\t6\tOld\tmix=0").expect("heard");
        assert_eq!(
            old,
            Said::Here(Peer {
                uid: 5,
                name: "Old".into(),
                port: 6,
                track: String::new(),
                keys: vec![("mix".into(), 0)],
            })
        );
    }

    #[test]
    fn two_instances_find_each_other_and_a_linked_setting_moves_both() {
        let (a, b) = (instance(&WIDE), instance(&NARROW));
        let (la, lb) = (a.scope(), b.scope());
        let b_uid = lb.links.uid();
        assert!(until(|| la.links.peers().iter().any(|p| p.uid == b_uid)));
        assert!(until(|| !lb.links.peers().is_empty()));

        // Bypass is setting 1 here and setting 0 there: it is the key
        // that is linked. The one asked to join takes the other's
        // value at once.
        a.set(1, 1.0);
        la.links.tie(1, "bypass", b_uid);
        assert!(until(|| lb.links.group(0) == la.links.group(1)));
        assert_ne!(la.links.group(1), 0);
        assert!(until(|| b.value(0) == Some(1.0)));
        // And it is told to its host as an editor's move would be.
        assert_eq!(b.take_edit(0), BEGIN | SET | END);

        // Either end moves the other.
        b.set(0, 0.0);
        assert!(until(|| a.value(1) == Some(0.0)));
        a.set(1, 1.0);
        assert!(until(|| b.value(0) == Some(1.0)));

        // What is not linked does not move.
        a.set(0, 0.5);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(b.value(1), Some(0.0));

        // Unlinked, it stays where it was left.
        la.links.untie(1, "bypass", Some(b_uid));
        assert!(until(|| lb.links.group(0) == 0));
        a.set(1, 0.0);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(b.value(0), Some(1.0));
    }

    #[test]
    fn a_value_crosses_in_its_own_units_and_stops_at_the_others_range() {
        let (a, b) = (instance(&WIDE), instance(&NARROW));
        let (la, lb) = (a.scope(), b.scope());
        let b_uid = lb.links.uid();
        assert!(until(|| la.links.peers().iter().any(|p| p.uid == b_uid)));
        la.links.tie(0, "drive", b_uid);
        assert!(until(|| lb.links.group(1) != 0));

        // 6 dB of 24 here is 6 dB of 12 there.
        a.set(0, 0.25);
        assert!(until(|| b.value(1) == Some(0.5)));
        // 18 dB is more than there is there.
        a.set(0, 0.75);
        assert!(until(|| b.value(1) == Some(1.0)));
    }

    #[test]
    fn an_instance_tells_the_others_which_track_it_is_on() {
        let (a, b) = (instance(&WIDE), instance(&NARROW));
        let (la, lb) = (a.scope(), b.scope());
        let b_uid = lb.links.uid();
        lb.links.set_track("  Drum Bus ");
        let heard = || {
            la.links
                .peers()
                .into_iter()
                .find(|p| p.uid == b_uid)
                .map(|p| p.track)
        };
        assert!(until(|| heard().as_deref() == Some("Drum Bus")));
        // And when the track is renamed.
        let version = la.links.version();
        lb.links.set_track("Kit");
        assert!(until(|| heard().as_deref() == Some("Kit")));
        assert!(la.links.version() > version);
    }

    #[test]
    fn an_instance_that_goes_is_forgotten() {
        let a = instance(&WIDE);
        let la = a.scope();
        {
            let b = instance(&NARROW);
            let uid = b.scope().links.uid();
            assert!(until(|| la.links.peers().iter().any(|p| p.uid == uid)));
        }
        let from = Instant::now();
        while from.elapsed() < Duration::from_secs(6) && !la.links.peers().is_empty() {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(la.links.peers().is_empty());
    }
}
