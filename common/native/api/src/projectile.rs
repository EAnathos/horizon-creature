//! Thrown things (SPECS §3.9): the cyclops's boulder, the scorpion dagger. A projectile is a mob of its mod (a copy of
//! the wandering trader that can't be hurt, its own box and drawing), launched with a velocity (`launch`) and flown by
//! the `Flight` goal: its own gravity every tick, until it reaches its target (it hurts it with its own attack
//! damage), lands, hits a wall or flies too long. The mod reads how it ended (`ended`, from its `entity_tick_post`),
//! does what its projectile does there (a boulder shatters, a dagger falls as an item) and removes it.
//!
//! A projectile with no target finds what it hits on its own, from the mod's hook (`hits`: the mobs and players its
//! box meets), since a goal sees only its own target. A projectile saved mid-flight (its world left and loaded again)
//! has lost its velocity: it ends at once, as landed.
//!
//! Single player: the goals and the hooks share the mob's id (as the rest of the series, arismoko/horizon#2740).
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::mobs::Who;
use horizon_client::mobs::goals;
use horizon_client::mod_api::{Compound, Cx, Goal, Host, Mob, Tag};

/// How a projectile flies: its velocity (blocks a tick), its gravity (blocks a tick, each tick), how long it may fly
/// (ticks), and its target, if it has one, with how close it must come to its box to hit it (blocks).
#[derive(Clone, Copy, Debug)]
pub struct Launch {
    pub velocity: [f64; 3],
    pub gravity: f64,
    pub life: i32,
    pub target: Option<(Who, f64)>,
}

/// How a projectile's flight ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// It reached its target, and hurt it.
    Hit,
    /// It came down on the ground, hit a wall, or flew too long.
    Landed,
}

/// Each projectile's launch, read by its goal on its first tick.
static LAUNCHES: Mutex<BTreeMap<u32, Launch>> = Mutex::new(BTreeMap::new());
/// Each flying projectile's velocity now, for its drawing (it points where it goes).
static VELOCITIES: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());
/// How each projectile ended, for the mod's hook.
static ENDED: Mutex<BTreeMap<u32, End>> = Mutex::new(BTreeMap::new());

/// Summons projectile `kind` (a mob of the mod's, which `register` gives its flight) at `from`, flying as `launch` says.
/// Its id, if it came.
pub fn launch(host: &mut dyn Host, dimension: &str, kind: &str, from: [f64; 3], launch: Launch) -> Option<u32> {
    let mut nbt = Compound::new();
    nbt.insert("Invulnerable".into(), Tag::Byte(1));
    let id = host.summon(dimension, kind, from, nbt)?;
    LAUNCHES.lock().unwrap().insert(id, launch);
    VELOCITIES.lock().unwrap().insert(id, launch.velocity);
    Some(id)
}

/// From the mod's `register_goals`, for its projectile: muted, its own box (blocks: width, height, eye height), spared,
/// flown by `Flight`.
pub fn register(mob: &mut Mob, size: (f64, f64, f64)) {
    crate::mob::ours(mob, size.0, size.1, size.2);
    mob.ai.goals.add(1, Flight::default());
}

/// How projectile `id`'s flight ended, if it has (taken: told once).
pub fn ended(id: u32) -> Option<End> {
    ENDED.lock().unwrap().remove(&id)
}

/// Projectile `id`'s velocity now (for its drawing).
pub fn velocity(id: u32) -> Option<[f64; 3]> {
    VELOCITIES.lock().unwrap().get(&id).copied()
}

/// From the mod's hook: the projectile ends now (it hit something the hook found, `hits`).
pub fn stop(id: u32, end: End) {
    ENDED.lock().unwrap().insert(id, end);
    LAUNCHES.lock().unwrap().remove(&id);
}

/// What projectile `id` at `at` touches now, for one with no target: the mobs (not items, orbs or other projectiles
/// of `kind`) and players within `reach` blocks of it, leaving out `thrower` (its id).
pub fn hits(host: &mut dyn Host, dimension: &str, at: [f64; 3], kind: &str, thrower: u32, reach: f64) -> Vec<u32> {
    let near = |p: [f64; 3]| {
        // From the projectile to the middle of a body about a block tall standing at `p`.
        let mid = [p[0], p[1] + 0.6, p[2]];
        (0..3).map(|k| (mid[k] - at[k]).powi(2)).sum::<f64>().sqrt() < reach + 0.4
    };
    let mut found: Vec<u32> = host
        .level_entities(dimension)
        .into_iter()
        .filter(|(id, k, p)| {
            *id != thrower && k != kind && k != "minecraft:item" && k != "minecraft:experience_orb" && near(*p)
        })
        .map(|(id, _, _)| id)
        .collect();
    found.extend(
        host.server_players()
            .into_iter()
            .filter(|p| p.dimension == dimension && p.entity_id as u32 != thrower && near(p.position))
            .map(|p| p.entity_id as u32),
    );
    found
}

