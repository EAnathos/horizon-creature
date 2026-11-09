// Right-click on a vulture (SPECS §6): mounts it, when its owner clicks it with a harness on, an empty hand (or
// anything but what the vulture's `interaction.rs` handles) and not sneaking. Compiled into the game, it calls
// `start_riding` (the trait hook cannot run `/ride` in single player: `native_run_command` refuses during a player's
// turn). Everything else is left to the vulture's `interact_entity` trait hook: this sees a copy of the mob that the
// game refreshes from the one its AI ticks, so it only reads it (`tame`, `owner`), and whether it may be ridden
// (a harness on, grown up) from the `vulture:rideable` attachment the vulture's code keeps and syncs here.
fn vulture_interact(game: &mut crate::game::Game<'_>, entity: i32, kind: &str, claimed: &mut bool) {
    use crate::mod_api::{AttachmentCodec, AttachmentType, Holder, Host, Tag};
    static RIDEABLE: AttachmentType<bool> = AttachmentType {
        namespace: "vulture",
        name: "rideable",
        default: || false,
        codec: Some(AttachmentCodec {
            encode: |v| Ok([("Rideable".into(), Tag::Byte(i8::from(*v)))].into()),
            decode: |c| Ok(matches!(c.get(&"Rideable".into()), Some(Tag::Byte(1)))),
        }),
        copy_on_death: false,
        sync: true,
    };
    if *claimed || kind != "vulture:vulture" || game.sneaking() {
        return;
    }
    let held = game.held().map(|s| s.item.to_string()).unwrap_or_default();
    let used = matches!(
        held.as_str(),
        "vulture:harness"
            | "minecraft:chest"
            | "minecraft:rotten_flesh"
            | "minecraft:chicken"
            | "minecraft:beef"
            | "minecraft:porkchop"
            | "minecraft:mutton"
            | "minecraft:rabbit"
    );
    let Ok(id) = u32::try_from(entity) else {
        return;
    };
    // The clicking player, as the server knows them (`level.player_uuid` isn't set here).
    let Some(me) = game.acting_player().map(|p| p.uuid) else {
        return;
    };
    let me = [(me >> 96) as i32, (me >> 64) as i32, (me >> 32) as i32, me as i32];
    let Some(m) = game.mobs.iter().find(|m| m.id == id && m.alive()) else {
        return;
    };
    let owned = m.extra.tame && crate::mobs::tame::owned_by(m.extra.owner, me);
    let host: &mut dyn Host = game;
    let rideable = matches!(RIDEABLE.get_existing(host, Holder::Entity(id)), Ok(Some(true)));
    if used || !owned || !rideable || game.riding.is_some() || !game.native_may_board(id) {
        return;
    }
    let Some(m) = game.mobs.iter_mut().find(|m| m.id == id) else {
        return;
    };
    m.extra.flag = false;
    m.extra.counter = 0;
    let yaw = m.yaw;
    *claimed = true;
    game.yaw = yaw;
    game.mount.jump_ticks = 0;
    game.mount.jump_scale = 0.0;
    game.start_riding(id);
}
