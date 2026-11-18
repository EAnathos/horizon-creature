//! Its fight (SPECS §2): the club's overhead blow in reach, a boulder lifted and hurled out of it (`Smash`).
use super::*;

/// The club's blow (SPECS §2): it walks up to its target, raises the club over its head for `WINDUP` ticks, then
/// brings it down; hit if the target is still within `REACH`, its feet less than `CLUB_HEIGHT` blocks above the
/// cyclops's (and not more than `REACH` below). One blow every `EVERY` ticks. It lets go of a target out of `GIVE_UP`
/// blocks, dead, or a creative player.
pub(crate) const REACH: f64 = 4.0;
pub(crate) const CLUB_HEIGHT: f64 = 6.0;
pub(crate) const WINDUP: i32 = 14;
pub(crate) const EVERY: i32 = 40;
pub(crate) const CHASE_SPEED: f64 = 1.1;
/// The club's push on whom it hits (blocks a tick, sideways and up).
pub(crate) const PUSH: f64 = 1.0;
pub(crate) const GIVE_UP: f64 = 32.0;
/// The boulder (SPECS §2): thrown at a target it sees `THROW_FROM` to `THROW_TO` blocks away, raised over its head for
/// `THROW_WINDUP` ticks first, at most once every `THROW_EVERY` ticks; from just over its head.
pub(crate) const THROW_FROM: f64 = 7.0;
pub(crate) const THROW_TO: f64 = 24.0;
pub(crate) const THROW_WINDUP: i32 = 20;
pub(crate) const THROW_EVERY: i32 = 100;
pub(crate) const THROW_HEIGHT: f64 = 4.05;
/// Where it holds the boulder before the throw: between its raised hands (blocks from its feet).
pub(crate) const HELD_HEIGHT: f32 = 4.4;
pub(crate) const THROW: &str = "cyclops:entity.cyclops.throw";

/// When each cyclops raised its club, or a boulder, last, for the drawing (single player).
pub(crate) static RAISED: Mutex<BTreeMap<u32, Instant>> = Mutex::new(BTreeMap::new());
pub(crate) static LIFTED: Mutex<BTreeMap<u32, Instant>> = Mutex::new(BTreeMap::new());

/// What a cyclops's fight asked its tick to do (a behaviour can't hurt or change a target): its club hit `target`, or
/// it let its target go.
pub(crate) static HITS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
pub(crate) static LET_GO: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// From the cyclops's tick: its club's hits (its attack damage, a push away and up) and a target let go.
pub(crate) fn apply(host: &mut dyn ServerHost, cyclops: u32) {
    let Some(me) = host.entity(cyclops) else {
        return;
    };
    if LET_GO.lock().unwrap().remove(&cyclops)
        && let Some(m) = host.mob(me)
    {
        m.set_target(host, None);
    }
    let hits: Vec<u32> = {
        let mut all = HITS.lock().unwrap();
        let (mine, rest): (Vec<_>, Vec<_>) = all.drain(..).partition(|h| h.0 == cyclops);
        *all = rest;
        mine.into_iter().map(|h| h.1).collect()
    };
    for target in hits {
        let Some(t) = host.entity(target) else {
            continue;
        };
        if let Some(l) = host.living(me) {
            l.swing(host, mod_api::Hand::Main, true);
        }
        let damage = host
            .living(me)
            .and_then(|l| l.attribute_value(host, "minecraft:attack_damage"))
            .unwrap_or(f64::from(ATTACK_DAMAGE));
        t.hurt(host, "minecraft:mob_attack", damage as f32, Some(me), Some(me));
        if let (Some(a), Some(b), Some(v)) = (me.position(host), t.position(host), t.velocity(host)) {
            let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
            let len = dx.hypot(dz).max(0.01);
            t.set_velocity(
                host,
                [v[0] + dx / len * PUSH, v[1] + PUSH * 0.4, v[2] + dz / len * PUSH],
            );
        }
    }
}