/// From the mod's `living_death` or once it removed the projectile.
pub fn forget(id: u32) {
    LAUNCHES.lock().unwrap().remove(&id);
    VELOCITIES.lock().unwrap().remove(&id);
    ENDED.lock().unwrap().remove(&id);
}

/// A projectile's flight: its own velocity and gravity every tick, until it hits its target or lands.
#[derive(Clone, Debug, Default)]
pub struct Flight {
    launch: Option<Launch>,
    age: i32,
    done: bool,
}

impl Flight {
    fn end(&mut self, m: &Mob, end: End) {
        self.done = true;
        ENDED.lock().unwrap().insert(m.id, end);
    }
}

impl Goal for Flight {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn can_continue(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        !self.done
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        if self.done {
            return;
        }
        self.age += 1;
        m.no_gravity = true;
        m.fall_distance = 0.0;
        if self.launch.is_none() {
            self.launch = LAUNCHES.lock().unwrap().remove(&m.id);
        }
        // Stopped by the mod's hook, or a projectile without a launch (one saved mid-flight): it ends.
        if ENDED.lock().unwrap().contains_key(&m.id) {
            self.done = true;
            return;
        }
        let Some(mut launch) = self.launch else {
            if self.age > 2 {
                self.end(m, End::Landed);
            }
            return;
        };
        if let Some((who, reach)) = launch.target
            && let Some(t) = cx.w.target_info(who)
        {
            let mut centre = m.position;
            centre.y += m.kind.height / 2.0;
            if t.alive && t.bounds.distance_squared_to_point(centre) < reach * reach {
                horizon_client::mobs::ai::do_hurt_target(m, who, cx);
                self.end(m, End::Hit);
                return;
            }
        }
        if self.age > 1 && (m.on_ground || m.horizontal_collision) || self.age > launch.life {
            self.end(m, End::Landed);
            return;
        }
        launch.velocity[1] -= launch.gravity;
        self.launch = Some(launch);
        VELOCITIES.lock().unwrap().insert(m.id, launch.velocity);
        m.velocity.x = launch.velocity[0];
        m.velocity.y = launch.velocity[1];
        m.velocity.z = launch.velocity[2];
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// The velocity that carries a projectile from `from` to `to` on an arc, at `speed` along the ground, with `gravity`
/// (blocks a tick): it is there after as many ticks as the ground takes at that speed.
pub fn aim(from: [f64; 3], to: [f64; 3], speed: f64, gravity: f64) -> [f64; 3] {
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let flat = dx.hypot(dz).max(0.1);
    let ticks = (flat / speed).ceil().max(1.0);
    // y after n ticks of v - g k: n v - g n (n - 1) / 2.
    let vy = (to[1] - from[1] + gravity * ticks * (ticks - 1.0) / 2.0) / ticks;
    [dx / ticks, vy, dz / ticks]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where a projectile thrown with `velocity` from `from` is after as many ticks as the ground to `to` takes.
    fn landing(from: [f64; 3], to: [f64; 3], velocity: [f64; 3], speed: f64, gravity: f64) -> [f64; 3] {
        let ticks = ((to[0] - from[0]).hypot(to[2] - from[2]) / speed).ceil().max(1.0) as i32;
        let (mut at, mut v) = (from, velocity);
        for _ in 0..ticks {
            at = [at[0] + v[0], at[1] + v[1], at[2] + v[2]];
            v[1] -= gravity;
        }
        at
    }

    #[test]
    fn a_projectile_lands_where_it_was_aimed() {
        for to in [[10.0, 0.0, 0.0], [0.0, 3.0, -15.0], [7.0, -4.0, 7.0], [0.5, 1.0, 0.0]] {
            let from = [0.0, 4.0, 0.0];
            let at = landing(from, to, aim(from, to, 0.9, 0.05), 0.9, 0.05);
            for k in 0..3 {
                assert!((at[k] - to[k]).abs() < 1e-9, "aimed at {to:?}, landed at {at:?}");
            }
        }
    }

    #[test]
    fn it_flies_at_its_speed_along_the_ground() {
        let v = aim([0.0, 0.0, 0.0], [20.0, 0.0, 0.0], 0.9, 0.05);
        assert!(v[0].hypot(v[2]) <= 0.9 + 1e-9);
    }
}
