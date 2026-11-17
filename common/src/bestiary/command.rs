//! The operators' `/bestiary` command, to check the pages: `unlock <species|all>` writes everything down for the
//! player who runs it, `reset` forgets everything.
use super::*;

/// The operators' command and its operator level.
const COMMAND: &str = "bestiary";
const COMMAND_LEVEL: u8 = 2;

/// From the library's `register_commands`.
pub fn commands(commands: &mut Vec<Command>) {
    commands.push(Command::op(COMMAND, COMMAND_LEVEL));
    commands.push(Command::op(&format!("{COMMAND} unlock <species:text>"), COMMAND_LEVEL));
    commands.push(Command::op(&format!("{COMMAND} reset"), COMMAND_LEVEL));
}

/// Every look `scheme` can roll, by key: the colours, patterns and pattern colours some variety gives a weight; a
/// pattern with no texture once, its pattern colour unseen.
fn every_morph(scheme: &Scheme) -> Vec<String> {
    let mut keys = Vec::new();
    for v in scheme.varieties {
        let given =
            |list: &[(usize, u32)]| -> Vec<usize> { list.iter().filter(|(_, w)| *w > 0).map(|(i, _)| *i).collect() };
        let pattern_colours = given(v.pattern_colours);
        for &base in &given(v.base) {
            for &pattern in &given(v.patterns) {
                let plain = scheme.patterns.get(pattern).is_none_or(|p| p.texture.is_none());
                let colours = if plain {
                    &pattern_colours[..pattern_colours.len().min(1)]
                } else {
                    &pattern_colours[..]
                };
                for &pattern_colour in colours {
                    let key = colours::key(
                        scheme,
                        Look {
                            base,
                            pattern,
                            pattern_colour,
                        },
                    );
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
            }
        }
    }
    keys
}

/// Writes down everything about `species` for `player`: every biome it spawns in (not a whole dimension's, which has
/// no list), every fact and every look.
fn unlock(host: &mut dyn ServerHost, player: u128, species: &Species) {
    let morphs = species.scheme.map(every_morph).unwrap_or_default();
    update(host, player, species.id, |e| {
        for b in species.biomes.iter().filter(|b| !b.starts_with('@')) {
            if !e.biomes.iter().any(|known| known == b) {
                e.biomes.push((*b).to_owned());
            }
        }
        for f in species.facts {
            if !e.facts.iter().any(|known| known == f.id) {
                e.facts.push(f.id.to_owned());
            }
        }
        for m in morphs {
            if !e.morphs.contains(&m) {
                e.morphs.push(m);
            }
        }
    });
}

/// The species a command names: its id (`minecraft:cow`), or the part after the colon when one species has it
/// (`cow`).
fn named(name: &str) -> Option<&'static Species> {
    let all = SPECIES.lock().unwrap();
    if let Some(s) = all.iter().find(|s| s.id == name) {
        return Some(s);
    }
    let mut short = all
        .iter()
        .filter(|s| s.id.split_once(':').is_some_and(|(_, path)| path == name));
    match (short.next(), short.next()) {
        (Some(s), None) => Some(s),
        _ => None,
    }
}

fn answer(host: &mut dyn ServerHost, key: &str, args: Vec<Text>) {
    let _ = host.commands().feedback(Text::translate(key, args));
}

/// From the library's `run_mod_command`: the operators' `/bestiary`, for the player who runs it. Whether it was ours.
pub fn perform(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> bool {
    let literals = &event.command.literals;
    if literals.first().map(String::as_str) != Some(COMMAND) {
        return false;
    }
    let CommandSource::Player { player, .. } = event.source else {
        return true;
    };
    let player = player.uuid();
    let species = match event.command.argument("species") {
        Some(CommandArgumentValue::String(s)) => Some(s.as_str()),
        _ => None,
    };
    match (literals.get(1).map(String::as_str), species) {
        (None, _) => book::open_for_player(host, player),
        (Some("reset"), None) => {
            store(host, player, &[]);
            answer(host, "commands.creature.bestiary.reset", vec![]);
        }
        (Some("unlock"), Some("all")) => {
            let all: Vec<&'static Species> = SPECIES.lock().unwrap().clone();
            for s in &all {
                unlock(host, player, s);
            }
            answer(
                host,
                "commands.creature.bestiary.unlocked_all",
                vec![Text::literal(all.len().to_string())],
            );
        }
        (Some("unlock"), Some(name)) => match named(name) {
            Some(s) => {
                unlock(host, player, s);
                answer(
                    host,
                    "commands.creature.bestiary.unlocked",
                    vec![Text::translate(entity_key(s.id), vec![])],
                );
            }
            None => answer(host, "commands.creature.bestiary.unknown", vec![Text::literal(name)]),
        },
        _ => answer(host, "commands.creature.bestiary.usage", vec![]),
    }
    true
}

/// From the library's `tab_complete`: `/bestiary unlock` completes `all` and the species' ids.
pub fn complete(event: &mut TabCompleteEvent<'_>) {
    let words: Vec<&str> = event.command.trim_start_matches('/').split(' ').collect();
    if words.len() != 3 || words[0] != COMMAND || words[1] != "unlock" {
        return;
    }
    let typed = words[2];
    let ids: Vec<&'static str> = SPECIES.lock().unwrap().iter().map(|s| s.id).collect();
    for id in std::iter::once("all").chain(ids) {
        let short = id.split_once(':').map_or(id, |(_, path)| path);
        if id.starts_with(typed) || short.starts_with(typed) {
            event
                .suggestions
                .push(Suggestion::new((event.range[0], event.range[1]), id));
        }
    }
}