/// Its target, if it is still one to fight: where it is and its eye height; none (and let go) when gone, dead,
/// creative or out of `GIVE_UP`.
fn target(m: &Body, cx: &World<'_>) -> Option<(u32, [f64; 3], f64)> {
    let host = cx.host();
    let t = host.entity(m.id).and_then(|e| host.mob(e))?.target(host)?;
    let e = t.entity();
    let at = e.position(host)?;
    let alive = t.alive(host).unwrap_or(false);
    let creative = cx.player.as_ref().is_some_and(|p| p.entity_id == e.id() && p.creative);
    let far = (at[0] - m.position.x).hypot(at[2] - m.position.z) > GIVE_UP;
    if !alive || creative || far {
        LET_GO.lock().unwrap().insert(m.id);
        return None;
    }
    let eye = at[1] + e.dimensions(host).map_or(1.6, |d| d[1] * 0.85);
    Some((e.id(), at, eye))
}

/// Whether the cyclops sees `target`.
fn sees(cx: &World<'_>, cyclops: u32, target: u32) -> bool {
    let host = cx.host();
    let (Some(me), Some(t)) = (host.entity(cyclops).and_then(|e| host.living(e)), host.entity(target)) else {
        return false;
    };
    me.has_line_of_sight(host, t).unwrap_or(false)
}

#[derive(Debug, Default)]
pub(crate) struct Smash {
    windup: i32,
    cooldown: i32,
    repath: i32,
    throwing: i32,
    throw_cooldown: i32,
    started: bool,
}

impl Behaviour for Smash {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = target(m, cx).is_some();
        if on && !self.started {
            self.repath = 0;
            self.throw_cooldown = self.throw_cooldown.max(THROW_WINDUP);
        }
        self.started = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.windup = 0;
        self.throwing = 0;
        self.started = false;
        m.stop_walking = true;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some((who, t, eye)) = target(m, cx) else {
            return;
        };
        m.look_at = Some([t[0], eye, t[2]]);
        let reach = (t[0] - m.position.x).hypot(t[2] - m.position.z);
        let rise = t[1] - m.position.y;
        let club_height = (-REACH..CLUB_HEIGHT).contains(&rise);
        self.cooldown -= 1;
        if self.windup > 0 {
            // The club up: it stands, then strikes.
            self.windup -= 1;
            if self.windup == 0 {
                if reach < REACH + 0.5 && club_height {
                    HITS.lock().unwrap().push((m.id, who));
                }
                // The club comes down in front of it, at its feet, whether or not it hits.
                let (dx, dz) = (t[0] - m.position.x, t[2] - m.position.z);
                let far = dx.hypot(dz).max(0.01);
                let ahead = far.min(REACH * 0.75) / far;
                smash::blow(
                    m.id,
                    [m.position.x + dx * ahead, m.position.y - 0.5, m.position.z + dz * ahead],
                );
                creature::sounds::queue(m.id, SWING, 1.0, 1.0);
                let pitch = 0.9 + m.random.next_float() * 0.2;
                creature::sounds::queue(m.id, HIT, 1.3, pitch);
                self.cooldown = EVERY;
            }
            return;
        }
        if self.throwing > 0 {
            // The boulder up: it stands, then hurls it at where the target is.
            self.throwing -= 1;
            if self.throwing == 0 {
                let from = [m.position.x, m.position.y + THROW_HEIGHT, m.position.z];
                let to = [t[0], t[1] + 0.9, t[2]];
                let velocity = boulder::aim(from, to);
                boulder::THROWS.lock().unwrap().push(boulder::Throw {
                    from,
                    velocity,
                    target: who,
                });
                creature::sounds::queue(m.id, THROW, 1.2, 1.0);
                self.throw_cooldown = THROW_EVERY;
            }
            return;
        }
        self.throw_cooldown -= 1;
        // A target out of the club's height (up a pillar, a cliff) gets a boulder at any distance.
        let throwable = (THROW_FROM..THROW_TO).contains(&reach) || (!club_height && reach < THROW_TO);
        if throwable && self.throw_cooldown <= 0 && sees(cx, m.id, who) {
            self.throwing = THROW_WINDUP;
            m.stop_walking = true;
            LIFTED.lock().unwrap().insert(m.id, Instant::now());
            creature::sounds::queue(m.id, ROAR, 1.5, 1.0);
            return;
        }
        if reach < REACH && club_height && self.cooldown <= 0 {
            self.windup = WINDUP;
            m.stop_walking = true;
            RAISED.lock().unwrap().insert(m.id, Instant::now());
            return;
        }
        self.repath -= 1;
        if self.repath <= 0 {
            m.walk_to = Some((t, CHASE_SPEED));
            self.repath = 10;
        }
    }
}
