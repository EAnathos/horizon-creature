//! Small crawlers (SPECS §3.6): the slow crawl on the ground of worms, scorpions and the other ground creatures.
//!
//! A mod gives a `Cfg` (how fast, how long a crawl and a pause last, how far a wild one wanders, how its settled area
//! is found) and adds `Crawl::new(&CFG)` to its goals.
//!
//! - A wild one crawls a few blocks around where it first crawled: it picks a point within `wander` blocks of its home,
//!   crawls slowly towards it with pauses between, facing the way it goes.
//! - A settled one (released from a jar, the library's territory) crawls only over its area: it picks one of those
//!   blocks and crawls onto it. Every `check_every` ticks it asks its mod for the area again (`settled_area`), so a
//!   patch grown too small frees it.
//! - `on_pause` runs when it stops to rest (the worm's dig now and then): a hook on the mob, to play a sound.
//!
//! The mod poses the crawl itself (the worm's wiggle) from the mob's walk speed in its `mob_submit`.
use crate::sounds;
use horizon_client::mobs::goals;
use horizon_client::mod_api::{Cx, Goal, Mob, RandomSource};

/// How a kind of crawler moves. `settled_area` returns the blocks a settled creature may crawl over (empty when it is
/// wild or its area is gone), each the ground it stands on; the creature crawls on top of them.
#[derive(Clone, Copy, Debug)]
pub struct Cfg {
    /// Crawling speed (blocks a tick).
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
    pub settled_area: fn(&Cx, u32) -> Vec<[i32; 3]>,
    /// Run when it stops to rest, on the mob (the worm's dig); `None` for silence.
    pub on_pause: Option<fn(&mut Mob)>,
}

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
        }
    }

    fn pick(&self, m: &mut Mob) -> [f64; 3] {
        if !self.area.is_empty() {
            let b = self.area[m.random.next_int_bound(self.area.len() as i32).unwrap_or(0) as usize];
            return [f64::from(b[0]) + 0.5, f64::from(b[1]) + 1.0, f64::from(b[2]) + 0.5];
        }
        let home = self.home.unwrap_or([m.position.x, m.position.y, m.position.z]);
        let r = |m: &mut Mob| (f64::from(m.random.next_float()) * 2.0 - 1.0) * self.cfg.wander;
        [home[0] + r(m), m.position.y, home[2] + r(m)]
    }
}

/// A number in `range`, from the mob's own randomness.
pub fn between(m: &mut Mob, range: (i32, i32)) -> i32 {
    range.0 + m.random.next_int_bound(range.1 - range.0).unwrap_or(0)
}

impl Goal for Crawl {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        m.fall_distance = 0.0;
        if self.home.is_none() {
            self.home = Some([m.position.x, m.position.y, m.position.z]);
        }
        self.check -= 1;
        if self.check <= 0 {
            self.check = self.cfg.check_every;
            self.area = (self.cfg.settled_area)(cx, m.id);
        }
        self.ticks -= 1;
        if self.pausing {
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            if self.ticks <= 0 {
                self.pausing = false;
                self.ticks = between(m, self.cfg.crawl);
                self.target = Some(self.pick(m));
            }
            return;
        }
        let target = match self.target {
            Some(t) => t,
            None => {
                let t = self.pick(m);
                self.target = Some(t);
                t
            }
        };
        let (dx, dz) = (target[0] - m.position.x, target[2] - m.position.z);
        let len = dx.hypot(dz);
        if len < 0.2 || self.ticks <= 0 {
            if let Some(rest) = self.cfg.on_pause {
                rest(m);
            }
            self.pausing = true;
            self.ticks = between(m, self.cfg.pause);
            self.target = None;
            return;
        }
        m.velocity.x = dx / len * self.cfg.speed;
        m.velocity.z = dz / len * self.cfg.speed;
        let yaw = (-dx).atan2(dz).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// A handy `on_pause`: now and then (one time in `chance`), play `sound` softly on the mob, for a crawler that noses
/// into the ground as it rests (the worm's dig).
pub fn rustle(m: &mut Mob, chance: u32, sound: &'static str) {
    if m.random.next_float() < 1.0 / chance as f32 {
        sounds::queue(m.id, sound, 0.4, 0.9 + m.random.next_float() * 0.2);
    }
}
