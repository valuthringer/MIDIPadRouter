//! Serde models, validation, migrations and atomic persistence of the JSON config.

use crate::devices::{Key, KeyKind, DEVICE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub const CURRENT_VERSION: u64 = 1;
pub const MAX_TARGETS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Switch,
    Combined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRef {
    #[serde(rename = "type")]
    pub kind: KeyKind,
    pub number: u8,
}

impl KeyRef {
    pub fn key(&self) -> Option<Key> {
        DEVICE.key_from_midi(self.kind, self.number)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCfg {
    pub id: String,
    pub name: String,
    pub color: String,
    pub enabled: bool,
    /// Port we send pad events to (the application listens on it).
    pub port_to_app: String,
    /// Port we read LED messages from (the application writes to it).
    pub port_from_app: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub mode: Mode,
    pub active_target: String,
    pub switch_key: Option<KeyRef>,
    pub shortcuts: BTreeMap<String, String>,
    pub targets: Vec<TargetCfg>,
    /// "pad:<note>" / "cc:<number>" -> target id. Absent = no owner.
    pub assignment: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub language: String,
    pub autostart: bool,
    pub start_minimized: bool,
    pub theme: String,
    /// First-run wizard completed.
    pub setup_done: bool,
    /// macOS/Linux: the app creates its own virtual ports.
    pub virtual_ports: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "fr".into(),
            autostart: false,
            start_minimized: false,
            theme: "auto".into(),
            setup_done: false,
            virtual_ports: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub version: u64,
    pub active_profile: String,
    pub profiles: BTreeMap<String, Profile>,
    #[serde(default)]
    pub settings: Settings,
}

/// Error shown to the user; `code` is translated by the front-end (`err.<code>`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConfigError {
    pub code: &'static str,
    pub arg: String,
}

pub fn err(code: &'static str, arg: impl Into<String>) -> ConfigError {
    ConfigError {
        code,
        arg: arg.into(),
    }
}

/// "pad:12" / "cc:104" -> key index.
pub fn parse_key_id(s: &str) -> Option<Key> {
    let (kind, n) = s.split_once(':')?;
    let kind = match kind {
        "pad" => KeyKind::Pad,
        "cc" => KeyKind::Cc,
        _ => return None,
    };
    DEVICE.key_from_midi(kind, n.parse().ok()?)
}

pub fn key_id(key: Key) -> String {
    match DEVICE.key_to_midi(key) {
        (KeyKind::Pad, n) => format!("pad:{n}"),
        (KeyKind::Cc, n) => format!("cc:{n}"),
    }
}

impl Default for Config {
    fn default() -> Self {
        let target = |id: &str, name: &str, color: &str| TargetCfg {
            id: id.into(),
            name: name.into(),
            color: color.into(),
            enabled: true,
            port_to_app: format!("MIDIPadRouter {} in", id.to_uppercase()),
            port_from_app: format!("MIDIPadRouter {} out", id.to_uppercase()),
        };
        let switch_key = KeyRef {
            kind: KeyKind::Cc,
            number: 111,
        };
        // Left half (pads + top 104..107) to A, right half + scene buttons to B.
        let mut assignment = BTreeMap::new();
        for k in 0..crate::devices::KEY_COUNT {
            if Some(k) == switch_key.key() {
                continue;
            }
            let left = matches!(k, 0..=63 if k % 8 < 4) || matches!(k, 72..=75);
            assignment.insert(key_id(k), if left { "a" } else { "b" }.to_string());
        }
        let profile = Profile {
            mode: Mode::Switch,
            active_target: "a".into(),
            switch_key: Some(switch_key),
            shortcuts: [("a", "Ctrl+Alt+1"), ("b", "Ctrl+Alt+2")]
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            targets: vec![
                target("a", "Radio A", "#22c55e"),
                target("b", "Radio B", "#ef4444"),
            ],
            assignment,
        };
        Config {
            version: CURRENT_VERSION,
            active_profile: "Par défaut".into(),
            profiles: [("Par défaut".to_string(), profile)].into_iter().collect(),
            settings: Settings::default(),
        }
    }
}

impl Profile {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.targets.is_empty() {
            return Err(err("noTarget", ""));
        }
        if self.targets.len() > MAX_TARGETS {
            return Err(err("tooManyTargets", MAX_TARGETS.to_string()));
        }
        // Every port used once: a loopback used twice would feed our output back to us.
        let mut ports: Vec<&str> = Vec::new();
        for (i, t) in self.targets.iter().enumerate() {
            if t.id.is_empty() || self.targets[..i].iter().any(|o| o.id == t.id) {
                return Err(err("duplicateId", &t.id));
            }
            for p in [&t.port_to_app, &t.port_from_app] {
                if p.is_empty() {
                    continue;
                }
                if p.to_lowercase().contains("launchpad") {
                    return Err(err("launchpadPort", p));
                }
                if ports.contains(&p.as_str()) {
                    return Err(err("duplicatePort", p));
                }
                ports.push(p);
            }
        }
        let has = |id: &str| self.targets.iter().any(|t| t.id == id);
        let switch_key = match self.switch_key {
            Some(k) => Some(k.key().ok_or_else(|| err("badKey", format!("{k:?}")))?),
            None => None,
        };
        for (key, id) in &self.assignment {
            let k = parse_key_id(key).ok_or_else(|| err("badKey", key))?;
            if !has(id) {
                return Err(err("unknownTarget", id));
            }
            if Some(k) == switch_key {
                return Err(err("switchKeyAssigned", key));
            }
        }
        if let Some(id) = self.shortcuts.keys().find(|id| !has(id)) {
            return Err(err("unknownTarget", id));
        }
        Ok(())
    }
}

impl Config {
    pub fn active(&self) -> &Profile {
        &self.profiles[&self.active_profile]
    }

    pub fn active_mut(&mut self) -> &mut Profile {
        self.profiles
            .get_mut(&self.active_profile)
            .expect("validated")
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.profiles.contains_key(&self.active_profile) {
            return Err(err("unknownProfile", &self.active_profile));
        }
        for (name, p) in &self.profiles {
            if name.trim().is_empty() {
                return Err(err("emptyName", ""));
            }
            p.validate().map_err(|mut e| {
                if self.profiles.len() > 1 {
                    e.arg = format!("{} ({name})", e.arg);
                }
                e
            })?;
        }
        Ok(())
    }
}

/// Upgrade an older JSON document to `CURRENT_VERSION`.
fn migrate(v: Value) -> Result<Value, ConfigError> {
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    match version {
        // Add `n => upgrade_n_to_n_plus_1(v)` arms here when the format changes.
        CURRENT_VERSION => Ok(v),
        n if n > CURRENT_VERSION => Err(err("futureVersion", n.to_string())),
        n => Err(err("parse", format!("version {n}"))),
    }
}

pub fn parse(text: &str) -> Result<Config, ConfigError> {
    let v: Value = serde_json::from_str(text).map_err(|e| err("parse", e.to_string()))?;
    let c: Config = serde_json::from_value(migrate(v)?).map_err(|e| err("parse", e.to_string()))?;
    c.validate()?;
    Ok(c)
}

/// Load the config. A broken file is kept aside (never deleted) and defaults are used.
pub fn load_or_default(path: &Path) -> (Config, Option<ConfigError>) {
    match std::fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
        Err(e) => (Config::default(), Some(err("io", e.to_string()))),
        Ok(text) => match parse(&text) {
            Ok(c) => (c, None),
            Err(mut e) => {
                let backup = path.with_extension("invalid.json");
                let _ = std::fs::rename(path, &backup);
                e.arg = format!("{} → {}", e.arg, backup.display());
                (Config::default(), Some(e))
            }
        },
    }
}

