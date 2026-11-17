//! Its drawing's poses: strides, the club's blow, the boulder lifted and hurled, dazed.
use super::*;

/// The club raised over its head, then brought down (degrees on the right arm), `since` seconds after it was raised.
pub(crate) fn club(since: f32) -> f32 {
    let up = WINDUP as f32 / 20.0;
    if since < up {
        let t = since / up;
        -170.0 * t * (2.0 - t)
    } else if since < up + 0.12 {
        -170.0 + 150.0 * (since - up) / 0.12
    } else if since < up + 0.6 {
        -20.0 * (1.0 - (since - up - 0.12) / 0.48)
    } else {
        0.0
    }
}

/// The boulder raised over its head with both hands, then hurled forward (degrees on both arms), `since` seconds after
/// it was lifted.
pub(crate) fn hurl(since: f32) -> f32 {
    let up = THROW_WINDUP as f32 / 20.0;
    if since < up {
        let t = since / up;
        -170.0 * t * (2.0 - t)
    } else if since < up + 0.15 {
        -170.0 + 130.0 * (since - up) / 0.15
    } else if since < up + 0.6 {
        -40.0 * (1.0 - (since - up - 0.15) / 0.45)
    } else {
        0.0
    }
}

/// What it is doing, for its drawing: seconds since it raised its club, lifted a boulder, was struck in the eye.
pub(crate) struct Moment {
    raised: Option<f32>,
    lifted: Option<f32>,
    dazed: Option<f32>,
}

impl Moment {
    pub(crate) fn of(id: u32) -> Moment {
        let since = |m: &Mutex<BTreeMap<u32, Instant>>| {
            m.lock()
                .unwrap()
                .get(&id)
                .map(|at| at.elapsed().as_secs_f32())
                .filter(|s| *s < 2.0)
        };
        Moment {
            raised: since(&RAISED),
            lifted: since(&LIFTED),
            dazed: eye::dazed(id),
        }
    }
    /// Whether it holds a boulder over its head now.
    pub(crate) fn holds_boulder(&self) -> bool {
        self.lifted.is_some_and(|s| s < THROW_WINDUP as f32 / 20.0)
    }
}

/// A group's rotation this frame: heavy strides, arms swinging, the head looking about, the club's blow, the boulder
/// lifted and hurled, the head lolling while dazed.
pub(crate) fn pose(name: &str, mob: &NativeMobRender<'_>, now: &Moment) -> [f32; 3] {
    let stride = ((mob.walk_position * 0.6662).cos() * 1.4 * mob.walk_speed).to_degrees() * 0.6;
    let blow = now.raised.map_or(0.0, club);
    let hurl = now.lifted.map_or(0.0, hurl);
    let loll = now.dazed.map_or(0.0, |s| (s * 9.0).sin() * 18.0);
    match name {
        "head" => [mob.pitch + loll.abs() * 0.5, -mob.head_yaw, loll],
        "leg_left" => [stride, 0.0, 0.0],
        "leg_right" => [-stride, 0.0, 0.0],
        "arm_left" => [-stride * 0.6 + hurl, 0.0, 0.0],
        "arm_right" => [stride * 0.6 + blow + hurl, 0.0, 0.0],
        "body" => [(blow + hurl) * 0.06, 0.0, 0.0],
        _ => [0.0; 3],
    }
}
