//! Small crawlers (SPECS §3.6): the slow crawl on the ground of worms, scorpions and the other ground creatures.
//!
//! A mod gives a `Cfg` (how fast, how long a crawl and a pause last, how far a wild one wanders, how its settled area
//! is found) and adds `Crawl::new(&CFG)` to its goals.
//!
//! - A wild one crawls a few blocks around where it first crawled: it picks a point within `wander` blocks of its home,
//!   crawls slowly towards it with pauses between. The engine walks it there (`GoalContext::move_to`) and turns it the
//!   way it goes.
//! - A settled one (released from a jar, the library's territory) crawls only over its area: it picks one of those
//!   blocks and crawls onto it. Every `check_every` ticks it asks its mod for the area again (`settled_area`), so a
//!   patch grown too small frees it.
//! - `on_pause` runs when it stops to rest (the worm's dig now and then), to queue a sound (`sounds::queue`).
//!
//! The mod poses the crawl itself (the worm's wiggle) from the mob's walk animation in its renderer.
use mod_api::{Goal, GoalContext, GoalFlags, Host, RandomSource};

use crate::sounds;

/// How a kind of crawler moves. `settled_area` returns the blocks a settled creature may crawl over (empty when it is
/// wild or its area is gone), each the ground it stands on; the creature crawls on top of them.
#[derive(Clone, Copy, Debug)]
pub struct Cfg {
    /// Crawling speed: the multiplier of the creature's movement speed its path is followed at (`move_to`'s speed). Keep
    /// it near 1 and set the creature's slowness with its `minecraft:movement_speed` attribute: the engine's path
    /// follower barely moves a mob at a small multiplier (0.07 left a worm in place).
    pub speed: f64,
    /// How long a crawl lasts (ticks), picked in the range.
    pub crawl: (i32, i32),
    /// How long a pause lasts (ticks), picked in the range.
    pub pause: (i32, i32),
    /// How far a wild one wanders from where it first crawled (blocks).
    pub wander: f64,
    /// How often a settled one asks its mod for its area again (ticks).
    pub check_every: i32,
    /// The settled area of creature `entity` (empty when wild), from the mod's territory rule.
    pub settled_area: fn(&dyn Host, u32) -> Vec<[i32; 3]>,
    /// Run when it stops to rest (the worm's dig); `None` for silence.
    pub on_pause: Option<fn(&mut GoalContext<'_>)>,
    /// Whether creature `entity` is busy with something else this tick (a scorpion rearing up, fighting): it doesn't
    /// crawl then. `None` for never.
    pub busy: Option<fn(u32) -> bool>,
}

/// How often a crawl asks the engine for its path again (ticks): another goal's path, or a block in the way, may have
/// dropped it.
const REPATH: i32 = 40;

/// Crawling about: a few blocks around where it first crawled, or over its area when settled.
#[derive(Clone, Debug)]
pub struct Crawl {
    cfg: Cfg,
    target: Option<[f64; 3]>,
    ticks: i32,
    pausing: bool,
    home: Option<[f64; 3]>,
    area: Vec<[i32; 3]>,
    check: i32,
    repath: i32,
}

impl Crawl {
    pub fn new(cfg: &Cfg) -> Self {
        Crawl {
            cfg: *cfg,
            target: None,
            ticks: 0,
            pausing: false,
            home: None,
            area: Vec::new(),
            check: 0,
            repath: 0,
        }
    }

    fn pick(&self, at: [f64; 3], random: &mut dyn RandomSource) -> [f64; 3] {
        if !self.area.is_empty() {
            let b = self.area[random.next_int_bound(self.area.len() as i32).max(0) as usize];
            return [f64::from(b[0]) + 0.5, f64::from(b[1]) + 1.0, f64::from(b[2]) + 0.5];
        }
        let home = self.home.unwrap_or(at);
        let mut r = || (f64::from(random.next_float()) * 2.0 - 1.0) * self.cfg.wander;
        [home[0] + r(), at[1], home[2] + r()]
    }
}

/// A number in `range`, from the mob's own randomness.
pub fn between(random: &mut dyn RandomSource, range: (i32, i32)) -> i32 {
    range.0 + random.next_int_bound((range.1 - range.0).max(1))
}

/// Where the goal's mob is.
fn position(cx: &GoalContext<'_>) -> Option<[f64; 3]> {
    cx.mob().entity().position(cx.host())
}

impl Goal for Crawl {
    fn flags(&self) -> GoalFlags {
        GoalFlags::MOVE.union(GoalFlags::LOOK).union(GoalFlags::JUMP)
    }
    fn can_use(&mut self, cx: &mut GoalContext<'_>) -> bool {
        self.cfg.busy.is_none_or(|busy| !busy(cx.mob().entity().id()))
    }
    fn requires_update_every_tick(&self) -> bool {
        true
    }
    fn name(&self) -> &'static str {
        "creature:crawl"
    }
    fn tick(&mut self, cx: &mut GoalContext<'_>) {
        let Some(at) = position(cx) else {
            return;
        };
        if self.home.is_none() {
            self.home = Some(at);
        }
        self.check -= 1;
        if self.check <= 0 {
            self.check = self.cfg.check_every;
            self.area = (self.cfg.settled_area)(cx.host(), cx.mob().entity().id());
        }
        self.ticks -= 1;
        if self.pausing {
            if self.ticks <= 0 {
                self.pausing = false;
                self.ticks = between(cx.random(), self.cfg.crawl);
                self.target = Some(self.pick(at, cx.random()));
                self.repath = 0;
            }
            return;
        }
        let target = match self.target {
            Some(t) => t,
            None => {
                let t = self.pick(at, cx.random());
                self.target = Some(t);
                self.repath = 0;
                t
            }
        };
        let len = (target[0] - at[0]).hypot(target[2] - at[2]);
        if len < 0.3 || self.ticks <= 0 {
            cx.stop_navigation();
            if let Some(rest) = self.cfg.on_pause {
                rest(cx);
            }
            self.pausing = true;
            self.ticks = between(cx.random(), self.cfg.pause);
            self.target = None;
            return;
        }
        self.repath -= 1;
        if self.repath <= 0 {
            self.repath = REPATH;
            if !cx.move_to(target, self.cfg.speed) {
                // No path there (a wall, water): pick another point next tick.
                self.target = None;
            }
        }
    }
    fn stop(&mut self, cx: &mut GoalContext<'_>) {
        cx.stop_navigation();
    }
}

/// A handy `on_pause`: now and then (one time in `chance`), play `sound` softly on the mob, for a crawler that noses
/// into the ground as it rests (the worm's dig).
pub fn rustle(cx: &mut GoalContext<'_>, chance: u32, sound: &'static str) {
    if cx.random().next_float() < 1.0 / chance as f32 {
        let pitch = 0.9 + cx.random().next_float() * 0.2;
        sounds::queue(cx.mob().entity().id(), sound, 0.4, pitch);
    }
}
