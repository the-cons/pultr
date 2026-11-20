//! `config.toml`: where it lives, what it may contain, and loading it.
//!
//! Parsing is pure (`Config::parse`), so almost everything is testable without files.

use std::{
    collections::HashSet,
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use serde::Deserialize;

/// The whole file. Every section is optional; an empty file is a valid config.
#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub ui: Ui,

    /// `[[card]]` in the file: one table per card, in grid order.
    #[serde(rename = "card")]
    pub cards: Vec<CardConfig>,

    /// `[[scene]]`: shape arrives in M9. Accepted (and ignored) so example configs load today.
    #[serde(rename = "scene")]
    pub scenes: Vec<toml::Table>,
}

/// `[ui]`
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    pub theme: String,
    pub columns: usize,
    pub border: Border,
}

impl Default for Ui {
    fn default() -> Self {
        Self { theme: "default".to_string(), columns: 3, border: Border::default() }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Border {
    Plain,
    #[default]
    Rounded,
    Double,
    None,
}

/// One `[[card]]`. Only the fields every card has are typed here.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CardConfig {
    /// Unique, used in action addresses: `airpods.toggle`.
    pub id: String,

    /// `type` in the file (a keyword in Rust): `battery`, `bluetooth`, …
    #[serde(rename = "type")]
    pub kind: String,

    pub title: Option<String>,

    /// Everything else: card-specific fields like `device`. Later steps promote
    /// common ones (`pos`, `span`, `keys`) to real fields above.
    #[serde(flatten)]
    pub options: toml::Table,
}

impl Config {
    /// Parse and validate. The error says what's wrong and where (line, column).
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    /// Rules serde can't express on its own.
    fn validate(&self) -> anyhow::Result<()> {
        if self.ui.columns == 0 {
            bail!("ui.columns must be at least 1");
        }

        let mut seen = HashSet::new();
        for card in &self.cards {
            if !is_valid_id(&card.id) {
                bail!("card id `{}`: use only letters, digits, `-` and `_`", card.id);
            }
            if !seen.insert(card.id.as_str()) {
                bail!("duplicate card id `{}`", card.id);
            }
        }
        Ok(())
    }
}

/// Ids end up in `<card>.<action>` addresses and on the command line: keep them boring.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `--config <path>` must exist. The default file may be missing: then you get the defaults.
pub fn load(explicit: Option<&Path>) -> anyhow::Result<Config> {
    match explicit {
        Some(path) => read(path, true),
        None => match default_path() {
            Some(path) => read(&path, false),
            None => Ok(Config::default()),
        },
    }
}

fn read(path: &Path, required: bool) -> anyhow::Result<Config> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound && !required => return Ok(Config::default()),
        Err(e) => return Err(e).with_context(|| format!("can't read {}", path.display())),
    };
    Config::parse(&text).with_context(|| format!("invalid config: {}", path.display()))
}

/// `$XDG_CONFIG_HOME/pultr/config.toml`, else `~/.config/pultr/config.toml`. Same on macOS and Linux.
pub fn default_path() -> Option<PathBuf> {
    path_from(env::var_os("XDG_CONFIG_HOME"), env::var_os("HOME"))
}

/// Pure half of [`default_path`]: env values in, path out.
fn path_from(xdg_config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    // The XDG spec says relative paths are invalid and must be ignored. Same for an empty HOME.
    let absolute = |p: PathBuf| p.is_absolute().then_some(p);

    let base = xdg_config_home
        .map(PathBuf::from)
        .and_then(absolute)
        .or_else(|| home.map(PathBuf::from).and_then(absolute).map(|h| h.join(".config")))?;

    Some(base.join("pultr").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(text: &str) -> String {
        format!("{:#}", Config::parse(text).unwrap_err())
    }

    #[test]
    fn empty_file_is_the_default() {
        let config = Config::parse("").unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.ui.columns, 3);
        assert_eq!(config.ui.border, Border::Rounded);
    }

    #[test]
    fn example_config_in_repo_is_valid() {
        let config = Config::parse(include_str!("../../../config.example.toml")).unwrap();
        let ids: Vec<&str> = config.cards.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(ids, ["headphones", "ipad", "battery", "music", "out"]);
    }

    #[test]
    fn card_fields() {
        let config = Config::parse(
            r#"
            [[card]]
            id     = "airpods"
            type   = "bluetooth"
            title  = "AirPods"
            device = "aa-bb"
            "#,
        )
        .unwrap();

        let card = &config.cards[0];
        assert_eq!(card.id, "airpods");
        assert_eq!(card.kind, "bluetooth");
        assert_eq!(card.title.as_deref(), Some("AirPods"));
        assert_eq!(card.options["device"].as_str(), Some("aa-bb")); // card-specific → options
    }

    #[test]
    fn typos_are_errors_not_silence() {
        assert!(err("[[cards]]\nid = \"x\"").contains("unknown field `cards`"));
        assert!(err("[ui]\ncolums = 2").contains("unknown field `colums`"));
        assert!(err("[ui]\nborder = \"fancy\"").contains("unknown variant `fancy`"));
    }

    #[test]
    fn card_needs_id_and_type() {
        assert!(err("[[card]]\ntype = \"battery\"").contains("missing field `id`"));
        assert!(err("[[card]]\nid = \"battery\"").contains("missing field `type`"));
    }

    #[test]
    fn validation() {
        assert_eq!(err("[ui]\ncolumns = 0"), "ui.columns must be at least 1");
        assert_eq!(
            err("[[card]]\nid = \"a\"\ntype = \"x\"\n[[card]]\nid = \"a\"\ntype = \"y\""),
            "duplicate card id `a`"
        );
        assert!(err("[[card]]\nid = \"my.card\"\ntype = \"x\"").starts_with("card id `my.card`"));
        assert!(err("[[card]]\nid = \"\"\ntype = \"x\"").starts_with("card id ``"));
    }

    #[test]
    fn path_prefers_xdg() {
        let path = path_from(Some("/xdg".into()), Some("/home/me".into()));
        assert_eq!(path, Some(PathBuf::from("/xdg/pultr/config.toml")));
    }

    #[test]
    fn path_falls_back_to_home() {
        let expected = Some(PathBuf::from("/home/me/.config/pultr/config.toml"));
        assert_eq!(path_from(None, Some("/home/me".into())), expected);
        assert_eq!(path_from(Some("relative".into()), Some("/home/me".into())), expected);
        assert_eq!(path_from(Some("".into()), Some("/home/me".into())), expected);
    }

    #[test]
    fn no_home_no_path() {
        assert_eq!(path_from(None, None), None);
        assert_eq!(path_from(None, Some("".into())), None);
    }

    #[test]
    fn missing_file() {
        let path = env::temp_dir().join("pultr-test-does-not-exist").join("config.toml");
        assert_eq!(read(&path, false).unwrap(), Config::default());
        assert!(format!("{:#}", read(&path, true).unwrap_err()).starts_with("can't read"));
    }
}
