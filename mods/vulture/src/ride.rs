//! Its rider (SPECS §6.2): who rides it and which keys they hold.
//!
//! The server hears each player's movement keys (`player_input`, vanilla's `ServerboundPlayerInputPacket` flags:
//! forward, backward, left, right, jump, sneak, sprint) and keeps them (`heard`). Each tick, before its behaviours run,
//! the vulture's rider is read (its first passenger: vanilla names a controlling one only on saddled mounts; the player's look and keys: `read`) for the ridden flight
//! (`Fly`). Sneaking in the air descends instead of getting off: the dismount is cancelled (`keep_seated`), on the
//! ground it gets off as on any mount.
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::ServerHost;

/// The rider of a vulture this tick: where they look and what they press.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rider {
    pub yaw: f32,
    /// 1 forward (W), -1 backward (S).
    pub forward: f32,
    /// 1 left (A), -1 right (D).
    pub strafe: f32,
    /// Space.
    pub jumping: bool,
    /// Sneak or sprint: down.
    pub descend: bool,
}

/// Vanilla's `Input` flags.
const FORWARD: i32 = 1;
const BACKWARD: i32 = 2;
const LEFT: i32 = 4;
const RIGHT: i32 = 8;
const JUMP: i32 = 16;
const SNEAK: i32 = 32;
const SPRINT: i32 = 64;

/// Each player's keys, as last heard.
static KEYS: Mutex<BTreeMap<u128, i32>> = Mutex::new(BTreeMap::new());
/// Each ridden vulture's rider this tick.
static RIDERS: Mutex<BTreeMap<u32, Rider>> = Mutex::new(BTreeMap::new());

/// From the `player_input` hook.
pub fn heard(player: u128, flags: i32) {
    KEYS.lock().unwrap().insert(player, flags);
}

/// From the vulture's tick, before its behaviours: its rider now, if a player rides it.
pub fn read(host: &dyn ServerHost, id: u32) {
    let rider = host
        .entity(id)
        .and_then(|e| e.passengers(host))
        .and_then(|passengers| passengers.into_iter().next())
        .and_then(|first| {
            let player = host
                .players()
                .server_players()
                .into_iter()
                .find(|p| p.entity().id() == first.id())?;
            let flags = KEYS.lock().unwrap().get(&player.uuid()).copied().unwrap_or(0);
            let yaw = player.entity().rotation(host)?[0];
            let axis = |plus: i32, minus: i32| {
                f32::from(i8::from(flags & plus != 0)) - f32::from(i8::from(flags & minus != 0))
            };
            Some(Rider {
                yaw,
                forward: axis(FORWARD, BACKWARD),
                strafe: axis(LEFT, RIGHT),
                jumping: flags & JUMP != 0,
                descend: flags & (SNEAK | SPRINT) != 0,
            })
        });
    let mut riders = RIDERS.lock().unwrap();
    match rider {
        Some(r) => riders.insert(id, r),
        None => riders.remove(&id),
    };
}

/// Its rider this tick, if any.
pub fn of(id: u32) -> Option<Rider> {
    RIDERS.lock().unwrap().get(&id).copied()
}

pub fn forget(id: u32) {
    RIDERS.lock().unwrap().remove(&id);
}

/// How far below its feet ground still counts as landed, in blocks.
const LANDED: f64 = 0.5;

/// From the `entity_mount` hook: a rider getting off vulture `vehicle` in the air (sneaking to descend) stays on. In
/// the air means no ground just under its feet: its `on_ground` flickers while it walks ridden (its flight's no
/// gravity), and a sneak there must get the rider off.
pub fn keep_seated(host: &dyn ServerHost, vehicle: u32) -> bool {
    let Some(e) = host.entity(vehicle) else {
        return false;
    };
    let (Some([x, y, z]), Some(level)) = (e.position(host), e.level(host)) else {
        return false;
    };
    let below = [x - 0.4, y - LANDED, z - 0.4, x + 0.4, y, z + 0.4];
    let grounded = level.collision_boxes(host, below).is_none_or(|b| !b.is_empty());
    !grounded && !e.is_in_water(host).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_make_the_rider_s_axes() {
        heard(7, FORWARD | LEFT | SNEAK);
        let flags = KEYS.lock().unwrap()[&7];
        assert_eq!(flags & FORWARD, FORWARD);
        assert_eq!(flags & (SNEAK | SPRINT), SNEAK);
    }
}
