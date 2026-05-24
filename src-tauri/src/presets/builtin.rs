//! Built-in presets bundled at compile time.

use super::schema::Preset;

const RAW_PRESETS: &[(&str, &str)] = &[
    ("Xbox 360 Lobby",       include_str!("builtin_json/xbox-360-lobby.json")),
    ("Cheap Headset",        include_str!("builtin_json/cheap-headset.json")),
    ("Drive-Thru Speaker",   include_str!("builtin_json/drive-thru-speaker.json")),
    ("Broken Radio",         include_str!("builtin_json/broken-radio.json")),
    ("Discord Packet Loss",  include_str!("builtin_json/discord-packet-loss.json")),
    ("Deep Fried Mic",       include_str!("builtin_json/deep-fried-mic.json")),
    ("Tin Can",              include_str!("builtin_json/tin-can.json")),
    ("Underwater",           include_str!("builtin_json/underwater.json")),
    ("Fan Noise Hell",       include_str!("builtin_json/fan-noise-hell.json")),
    ("2007 Skype Call",      include_str!("builtin_json/2007-skype-call.json")),
    ("Blown Mic",            include_str!("builtin_json/blown-mic.json")),
];

pub fn all() -> Vec<Preset> {
    let mut out = Vec::with_capacity(RAW_PRESETS.len());
    for (label, raw) in RAW_PRESETS {
        match Preset::from_json_str(raw) {
            Ok(p) => out.push(p),
            Err(e) => panic!(
                "built-in preset '{}' failed to parse at compile time: {}",
                label, e
            ),
        }
    }
    out
}

pub fn by_name(name: &str) -> Option<Preset> {
    all().into_iter().find(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_built_in_presets_parse() {
        let presets = all();
        assert_eq!(presets.len(), 11, "expected 11 built-in presets");
    }

    #[test]
    fn each_preset_has_a_name() {
        for p in all() {
            assert!(!p.name.is_empty(), "preset missing name");
        }
    }

    #[test]
    fn xbox_360_lobby_round_trips() {
        let p = by_name("Xbox 360 Lobby").expect("Xbox 360 Lobby present");
        assert_eq!(p.effects.len(), 5);
        let types: Vec<&str> = p.effects.iter().map(|e| e.type_.as_str()).collect();
        assert_eq!(types, vec!["gain", "bandpass", "bitcrusher", "clipper", "packetLoss"]);
    }

    #[test]
    fn no_preset_contains_a_limiter_entry() {
        for p in all() {
            for e in &p.effects {
                assert_ne!(
                    e.type_, "limiter",
                    "preset '{}' must not list a limiter — chain builder appends one",
                    p.name
                );
            }
        }
    }
}
