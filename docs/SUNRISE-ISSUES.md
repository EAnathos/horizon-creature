# Sunrise issues

The issues we raised or follow on [Sunrise](https://github.com/sunrise-org/sunrise), and the workaround each one left in
our code (marked there with its issue). Update this page when an issue moves, and before moving the series' pin
(below). Issues go to [sunrise-org/sunrise](https://github.com/sunrise-org/sunrise/issues): search it first, open
and closed, and file only with a clean repro and the pinned revision. A fixed issue whose workaround is gone leaves
this page (its history is in git).

Our pin: Nightly `nightly-2026-11-20-9425651`, source `94256510f7` (every `sunrise-mod.toml`'s `[native]` pins and
CI's `SUNRISE_REV`). Last checked: 2026-11-20.

Status: **to open** (found, not filed), **open**, **fixed upstream** (merged, not yet in our pin).

## Open

| Issue | What | Status | Our workaround (where) | Drop it when fixed |
|---|---|---|---|---|
| [#1134](https://github.com/sunrise-org/sunrise/issues/1134) (not ours, we follow it) | `entity_size` isn't asked for a client's copies of entities: there, our creatures keep their template's box (a wandering trader's, 1.95 high) | open | The book centres our creatures by their own box, offset from the template's height (`common/src/bestiary/book.rs`, `TEMPLATE_HEIGHT`); the cyclops gets a client box through `minecraft:scale` (`creature::mob::scale_client_box`) | Drop the offset and the scale |

## Limits of our workarounds

| Workaround | Its limits |
|---|---|
| The book's offset (#1134) | Reads our box from the library's table (`creature::mob::size_of`), not from the entity |
| The vulture's screen opened again after a throw (cursor, below) | What is left on the cursor after a right-click throw goes back to the inventory |

## To open (candidates)

Found, not filed: check the tracker and the newest Nightly first.

- **A cursor a mod changes is not sent to the client**: from `menu_clicked`, writing a menu's cursor through
  `MenuRef::cursor` changes the server's, but the client keeps showing its own until its next click, as a ghost stack
  (seen when a click outside the window throws the cursor's stack: the client doesn't predict it). `menu_set_items`
  sends the slots only. Ours: the vulture's screen opens again in place after a throw (`mods/vulture/src/screen.rs`,
  `clicked`).
- **A mod mob's spawn egg never says "Disabled in Peaceful"**: the game decides it from the copied vanilla type (a
  mod mob copies a zombie, a villager or a wandering trader), with no way to call it a monster. Ours: the cyclops's
  tooltip adds the line from a static its server tick keeps (`mods/cyclops/src/lib.rs`, `PEACEFUL`), single player
  only.

## Moving the pin

1. Pick the Nightly (`sunrise-launcher versions`) and its commit: the newest, or the first with a fix we wait for
   (`git log <old>..<new>` in the Sunrise sources). `cargo sunrise-mod update --sunrise <commit>` in a module moves its
   pin and lists the API items it uses that changed.
2. Set the commit in every `sunrise-mod.toml` (`[native]`), CI's `SUNRISE_REV` and CONTRIBUTING's `REV`, and install
   `cargo-sunrise-mod` and `sunrise-stage` at it (CONTRIBUTING.md); `cargo creature native-check`
   fetches the new SDK by itself (it compares `target/sunrise-sdk/sdk.json` with the library's pin).
3. Install that Nightly and keep the instance on it (`sunrise-launcher install <nightly>`,
   `sunrise-launcher instance version creature-dev <nightly>`), or `cargo creature install` refuses the mods.
4. `cargo creature check`, `cargo creature install`; drop the workarounds it fixes, update the tables; test in game.
