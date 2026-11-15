//! Tests for the speech engine abstraction.

use std::sync::Arc;

use super::*;
use crate::voice::{VoiceId, VoiceSource};

fn voice() -> VoiceId {
    VoiceId::new("piper-en")
}

fn utterance(text: &str) -> Utterance {
    Utterance::new(text, voice())
}

/// A configurable engine for chain tests.
struct StubEngine {
    name: &'static str,
    available: bool,
    latency: u64,
    network: bool,
    outcome: Result<SpeechAudio, SpeechError>,
}

impl TtsEngine for StubEngine {
    fn name(&self) -> &'static str {
        self.name
    }

    fn kind(&self) -> EngineKind {
        EngineKind::Remote
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn start_latency_ms(&self) -> u64 {
        self.latency
    }

    fn uses_network(&self) -> bool {
        self.network
    }

    fn synthesize(&self, _utterance: &Utterance) -> Result<SpeechAudio, SpeechError> {
        self.outcome.clone()
    }
}

fn ok(hash: u64) -> Result<SpeechAudio, SpeechError> {
    Ok(SpeechAudio::new(22_050, 500, hash))
}

#[test]
fn an_utterance_holds_its_fields() {
    let speech = utterance("hello there")
        .with_id(UtteranceId(7))
        .with_rate(2.0)
        .with_priority(Priority::High);

    assert_eq!(speech.id().get(), 7);
    assert_eq!(speech.text(), "hello there");
    assert_eq!(speech.voice(), &voice());
    assert_eq!(speech.rate(), 2.0);
    assert_eq!(speech.priority(), Priority::High);
    assert_eq!(speech.priority().as_str(), "high");
    assert!(!speech.is_empty());
    assert_eq!(speech.len_bytes(), 11);
}

#[test]
fn the_rate_is_clamped() {
    assert_eq!(utterance("a").with_rate(0.1).rate(), 0.5);
    assert_eq!(utterance("a").with_rate(9.0).rate(), 2.0);
    assert_eq!(utterance("a").with_rate(1.25).rate(), 1.25);
}

#[test]
fn an_empty_utterance_is_recognised() {
    assert!(utterance("").is_empty());
    assert!(utterance("   \n ").is_empty());
    assert!(!utterance("a").is_empty());
}

#[test]
fn the_content_hash_is_stable() {
    assert_eq!(
        utterance("same").content_hash(),
        utterance("same").content_hash()
    );
    assert_ne!(
        utterance("same").content_hash(),
        utterance("different").content_hash()
    );
}

