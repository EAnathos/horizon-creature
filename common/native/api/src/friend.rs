//! Friends (SPECS §6.2): a creature befriended by a player, who follows them or stays where it is told.
//!
//! The friend is saved on the entity (`creature:friend`: the player's UUID and whether it stays). The mod's goals read
//! it through a static map (`of`), filled by `tick` from the mod's `entity_tick_post` (single player, as the territory;
//! arismoko/horizon#2740); a change (`befriend`, `set_staying`) is saved by the next `tick`. A glass jar keeps the friend
//! of the creature it holds (`jar`).
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, Tag};

/// Whom a creature is the friend of, and whether it stays where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Friend {
    /// The player's UUID as four ints (`UUIDUtil.uuidToIntArray`, as `PlayerView.uuid` holds it).
    pub uuid: [i32; 4],
    pub staying: bool,
}

impl Friend {
    /// Whether `player` (as a server hook gives it) is this friend.
    pub fn is(&self, player: u128) -> bool {
        self.uuid == ints(player)
    }
}

/// A player's UUID as four ints, as `Friend.uuid` holds it.
pub fn ints(uuid: u128) -> [i32; 4] {
    [
        (uuid >> 96) as u32 as i32,
        (uuid >> 64) as u32 as i32,
        (uuid >> 32) as u32 as i32,
        uuid as u32 as i32,
    ]
}

/// A UUID back from its four ints.
pub fn uuid(ints: [i32; 4]) -> u128 {
    ints.iter()
        .fold(0u128, |acc, part| acc << 32 | u128::from(*part as u32))
}

static FRIEND: AttachmentType<Option<Friend>> = AttachmentType {
    namespace: "creature",
    name: "friend",
    default: || None,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

fn encode(friend: &Option<Friend>) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    if let Some(f) = friend {
        c.insert("Friend".into(), Tag::IntArray(f.uuid.to_vec()));
        c.insert("Staying".into(), Tag::Byte(i8::from(f.staying)));
    }
    Ok(c)
}

fn decode(c: &Compound) -> Result<Option<Friend>, AttachmentError> {
    Ok(match c.get(&"Friend".into()) {
        Some(Tag::IntArray(a)) if a.len() == 4 => Some(Friend {
            uuid: [a[0], a[1], a[2], a[3]],
            staying: matches!(c.get(&"Staying".into()), Some(Tag::Byte(1))),
        }),
        _ => None,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct Known {
    friend: Option<Friend>,
    /// Changed since it was saved.
    dirty: bool,
}

static FRIENDS: Mutex<BTreeMap<u32, Known>> = Mutex::new(BTreeMap::new());

/// `entity` becomes the friend of `player` (following them), saved at once.
pub fn befriend(host: &mut dyn Host, entity: u32, player: u128) {
    let friend = Some(Friend {
        uuid: ints(player),
        staying: false,
    });
    let saved = FRIEND.set(host, Holder::Entity(entity), friend).is_ok();
    FRIENDS.lock().unwrap().insert(entity, Known { friend, dirty: !saved });
}

/// Whose friend `entity` is, if it has one (from a goal or a hook).
pub fn of(entity: u32) -> Option<Friend> {
    FRIENDS.lock().unwrap().get(&entity).and_then(|k| k.friend)
}

/// Told to stay where it is, or to follow again; saved by the next `tick`.
pub fn set_staying(entity: u32, staying: bool) {
    if let Some(k) = FRIENDS.lock().unwrap().get_mut(&entity)
        && let Some(f) = k.friend.as_mut()
    {
        f.staying = staying;
        k.dirty = true;
    }
}

/// From the mod's `entity_tick_post`: its saved friend the first time, a change saved.
pub fn tick(host: &mut dyn Host, entity: u32) {
    let known = FRIENDS.lock().unwrap().get(&entity).copied();
    match known {
        None => {
            // Not readable yet (its first tick after loading): next tick.
            let Ok(saved) = FRIEND.get_existing(host, Holder::Entity(entity)) else {
                return;
            };
            FRIENDS.lock().unwrap().insert(
                entity,
                Known {
                    friend: saved.flatten(),
                    dirty: false,
                },
            );
        }
        Some(k) if k.dirty => {
            if FRIEND.set(host, Holder::Entity(entity), k.friend).is_ok()
                && let Some(k) = FRIENDS.lock().unwrap().get_mut(&entity)
            {
                k.dirty = false;
            }
        }
        Some(_) => {}
    }
}

/// From the mod's `living_death` (or when it is caught in a jar).
pub fn forget(entity: u32) {
    FRIENDS.lock().unwrap().remove(&entity);
}
