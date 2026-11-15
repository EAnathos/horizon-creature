//! It fights back (SPECS §3): once the vanilla "hurt by" target goal sets its target (whoever hit it), the scorpion
//! runs at them and strikes about once a second, a pinch or, one strike in two, a poisoned sting. It lets a target go
//! out of `GIVE_UP` blocks, dead, or a creative player.
use horizon_client::mobs::goals;
use horizon_client::mobs::{Who, ai::do_hurt_target};
use horizon_client::mod_api::{Cx, Goal, Mob, RandomSource};

use crate::{kind, queue_sting, set_reared};

/// Within this it can strike (blocks); it chases up to `GIVE_UP`, striking every `EVERY` ticks, at `CHASE_SPEED`.
const REACH: f64 = 1.1;
const GIVE_UP: f64 = 12.0;
const EVERY: i32 = 20;
const CHASE_SPEED: f64 = 1.2;

/// The snap of its claws on a strike (`assets/sounds.toml`).
const STRIKE: &str = "scorpions:entity.scorpion.strike";

#[derive(Clone, Debug, Default)]
pub(crate) struct Fight {
    cooldown: i32,
    strikes: u32,
    repath: i32,
}

impl Goal for Fight {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        m.target_info(cx.w).is_some_and(|t| t.alive && !t.creative)
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        let Some(t) = m.target_info(cx.w) else {
            set_reared(m.id, false);
            return false;
        };
        let far = (t.position.x - m.position.x).hypot(t.position.z - m.position.z) > GIVE_UP;
        if far || !t.alive || t.creative {
            m.ai.target = None;
            set_reared(m.id, false);
            return false;
        }
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, m: &mut Mob, _: &mut Cx) {
        set_reared(m.id, true);
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        set_reared(m.id, false);
        m.ai.nav.stop();
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(who) = m.ai.target else {
            return;
        };
        let Some(t) = cx.w.target_info(who) else {
            return;
        };
        set_reared(m.id, true);
        m.ai.look.set_look_at([t.position.x, t.eye_y, t.position.z], 30.0, 30.0);
        let reach = (t.position.x - m.position.x).hypot(t.position.z - m.position.z);
        self.cooldown -= 1;
        if reach < REACH && self.cooldown <= 0 {
            m.swing();
            do_hurt_target(m, who, cx);
            self.strikes += 1;
            // One strike in two is a sting: its kind's poison, on the player (a mob takes the strike's damage only).
            if self.strikes.is_multiple_of(2) && who == Who::Player {
                queue_sting(kind(m.id));
            }
            creature::sounds::queue(m.id, STRIKE, 0.8, 0.9 + m.random.next_float() * 0.2);
            self.cooldown = EVERY;
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
