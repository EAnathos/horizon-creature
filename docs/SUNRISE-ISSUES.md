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
| [#3814](https://github.com/sunrise-org/sunrise/issues/3814) | `EntityRef::merge_nbt` answers true on a dolphin, but `GotFish` stays `0b` (`/data merge entity` keeps it) | open | The flying fish's dolphin is fed through `EntityRef::run_command` (`mods/flying_fish/src/lib.rs`, `fed`) | Back to `merge_nbt` |
| [#3815](https://github.com/sunrise-org/sunrise/issues/3815) | A cursor a mod changes from `menu_clicked` (`MenuRef::cursor`) isn't sent to the client, which shows a ghost stack until its next click | open | The vulture's screen opens again in place after a throw (`mods/vulture/src/screen.rs`, `clicked`) | Drop the reopening |
| [#3816](https://github.com/sunrise-org/sunrise/issues/3816) | A mod's living type can't be disallowed in Peaceful: the egg's "Disabled in Peaceful", `/summon` and spawns follow the template | open (a request) | The cyclops goes in Peaceful by itself, and its egg's line comes from a static its server tick keeps (`mods/cyclops/src/lib.rs`, `PEACEFUL`), single player only | Use the new key, drop the static |

## Limits of our workarounds

| Workaround | Its limits |
|---|---|
| The book's offset (#1134) | Reads our box from the library's table (`creature::mob::size_of`), not from the entity |
| The vulture's screen opened again after a throw (cursor, below) | What is left on the cursor after a right-click throw goes back to the inventory |

## To open (candidates)

Found, not filed: check the tracker and the newest Nightly first. None at the moment.

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
