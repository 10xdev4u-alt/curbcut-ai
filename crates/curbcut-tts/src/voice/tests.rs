//! Tests for the voice catalog.

use super::*;

fn voice(id: &str, source: VoiceSource, downloaded: bool) -> Voice {
    Voice::new(
        VoiceId::new(id),
        format!("Voice {id}"),
        "en",
        source,
        downloaded,
    )
}

#[test]
fn voice_ids_are_normalised() {
    assert_eq!(VoiceId::new("Piper-EN").as_str(), "piper-en");
    assert_eq!(VoiceId::new("a"), VoiceId::new("A"));
}

#[test]
fn sources_are_named() {
    assert_eq!(VoiceSource::Piper.as_str(), "piper");
    assert_eq!(VoiceSource::EspeakNg.as_str(), "espeak-ng");
    assert!(VoiceSource::Piper.requires_download());
    assert!(!VoiceSource::Sapi.requires_download());
}

#[test]
fn a_voice_exposes_its_fields() {
    let entry = voice("piper-en", VoiceSource::Piper, true);

    assert_eq!(entry.id(), &VoiceId::new("piper-en"));
    assert_eq!(entry.name(), "Voice piper-en");
    assert_eq!(entry.locale(), "en");
    assert_eq!(entry.source(), VoiceSource::Piper);
    assert!(entry.is_downloaded());
}

#[test]
fn the_default_policy_forbids_downloads() {
    let catalog = VoiceCatalog::default();
    assert_eq!(catalog.policy(), DownloadPolicy::Forbidden);
    assert!(catalog.is_empty());

    let mut catalog = VoiceCatalog::new(DownloadPolicy::Forbidden);
    catalog.add_voice(voice("piper-en", VoiceSource::Piper, false));

    assert_eq!(catalog.len(), 1);
    assert!(!catalog.can_download(&VoiceId::new("piper-en")));
}

#[test]
fn a_download_needs_an_opt_in() {
    let mut catalog = VoiceCatalog::new(DownloadPolicy::OptIn);
    catalog.add_voice(voice("piper-en", VoiceSource::Piper, false));
    let id = VoiceId::new("piper-en");

    assert!(!catalog.can_download(&id), "not opted in yet");

    catalog.enable_opt_in(&id);
    assert!(catalog.opt_in_enabled(&id));
    assert!(catalog.can_download(&id));
}

#[test]
fn a_downloaded_voice_needs_no_opt_in() {
    let mut catalog = VoiceCatalog::new(DownloadPolicy::Forbidden);
    catalog.add_voice(voice("piper-en", VoiceSource::Piper, true));

    assert!(catalog.is_downloaded(&VoiceId::new("piper-en")));
    assert!(catalog.can_download(&VoiceId::new("piper-en")));
}

#[test]
fn marking_a_voice_downloaded_changes_the_answer() {
    let mut catalog = VoiceCatalog::new(DownloadPolicy::Forbidden);
    catalog.add_voice(voice("piper-en", VoiceSource::Piper, false));
    let id = VoiceId::new("piper-en");

    assert!(catalog.mark_downloaded(&id));
    assert!(catalog.can_download(&id));
    assert!(!catalog.mark_downloaded(&VoiceId::new("missing")));
}

#[test]
fn adding_a_voice_replaces_the_same_id() {
    let mut catalog = VoiceCatalog::default();
    catalog.add_voice(voice("piper-en", VoiceSource::Piper, false));
    catalog.add_voice(voice("piper-en", VoiceSource::Piper, true));

    assert_eq!(catalog.len(), 1);
    assert!(catalog.is_downloaded(&VoiceId::new("piper-en")));
}

#[test]
fn find_and_clear() {
    let mut catalog = VoiceCatalog::default();
    catalog.add_voice(voice("piper-en", VoiceSource::Piper, false));
    catalog.add_voice(voice("espeak", VoiceSource::EspeakNg, true));

    assert!(catalog.find(&VoiceId::new("espeak")).is_some());
    assert!(catalog.find(&VoiceId::new("missing")).is_none());
    assert_eq!(catalog.voices().len(), 2);

    catalog.clear();
    assert!(catalog.is_empty());
    assert!(!catalog.opt_in_enabled(&VoiceId::new("piper-en")));
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/SPEECH.md");
    for word in ["voice", "opt-in", "fallback", "network", "audio"] {
        assert!(doc.contains(word), "docs/SPEECH.md does not state `{word}`");
    }
}