/// Atomic write: temp file + rename (rename replaces the target on every OS).
pub fn write_atomic(path: &Path, text: &str) -> Result<(), ConfigError> {
    let io = |e: std::io::Error| err("io", e.to_string());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    let tmp = path.with_extension("tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).map_err(io)?;
        f.write_all(text.as_bytes()).map_err(io)?;
        f.sync_all().map_err(io)?;
    }
    std::fs::rename(&tmp, path).map_err(io)
}

pub fn save(path: &Path, c: &Config) -> Result<(), ConfigError> {
    write_atomic(
        path,
        &serde_json::to_string_pretty(c).expect("serializable"),
    )
}

/// Exported profile file.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFile {
    pub version: u64,
    pub name: String,
    pub profile: Profile,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Config::default().active().clone()
    }

    #[test]
    fn default_is_valid_and_round_trips() {
        let c = Config::default();
        c.validate().unwrap();
        let text = serde_json::to_string(&c).unwrap();
        assert_eq!(parse(&text).unwrap(), c);
    }

    #[test]
    fn spec_example_parses() {
        let text = r##"{
          "version": 1, "activeProfile": "default",
          "profiles": { "default": {
            "mode": "switch", "activeTarget": "a",
            "switchKey": { "type": "cc", "number": 111 },
            "shortcuts": { "a": "Ctrl+Alt+1" },
            "targets": [{ "id": "a", "name": "Radio A", "color": "#3b82f6", "enabled": true,
              "portToApp": "MIDIPadRouter A in", "portFromApp": "MIDIPadRouter A out" }],
            "assignment": { "pad:0": "a", "cc:104": "a" } } },
          "settings": { "language": "fr", "autostart": false, "startMinimized": false, "theme": "auto" }
        }"##;
        let c = parse(text).unwrap();
        assert_eq!(c.active().switch_key.unwrap().key(), Some(79));
    }

    #[test]
    fn validation_rules() {
        let mut p = profile();
        let base = p.targets[0].clone();
        p.targets.extend((0..4).map(|i| TargetCfg {
            id: format!("x{i}"),
            port_to_app: String::new(),
            port_from_app: String::new(),
            ..base.clone()
        }));
        assert_eq!(p.validate().unwrap_err().code, "tooManyTargets");

        let mut p = profile();
        p.targets[1].id = "a".into();
        assert_eq!(p.validate().unwrap_err().code, "duplicateId");

        let mut p = profile();
        p.targets[1].port_to_app = p.targets[0].port_from_app.clone();
        assert_eq!(p.validate().unwrap_err().code, "duplicatePort");

        let mut p = profile();
        p.targets[0].port_from_app = p.targets[0].port_to_app.clone();
        assert_eq!(p.validate().unwrap_err().code, "duplicatePort"); // feedback loop

        let mut p = profile();
        p.targets[0].port_to_app = "Launchpad Mini".into();
        assert_eq!(p.validate().unwrap_err().code, "launchpadPort");

        let mut p = profile();
        p.assignment.insert("pad:0".into(), "zz".into());
        assert_eq!(p.validate().unwrap_err().code, "unknownTarget");

        let mut p = profile();
        p.assignment.insert("cc:111".into(), "a".into());
        assert_eq!(p.validate().unwrap_err().code, "switchKeyAssigned");

        let mut p = profile();
        p.assignment.insert("pad:9".into(), "a".into());
        assert_eq!(p.validate().unwrap_err().code, "badKey");

        let mut p = profile();
        p.targets.clear();
        assert_eq!(p.validate().unwrap_err().code, "noTarget");
    }

    #[test]
    fn future_version_rejected() {
        let mut v = serde_json::to_value(Config::default()).unwrap();
        v["version"] = 99.into();
        assert_eq!(parse(&v.to_string()).unwrap_err().code, "futureVersion");
    }

    #[test]
    fn save_and_reload_identical() {
        let dir = std::env::temp_dir().join(format!("midipr-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let mut c = Config::default();
        c.active_mut().mode = Mode::Combined;
        save(&path, &c).unwrap();
        save(&path, &c).unwrap(); // overwrite works
        assert_eq!(load_or_default(&path), (c, None));
        std::fs::write(&path, "{ broken").unwrap();
        let (c2, e) = load_or_default(&path);
        assert_eq!(c2, Config::default());
        assert_eq!(e.unwrap().code, "parse");
        assert!(path.with_extension("invalid.json").exists()); // kept aside, not lost
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn key_ids_round_trip() {
        for k in 0..crate::devices::KEY_COUNT {
            assert_eq!(parse_key_id(&key_id(k)), Some(k));
        }
        assert_eq!(parse_key_id("note:1"), None);
    }
}
