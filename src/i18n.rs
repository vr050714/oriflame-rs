//! Translations of every text shown on screen.
//!
//! Messages are written in [Fluent](https://projectfluent.org) and live in
//! `locales/<tag>.ftl`, one file per language. The files are compiled into the
//! executable. A message missing from a language falls back to English.
//!
//! To add a language: add a variant to [`Language`], fill in its three
//! methods, and add `locales/<tag>.ftl` with the same keys as `en.ftl`.
//!
//! Text is translated when the interface is drawn, so changing the language
//! and setting `UiDirty` is enough to redraw everything, the log included.

use bevy::log::warn;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use oriflamme_core::CardKind;
use std::sync::{
    LazyLock,
    atomic::{AtomicU8, Ordering},
};
use unic_langid::LanguageIdentifier;

/// A language the interface can be shown in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    Russian,
    Serbian,
}

impl Language {
    /// Every language, in the order of the language menu.
    pub const ALL: [Language; 3] = [Language::English, Language::Russian, Language::Serbian];

    /// BCP 47 tag, also the name of the `.ftl` file.
    fn tag(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Russian => "ru",
            Language::Serbian => "sr-Cyrl",
        }
    }

    /// The Fluent source of this language.
    fn source(self) -> &'static str {
        match self {
            Language::English => include_str!("../locales/en.ftl"),
            Language::Russian => include_str!("../locales/ru.ftl"),
            Language::Serbian => include_str!("../locales/sr-Cyrl.ftl"),
        }
    }

    /// The language's name in that language, for the language menu.
    pub fn native_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Russian => "Русский",
            Language::Serbian => "Српски",
        }
    }

    fn index(self) -> u8 {
        match self {
            Language::English => 0,
            Language::Russian => 1,
            Language::Serbian => 2,
        }
    }
}

/// The language in use, as [`Language::index`].
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Switches the interface language. Redraw the interface afterwards.
pub fn set_language(language: Language) {
    CURRENT.store(language.index(), Ordering::Relaxed);
}

/// The language in use.
pub fn language() -> Language {
    let index = CURRENT.load(Ordering::Relaxed);
    Language::ALL
        .into_iter()
        .find(|language| language.index() == index)
        .unwrap_or_default()
}

/// One parsed bundle per language, built on first use.
static BUNDLES: LazyLock<Vec<(Language, FluentBundle<FluentResource>)>> =
    LazyLock::new(|| Language::ALL.into_iter().map(|l| (l, bundle(l))).collect());

fn bundle(language: Language) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = language.tag().parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // No invisible direction marks around arguments: the font has no glyphs
    // for them, and every supported language runs left to right.
    bundle.set_use_isolating(false);
    let resource = FluentResource::try_new(language.source().to_owned()).unwrap_or_else(
        |(resource, errors)| {
            warn!("{}.ftl has syntax errors: {errors:?}", language.tag());
            resource
        },
    );
    if let Err(errors) = bundle.add_resource(resource) {
        warn!("{}.ftl has duplicate messages: {errors:?}", language.tag());
    }
    bundle
}

/// Formats message `key` in `language`, or `None` when it has no such message.
fn format(language: Language, key: &str, args: Option<&FluentArgs>) -> Option<String> {
    let (_, bundle) = BUNDLES.iter().find(|(l, _)| *l == language)?;
    let pattern = bundle.get_message(key)?.value()?;
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, args, &mut errors);
    if !errors.is_empty() {
        warn!("message {key} in {}: {errors:?}", language.tag());
    }
    Some(text.into_owned())
}

/// Message `key` in the current language, with named arguments. Prefer the
/// [`t!`](crate::t) macro.
pub fn translate(key: &str, args: &[(&str, FluentValue)]) -> String {
    let args = (!args.is_empty()).then(|| args.iter().cloned().collect::<FluentArgs>());
    format(language(), key, args.as_ref())
        .or_else(|| format(Language::English, key, args.as_ref()))
        .unwrap_or_else(|| {
            warn!("no message {key}");
            key.to_owned()
        })
}

/// Translates a message: `t!("key")` or `t!("key", name = value, ...)`.
/// Values are numbers or strings.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::translate($key, &[])
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::translate(
            $key,
            &[$((stringify!($name), ::fluent_bundle::FluentValue::from($value))),+],
        )
    };
}

/// Name of a card.
pub fn card_name(kind: CardKind) -> String {
    translate(&format!("card-{}", kind.slug()), &[])
}

/// Rules text of a card.
pub fn card_description(kind: CardKind) -> String {
    translate(&format!("card-{}-text", kind.slug()), &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids of the messages in a Fluent source: lines that start with a
    /// lower-case letter, up to the `=`.
    fn keys(language: Language) -> Vec<&'static str> {
        let mut keys: Vec<&str> = language
            .source()
            .lines()
            .filter(|line| line.starts_with(|c: char| c.is_ascii_lowercase()))
            .filter_map(|line| line.split_once('=').map(|(id, _)| id.trim()))
            .collect();
        keys.sort_unstable();
        keys
    }

    /// Every language has the English messages, and only those, and every
    /// file parses.
    #[test]
    fn languages_have_the_same_messages() {
        let english = keys(Language::English);
        assert!(english.len() > 50);
        for language in Language::ALL {
            assert_eq!(keys(language), english, "{}", language.tag());
            assert!(FluentResource::try_new(language.source().to_owned()).is_ok());
            for key in &english {
                assert!(format(language, key, None).is_some(), "{key}");
            }
        }
    }

    #[test]
    fn formats_arguments() {
        let text = translate("round-of", &[("round", 2.into()), ("rounds", 6.into())]);
        assert_eq!(text, "Round 2 of 6");
    }
}
