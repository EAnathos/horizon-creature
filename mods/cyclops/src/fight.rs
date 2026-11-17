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

/// When each cyclops raised its club, or a boulder, last, for the drawing (single player; arismoko/horizon#2740).
pub(crate) static RAISED: Mutex<BTreeMap<u32, Instant>> = Mutex::new(BTreeMap::new());
pub(crate) static LIFTED: Mutex<BTreeMap<u32, Instant>> = Mutex::new(BTreeMap::new());

#[derive(Clone, Debug, Default)]
pub(crate) struct Smash {
    windup: i32,
    cooldown: i32,
    repath: i32,
    throwing: i32,
    throw_cooldown: i32,
}

impl Goal for Smash {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        m.target_info(cx.w).is_some_and(|t| t.alive && !t.creative)
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        let Some(t) = m.target_info(cx.w) else {
            return false;
        };
        let far = (t.position.x - m.position.x).hypot(t.position.z - m.position.z) > GIVE_UP;
        if far || !t.alive || t.creative {
            m.ai.target = None;
            return false;
        }
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, _: &mut Mob, _: &mut Cx) {
        self.repath = 0;
        self.throw_cooldown = self.throw_cooldown.max(THROW_WINDUP);
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.windup = 0;
        self.throwing = 0;
        m.ai.nav.stop();
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(who) = m.ai.target else {
            return;
        };
        let Some(t) = cx.w.target_info(who) else {
            return;
        };
        let at = [t.position.x, t.eye_y, t.position.z];
        m.ai.look.set_look_at(at, 30.0, 30.0);
        let reach = (t.position.x - m.position.x).hypot(t.position.z - m.position.z);
        let rise = t.position.y - m.position.y;
        let club_height = (-REACH..CLUB_HEIGHT).contains(&rise);
        self.cooldown -= 1;
        if self.windup > 0 {
            // The club up: it stands, then strikes.
            self.windup -= 1;
            if self.windup == 0 {
                if reach < REACH + 0.5 && club_height {
                    m.swing();
                    horizon_client::mobs::ai::do_hurt_target(m, who, cx);
                }
                // The club comes down in front of it, at its feet, whether or not it hits.
                let (dx, dz) = (t.position.x - m.position.x, t.position.z - m.position.z);
                let far = dx.hypot(dz).max(0.01);
                let ahead = far.min(REACH * 0.75) / far;
                smash::blow(
                    m.id,
                    [m.position.x + dx * ahead, m.position.y - 0.5, m.position.z + dz * ahead],
                );
                creature::sounds::queue(m.id, SWING, 1.0, 1.0);
                creature::sounds::queue(m.id, HIT, 1.3, 0.9 + m.random.next_float() * 0.2);
                self.cooldown = EVERY;
            }
            return;
        }
        if self.throwing > 0 {
            // The boulder up: it stands, then hurls it at where the target is.
            self.throwing -= 1;
            if self.throwing == 0 {
                let from = [m.position.x, m.position.y + THROW_HEIGHT, m.position.z];
                let to = [t.position.x, t.position.y + 0.9, t.position.z];
                let velocity = boulder::aim(from, to);
                boulder::THROWS.lock().unwrap().push(boulder::Throw {
                    from,
                    velocity,
                    target: who,
                });
                m.swing();
                creature::sounds::queue(m.id, THROW, 1.2, 1.0);
                self.throw_cooldown = THROW_EVERY;
            }
            return;
        }
        self.throw_cooldown -= 1;
        // A target out of the club's height (up a pillar, a cliff) gets a boulder at any distance.
        let throwable = (THROW_FROM..THROW_TO).contains(&reach) || (!club_height && reach < THROW_TO);
        if throwable && self.throw_cooldown <= 0 && m.sees(cx.w, who) {
            self.throwing = THROW_WINDUP;
            m.ai.nav.stop();
            LIFTED.lock().unwrap().insert(m.id, Instant::now());
            creature::sounds::queue(m.id, ROAR, 1.5, 1.0);
            return;
        }
        if reach < REACH && club_height && self.cooldown <= 0 {
            self.windup = WINDUP;
            m.ai.nav.stop();
            RAISED.lock().unwrap().insert(m.id, Instant::now());
            return;
        }
        self.repath -= 1;
        if self.repath <= 0 {
            m.nav_move_to(cx.w, [t.position.x, t.position.y, t.position.z], CHASE_SPEED);
            self.repath = 10;
        }
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