#[test]
fn an_utterance_is_redacted_in_logs() {
    let speech = utterance("alpha-bravo-charlie");
    let summary = speech.describe();

    assert_eq!(
        summary,
        format!(
            "utterance 0 ({} bytes, hash {})",
            speech.len_bytes(),
            speech.content_hash()
        )
    );
    assert!(!summary.contains("alpha-bravo-charlie"));

    let debug = format!("{speech:?}");
    assert!(!debug.contains("alpha-bravo-charlie"));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn engine_kinds_are_described() {
    assert_eq!(EngineKind::Piper.as_str(), "piper");
    assert!(EngineKind::Piper.is_local());
    assert!(!EngineKind::Remote.is_local());
    assert!(EngineKind::Piper.requires_voice_download());
    assert!(!EngineKind::Script.requires_voice_download());
}

#[test]
fn default_voice_sources_map_engines() {
    assert_eq!(
        default_voice_source(EngineKind::Piper),
        Some(VoiceSource::Piper)
    );
    assert_eq!(
        default_voice_source(EngineKind::Sapi),
        Some(VoiceSource::Sapi)
    );
    assert_eq!(default_voice_source(EngineKind::Script), None);
}

#[test]
fn the_budget_defaults_to_fifty_milliseconds() {
    assert_eq!(LatencyBudget::DEFAULT_MAX_START_MS, 50);
    assert_eq!(LatencyBudget::default().max_start_ms(), 50);
    assert!(LatencyBudget::default().allows(50));
    assert!(!LatencyBudget::default().allows(51));
    assert!(LatencyBudget::new(5).allows(5));
}

#[test]
fn the_script_engine_is_the_instant_local_fallback() {
    let engine = ScriptEngine::new();

    assert_eq!(engine.name(), "script");
    assert_eq!(engine.kind(), EngineKind::Script);
    assert!(engine.is_available());
    assert_eq!(engine.start_latency_ms(), 5);
    assert!(!engine.uses_network());
    assert!(LatencyBudget::default().allows(engine.start_latency_ms()));
}

#[test]
fn the_script_engine_synthesizes_a_handle() {
    let engine = ScriptEngine::new();
    let speech = utterance("one two three");

    let audio = engine.synthesize(&speech).expect("it synthesizes");
    assert_eq!(audio.sample_rate_hz(), 22_050);
    assert_eq!(audio.duration_ms(), 240, "120 ms plus 40 ms per word");
    assert_eq!(audio.content_hash(), speech.content_hash());

    assert_eq!(engine.synthesize(&utterance("")), Err(SpeechError::Empty));
}

#[test]
fn an_empty_chain_has_no_engine() {
    let chain = FallbackChain::new(Vec::new());
    assert_eq!(chain.engine_count(), 0);
    assert_eq!(
        chain.select(&utterance("hello")).err(),
        Some(SpeechError::Unavailable)
    );
    assert_eq!(
        chain.speak(&utterance("hello")),
        Err(SpeechError::Unavailable)
    );
}

#[test]
fn an_empty_utterance_is_rejected() {
    let chain = FallbackChain::new(vec![Arc::new(ScriptEngine)]);

    assert_eq!(chain.select(&utterance("")).err(), Some(SpeechError::Empty));
    assert_eq!(chain.speak(&utterance("")), Err(SpeechError::Empty));
}

#[test]
fn the_chain_selects_the_first_engine_that_fits() {
    let slow = Arc::new(StubEngine {
        name: "slow",
        available: true,
        latency: 200,
        network: false,
        outcome: ok(1),
    });
    let fast: Arc<dyn TtsEngine> = Arc::new(ScriptEngine);
    let chain = FallbackChain::new(vec![slow, Arc::clone(&fast)]);

    let selected = chain.select(&utterance("hello")).expect("an engine");
    assert_eq!(selected.name(), "script");
    assert_eq!(chain.engine_count(), 2, "counting happens before filtering");
}

#[test]
fn engines_over_the_budget_are_skipped() {
    let slow = Arc::new(StubEngine {
        name: "slow",
        available: true,
        latency: 90,
        network: false,
        outcome: ok(1),
    });
    let chain = FallbackChain::new(vec![slow]);

    assert_eq!(
        chain.select(&utterance("hello")).err(),
        Some(SpeechError::OverBudget)
    );
    assert_eq!(
        chain.speak(&utterance("hello")),
        Err(SpeechError::OverBudget)
    );
}

#[test]
fn network_engines_are_never_the_fallback() {
    let remote = Arc::new(StubEngine {
        name: "remote",
        available: true,
        latency: 1,
        network: true,
        outcome: ok(1),
    });
    let chain = FallbackChain::new(vec![remote]);

    assert_eq!(
        chain.select(&utterance("hello")).err(),
        Some(SpeechError::OverBudget)
    );
    assert_eq!(chain.candidates().count(), 0);
}

#[test]
fn unavailable_engines_are_skipped() {
    let down = Arc::new(StubEngine {
        name: "down",
        available: false,
        latency: 1,
        network: false,
        outcome: ok(1),
    });
    let chain = FallbackChain::new(vec![down]);

    assert_eq!(
        chain.select(&utterance("hello")).err(),
        Some(SpeechError::OverBudget)
    );
}

#[test]
fn the_chain_falls_through_a_failing_engine() {
    let failing = Arc::new(StubEngine {
        name: "failing",
        available: true,
        latency: 1,
        network: false,
        outcome: Err(SpeechError::Interrupted),
    });
    let working = Arc::new(StubEngine {
        name: "working",
        available: true,
        latency: 2,
        network: false,
        outcome: ok(42),
    });
    let chain = FallbackChain::new(vec![failing, working]);

    let audio = chain.speak(&utterance("hello")).expect("the second engine");
    assert_eq!(audio.content_hash(), 42);
}

#[test]
fn a_chain_where_every_engine_fails_reports_the_last_error() {
    let first = Arc::new(StubEngine {
        name: "first",
        available: true,
        latency: 1,
        network: false,
        outcome: Err(SpeechError::Interrupted),
    });
    let second = Arc::new(StubEngine {
        name: "second",
        available: true,
        latency: 2,
        network: false,
        outcome: Err(SpeechError::VoiceNotDownloaded(voice())),
    });
    let chain = FallbackChain::new(vec![first, second]);

    assert_eq!(
        chain.speak(&utterance("hello")),
        Err(SpeechError::VoiceNotDownloaded(voice()))
    );
}

#[test]
fn the_budget_can_be_tightened() {
    let engine = Arc::new(ScriptEngine);
    let mut chain = FallbackChain::new(vec![engine]);
    chain.set_budget(LatencyBudget::new(4));

    assert_eq!(
        chain.select(&utterance("hello")).err(),
        Some(SpeechError::OverBudget)
    );
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/SPEECH.md");
    for word in ["voice", "opt-in", "fallback", "network", "audio"] {
        assert!(doc.contains(word), "docs/SPEECH.md does not state `{word}`");
    }
}
