// The keys of a player riding a vulture (SPECS §6.2): sneak in the air makes it descend instead of
// getting off; only on the ground does sneak dismount, as on any mount. Horizon dismounts on sneak at
// the start of `tick_mount_riding`, before any mod hears of it: this runs the function again without
// sneak, then tells the vulture's `Fly` goal through `Rider.sprint` (Ctrl, otherwise unused in flight,
// descends too). Workaround for arismoko/horizon#2739.
fn vulture_ride_keys(game: &mut Game<'_>, keys: Keys) -> ::core::ops::ControlFlow<bool> {
    use ::core::ops::ControlFlow;
    if !keys.shift {
        return ControlFlow::Continue(());
    }
    let Some(i) = game.mount_index() else {
        return ControlFlow::Continue(());
    };
    let m = &game.mobs[i];
    if m.kind.id != "vulture:vulture" || m.on_ground {
        return ControlFlow::Continue(());
    }
    let id = m.id;
    let ridden = game.tick_mount_riding(Keys { shift: false, ..keys });
    if let Some(m) = game.mobs.iter_mut().find(|m| m.id == id)
        && let Some(r) = m.rider.as_mut()
    {
        r.sprint = true;
    }
    ControlFlow::Break(ridden)
}
