//! How a jellyfish swims (SPECS §3), by hand (the library's `body`), only in water: it pulses (the bell contracts and
//! it rises about a block), then sinks slowly, drifting. It stays under the surface, deeper by day and nearer the
//! surface by night (the glow jelly deeper still by day). Out of water it is beached: the library's `aquatic` makes it
//! fall and flop.
use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::body::{Behaviour, Body, Vec3, World};

/// A pulse's length (ticks) and climb (blocks a tick); its sinking (blocks a tick) and drift between pulses.
const PULSE_TICKS: i32 = 8;
const PULSE_UP: f64 = 0.13;
const SINK: f64 = 0.012;
const DRIFT: f64 = 0.012;
/// The wait between two pulses (ticks).
const WAIT: (i32, i32) = (40, 90);
/// How deep under the surface it likes to be (blocks): by day, by night; the glow jelly's by day.
const DAY_DEPTH: (f64, f64) = (3.0, 8.0);
const NIGHT_DEPTH: (f64, f64) = (1.0, 3.0);
const GLOW_DAY_DEPTH: (f64, f64) = (8.0, 16.0);
/// How far up it looks for the surface (blocks).
const LOOK_UP: i32 = 20;

/// The ticks of the pulse each jellyfish is in, for its drawing (single player: the renderer reads it).
static PULSING: Mutex<BTreeMap<u32, i32>> = Mutex::new(BTreeMap::new());

/// How far into its pulse jellyfish `id` is (0 to 1), 0 between pulses.
pub(crate) fn pulse(id: u32) -> f32 {
    PULSING
        .lock()
        .unwrap()
        .get(&id)
        .map_or(0.0, |&left| left as f32 / PULSE_TICKS as f32)
}

pub(crate) fn forget(id: u32) {
    PULSING.lock().unwrap().remove(&id);
}

/// The water over its bell (blocks), up to `LOOK_UP`.
fn depth(m: &Body, cx: &World<'_>) -> f64 {
    let (x, y, z) = (
        m.position.x.floor() as i32,
        (m.position.y + 0.9).floor() as i32,
        m.position.z.floor() as i32,
    );
    let water = (0..LOOK_UP)
        .take_while(|&d| cx.block([x, y + d, z]) == "minecraft:water")
        .count();
    water as f64 - (m.position.y + 0.9).fract()
}

#[derive(Debug, Default)]
pub(crate) struct Swim {
    /// Ticks left of the pulse it is in.
    pulse: i32,
    wait: i32,
    drift: [f64; 2],
    /// Whether it is a glow jelly (deeper by day).
    glows: bool,
}

impl Swim {
    pub(crate) fn new(glows: bool) -> Swim {
        Swim {
            glows,
            ..Default::default()
        }
    }
}

impl Behaviour for Swim {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        m.in_water
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.no_gravity = true;
        if self.pulse > 0 {
            self.pulse -= 1;
            PULSING.lock().unwrap().insert(m.id, self.pulse);
            m.velocity = Vec3 {
                x: self.drift[0],
                y: PULSE_UP,
                z: self.drift[1],
            };
            return;
        }
        PULSING.lock().unwrap().remove(&m.id);
        let depth = depth(m, cx);
        // Never out of the water: just under the surface it sinks a little faster.
        let sink = if depth < 0.8 { SINK * 3.0 } else { SINK };
        m.velocity = Vec3 {
            x: self.drift[0],
            y: -sink,
            z: self.drift[1],
        };
        self.wait -= 1;
        if self.wait > 0 {
            return;
        }
        self.wait = WAIT.0 + m.random.next_int_bound(WAIT.1 - WAIT.0).unwrap_or(0);
        let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
        self.drift = [angle.cos() * DRIFT, angle.sin() * DRIFT];
        let (shallow, deep) = match (cx.daylight, self.glows) {
            (false, _) => NIGHT_DEPTH,
            (true, true) => GLOW_DAY_DEPTH,
            (true, false) => DAY_DEPTH,
        };
        // Deeper than it likes: it pulses up; shallower: it lets itself sink; in between, now and then.
        let pulses = depth > deep || (depth > shallow && m.random.next_int_bound(3) == Some(0));
        if pulses && depth > 1.5 {
            self.pulse = PULSE_TICKS;
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.pulse = 0;
        PULSING.lock().unwrap().remove(&m.id);
    }
}
