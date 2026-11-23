//! Deterministic failure injection.
//!
//! A chaos run drives a component through a scripted sequence of faults and
//! records what happened. Three invariants hold:
//!
//! - **No panic escapes.** Every injected fault runs inside [`guard`]. A caught
//!   panic is counted in the report, and the run stops rather than unwinding
//!   the caller.
//! - **A fault is announced.** After [`Faulty::inject`], the component must
//!   report a degradation. A fault with no announcement is counted as
//!   `unannounced`.
//! - **Recovery is automatic.** [`Faulty::recover`] must restore the baseline,
//!   and the report records whether it did.
//!
//! The harness is generic over the component, so each crate plugs in its own
//! real surface: permission revocation against a [`Session`], corruption
//! against a cache, a huge display against [`Rect`] geometry, a remote desktop
//! against a capture link. Every reason is a static string — never content.

use std::cell::Cell;
use std::fmt;
use std::panic::{catch_unwind, set_hook, take_hook, AssertUnwindSafe, PanicHookInfo};
use std::sync::{Mutex, Once};

use crate::capability::{Capability, CapabilityToken, Session};
use crate::id::SessionId;
use crate::prop::Rng;
use crate::types::Rect;

/// The hook that was installed before this module replaced it.
static PREVIOUS_HOOK: Mutex<Option<HookFn>> = Mutex::new(None);
/// Installs the silencing hook exactly once, blocking late callers until done.
///
/// A plain flag is not enough: `take_hook` resets the process hook to the
/// default one, so a second thread that saw the flag set would run its guarded
/// operation while the default hook was still live.
static INSTALL_HOOK: Once = Once::new();

type HookFn = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

thread_local! {
    /// How many guarded operations the current thread is inside.
    ///
    /// The panic hook runs on the panicking thread, so a thread-local depth is
    /// what makes suppression correct under concurrency: another thread's
    /// guarded panic cannot be un-silenced by this one finishing first.
    static SUPPRESSED_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// One injected failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fault {
    /// A granted capability is revoked mid-session.
    PermissionRevoked,
    /// A stored value no longer matches its checksum.
    CacheCorrupt,
    /// The display is far larger than a reference machine.
    HugeScreen,
    /// Capture arrives over a remote desktop session.
    RemoteDesktop,
}

impl Fault {
    /// Every fault the harness knows how to inject.
    pub const ALL: &'static [Fault] = &[
        Self::PermissionRevoked,
        Self::CacheCorrupt,
        Self::HugeScreen,
        Self::RemoteDesktop,
    ];

    /// The stable reason code. Content-free: it names a condition, never data.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PermissionRevoked => "permission-revoked",
            Self::CacheCorrupt => "cache-corrupt",
            Self::HugeScreen => "huge-screen",
            Self::RemoteDesktop => "remote-desktop",
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A fault scheduled at a step index.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Script {
    steps: Vec<(u64, Fault)>,
}

impl Script {
    /// Builds a script from `(step, fault)` pairs.
    ///
    /// The pairs are sorted by step, so a script built in any order reads the
    /// same way.
    pub fn new(steps: impl IntoIterator<Item = (u64, Fault)>) -> Self {
        let mut steps: Vec<(u64, Fault)> = steps.into_iter().collect();
        steps.sort_by_key(|(step, _)| *step);
        Self { steps }
    }

    /// How many faults the script schedules.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the script schedules nothing.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The scheduled `(step, fault)` pairs in step order.
    pub fn iter(&self) -> impl Iterator<Item = (u64, Fault)> + '_ {
        self.steps.iter().copied()
    }

    /// A random script: each step carries a fault with probability two in
    /// three, drawn from [`Fault::ALL`].
    ///
    /// A random script lets a property test explore orderings a hand-written
    /// list would miss. Case `n` derives from the seed, so a failing ordering
    /// replays exactly.
    pub fn random(rng: &mut Rng, steps: u64) -> Self {
        let mut scheduled = Vec::new();
        for step in 0..steps {
            if rng.below(3) == 0 {
                continue;
            }
            let fault = Fault::ALL[rng.below(Fault::ALL.len() as u64) as usize];
            scheduled.push((step, fault));
        }
        Self::new(scheduled)
    }
}

/// An announced degradation.
///
/// `reason` is documented as a static reason code, but the type system only
/// sees a `&'static str`, and a leaked `String` satisfies that bound. The
/// manual `Debug` below therefore prints the shape without the code, so a
/// report formatted into a log cannot carry content even if a caller misuses
/// the field.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Degradation {
    /// The step that produced it.
    pub step: u64,
    /// The fault that produced it.
    pub fault: Fault,
    /// A static reason code.
    pub reason: &'static str,
    /// Whether [`Faulty::recover`] restored the baseline.
    pub recovered: bool,
}

impl fmt::Debug for Degradation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Degradation")
            .field("step", &self.step)
            .field("fault", &self.fault)
            .field("reason", &"<static reason code omitted>")
            .field("recovered", &self.recovered)
            .finish()
    }
}

impl Degradation {
    /// An announcement for `fault` at `step`.
    ///
    /// `reason` must be a static reason code from this module's registry. It is
    /// never printed by `Debug`, so an accidental content string stays out of
    /// logs; treat the field as opaque.
    pub const fn new(step: u64, fault: Fault, reason: &'static str) -> Self {
        Self {
            step,
            fault,
            reason,
            recovered: false,
        }
    }
}

/// The result of one chaos run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChaosReport {
    steps: u64,
    faults: u64,
    unannounced: u64,
    unrecovered: u64,
    panics: u64,
    degradations: Vec<Degradation>,
}

impl ChaosReport {
    /// How many faults the script schedules, including ones this component
    /// does not accept.
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// How many faults were injected.
    pub fn faults(&self) -> u64 {
        self.faults
    }

    /// How many injected faults produced no announcement.
    pub fn unannounced(&self) -> u64 {
        self.unannounced
    }

    /// How many announcements were not followed by a successful recovery.
    pub fn unrecovered(&self) -> u64 {
        self.unrecovered
    }

    /// How many steps panicked. A caught panic never escapes the run.
    pub fn panics(&self) -> u64 {
        self.panics
    }

    /// Every announcement, in step order.
    pub fn degradations(&self) -> &[Degradation] {
        &self.degradations
    }

    /// Whether every injected fault was announced.
    pub fn announced_every_fault(&self) -> bool {
        self.unannounced == 0 && self.degradations.len() as u64 == self.faults
    }

    /// Whether every announcement ended in a successful recovery.
    pub fn all_recovered(&self) -> bool {
        self.unrecovered == 0
    }

    /// Whether no step panicked.
    pub fn no_panic_escaped(&self) -> bool {
        self.panics == 0
    }

    /// Whether the run met every invariant.
    pub fn is_clean(&self) -> bool {
        self.no_panic_escaped() && self.announced_every_fault() && self.all_recovered()
    }
}

impl fmt::Display for ChaosReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} steps, {} faults, {} panics, {} unannounced, {} unrecovered",
            self.steps, self.faults, self.panics, self.unannounced, self.unrecovered
        )
    }
}

/// A component a chaos script can fault.
pub trait Faulty {
    /// Whether this component models `fault`.
    ///
    /// A component that does not model a fault is skipped, so one script can
    /// drive surfaces with different responsibilities.
    fn accepts(&self, fault: Fault) -> bool;

    /// Applies `fault`. The component must accept it.
    fn inject(&mut self, fault: Fault);

    /// The announced degradation, or `None` when the component is healthy.
    fn degradation(&self) -> Option<Degradation>;

    /// Restores the baseline. Returns whether recovery succeeded.
    fn recover(&mut self) -> bool;
}

/// Runs `script` against `component`.
///
/// Each scheduled fault is checked and injected inside [`guard`]. A panic ends
/// the run and is counted, never propagated. Every other fault is followed by
/// [`Faulty::recover`] whether or not it was announced, so a quiet fault still
/// leaves the component at its baseline and the report shows both facts.
pub fn run<C: Faulty>(component: &mut C, script: &Script) -> ChaosReport {
    let mut report = ChaosReport {
        steps: script.len() as u64,
        ..ChaosReport::default()
    };

    for (step, fault) in script.iter() {
        // `accepts` is part of the component's behaviour, so it runs under the
        // same containment as `inject`: an acceptance check that panics must be
        // caught and counted rather than escaping into the caller's log.
        let accepted = match guard(|| component.accepts(fault)) {
            Ok(accepted) => accepted,
            Err(_caught) => {
                report.panics += 1;
                break;
            }
        };
        if !accepted {
            continue;
        }
        let outcome = guard(|| {
            component.inject(fault);
            let degradation = component.degradation();
            let recovered = component.recover();
            (degradation, recovered)
        });

        match outcome {
            Ok((Some(degradation), recovered)) => {
                report.faults += 1;
                if !recovered {
                    report.unrecovered += 1;
                }
                report.degradations.push(Degradation {
                    step,
                    recovered,
                    ..degradation
                });
            }
            Ok((None, recovered)) => {
                report.faults += 1;
                report.unannounced += 1;
                if !recovered {
                    report.unrecovered += 1;
                }
            }
            Err(_caught) => {
                report.panics += 1;
                break;
            }
        }
    }

    report
}

/// A caught panic.
///
/// The payload is dropped unread before this value exists: a panic message can
/// quote the input that caused it, and a panic message reaches the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caught;

impl fmt::Display for Caught {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a panic was caught")
    }
}

impl std::error::Error for Caught {}

/// Installs the silencing hook once, delegating to whatever was there before.
///
/// The default panic hook prints the payload to stderr at panic time, before
/// `catch_unwind` can inspect it, and a payload can quote its input. Swapping
/// hooks around each call is both racy — two threads interleave their
/// save/set/restore — and leaves the default hook live in the gap between
/// `take_hook` and `set_hook`. Instead one hook is installed for the process
/// and decides per panic whether the panicking thread asked for silence.
fn install_silencing_hook() {
    INSTALL_HOOK.call_once(|| {
        let previous = take_hook();
        // A poisoned lock means another thread panicked while taking it; keeping
        // the default behaviour (printing) is the safe failure mode, because
        // silently dropping every hook would hide unrelated crashes.
        if let Ok(mut slot) = PREVIOUS_HOOK.lock() {
            *slot = Some(previous);
        }
        set_hook(Box::new(|info| {
            if SUPPRESSED_DEPTH.with(Cell::get) > 0 {
                return;
            }
            if let Ok(slot) = PREVIOUS_HOOK.lock() {
                if let Some(previous) = slot.as_ref() {
                    previous(info);
                }
            }
        }));
    });
}

/// Runs `operation`, turning a panic into `Err(Caught)`.
///
/// A panic message can quote the input that caused it, and the panic hook
/// writes it to stderr before [`catch_unwind`] runs. The hook installed once at
/// first use withholds the message on any thread currently inside `guard`, and
/// leaves every other panic untouched.
///
/// Panics on a thread this call did not start are not suppressed: the depth is
/// thread-local, so containing one thread never hides a crash elsewhere.
pub fn guard<F, R>(operation: F) -> Result<R, Caught>
where
    F: FnOnce() -> R,
{
    install_silencing_hook();
    SUPPRESSED_DEPTH.with(|depth| depth.set(depth.get() + 1));
    let outcome = catch_unwind(AssertUnwindSafe(operation));
    SUPPRESSED_DEPTH.with(|depth| depth.set(depth.get() - 1));
    outcome.map_err(|_| Caught)
}

/// A session surface: one capability granted, then revoked mid-session.
///
/// [`Session::revoke`] invalidates every token for the capability, which is
/// exactly the failure a user hits when an OS permission prompt is dismissed
/// while the app is running.
#[derive(Debug)]
pub struct PermissionGate {
    session: Session,
    capability: Capability,
    token: Option<CapabilityToken>,
}

impl PermissionGate {
    /// Grants `capability` for a fresh session and mints its token.
    pub fn new(capability: Capability) -> Self {
        let mut session = Session::new(SessionId(1));
        let token = session.grant(capability);
        Self {
            session,
            capability,
            token: Some(token),
        }
    }

    /// Whether the current token still proves the capability.
    pub fn is_usable(&self) -> bool {
        self.token.as_ref().is_some_and(|token| token.is_valid())
    }

    /// The granted capability.
    pub const fn capability(&self) -> Capability {
        self.capability
    }
}

impl Faulty for PermissionGate {
    fn accepts(&self, fault: Fault) -> bool {
        fault == Fault::PermissionRevoked
    }

    fn inject(&mut self, fault: Fault) {
        debug_assert_eq!(fault, Fault::PermissionRevoked);
        self.session.revoke(self.capability);
    }

    fn degradation(&self) -> Option<Degradation> {
        if self.is_usable() {
            None
        } else {
            Some(Degradation::new(
                0,
                Fault::PermissionRevoked,
                "observe-permission-revoked",
            ))
        }
    }

    fn recover(&mut self) -> bool {
        let token = self.session.grant(self.capability);
        let usable = token.is_valid() && token.capability() == self.capability;
        self.token = Some(token);
        usable
    }
}

/// A deliberately corruptible byte cache.
///
/// Each entry carries a one-byte XOR checksum. [`Self::corrupt`] breaks the
/// agreement, which models a torn write or a stale buffer; a read then returns
/// `None` rather than a value that silently disagrees with its checksum. This
/// is a test surface for injection, not production storage.
///
/// The `Debug` form prints keys and counts only: the byte vectors are payload
/// and would reach a log through `{:?}`.
#[derive(Clone, PartialEq, Eq)]
pub struct ChecksumCache {
    entries: Vec<(u64, Vec<u8>, u8)>,
}

impl fmt::Debug for ChecksumCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys: Vec<u64> = self.entries.iter().map(|(key, _, _)| *key).collect();
        f.debug_struct("ChecksumCache")
            .field("entries", &self.entries.len())
            .field("keys", &keys)
            .finish()
    }
}

impl Default for ChecksumCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ChecksumCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Stores `bytes` under `key`, replacing any prior entry.
    pub fn put(&mut self, key: u64, bytes: &[u8]) {
        let checksum = checksum(bytes);
        match self.entries.iter_mut().find(|(known, _, _)| *known == key) {
            Some(entry) => {
                entry.1 = bytes.to_vec();
                entry.2 = checksum;
            }
            None => self.entries.push((key, bytes.to_vec(), checksum)),
        }
    }

    /// Reads `key`. `None` when the entry is absent or its checksum disagrees.
    pub fn get(&self, key: u64) -> Option<&[u8]> {
        let (_, bytes, stored) = self.entries.iter().find(|(known, _, _)| *known == key)?;
        if checksum(bytes) == *stored {
            Some(bytes)
        } else {
            None
        }
    }

    /// Breaks the checksum agreement for `key`. `false` when the key is absent.
    ///
    /// An empty payload has nothing to flip, so its stored checksum is moved
    /// instead; either way the next read disagrees.
    pub fn corrupt(&mut self, key: u64) -> bool {
        let Some((_, bytes, stored)) = self.entries.iter_mut().find(|(known, _, _)| *known == key)
        else {
            return false;
        };
        if bytes.is_empty() {
            *stored = stored.wrapping_add(1);
        } else {
            bytes[0] ^= 0x01;
        }
        true
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Drops the entries whose checksum no longer matches, keeping the rest.
    pub fn retain_valid(&mut self) {
        self.entries
            .retain(|(_, bytes, stored)| checksum(bytes) == *stored);
    }

    /// How many entries the cache holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache holds nothing.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A one-byte XOR checksum. Not cryptographic; it catches a torn write.
fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0_u8, |acc, byte| acc ^ *byte)
}

impl Faulty for ChecksumCache {
    fn accepts(&self, fault: Fault) -> bool {
        fault == Fault::CacheCorrupt
    }

    fn inject(&mut self, fault: Fault) {
        debug_assert_eq!(fault, Fault::CacheCorrupt);
        // Key 1 is the cache's baseline entry; a test seeds it. When the entry
        // is already gone, a fresh torn write leaves an unreadable entry, so
        // the fault still produces an announced degradation.
        if !self.corrupt(1) {
            // An empty payload with a checksum of 1 cannot be read back.
            self.entries.push((1, Vec::new(), 1));
        }
    }

    fn degradation(&self) -> Option<Degradation> {
        if self.entries.is_empty() || self.get(1).is_some() {
            None
        } else {
            Some(Degradation::new(
                0,
                Fault::CacheCorrupt,
                "plan-cache-unreadable",
            ))
        }
    }

    fn recover(&mut self) -> bool {
        self.retain_valid();
        self.get(1).is_none()
    }
}

/// How a frame reaches us.
///
/// `Unavailable` carries a caller-supplied reason code. Like
/// [`Degradation::reason`], it is printed by hand so `{:?}` cannot become a
/// content leak.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureLink {
    /// A local display.
    #[default]
    Local,
    /// A remote desktop session. Capture may be blocked or heavily compressed.
    RemoteDesktop,
    /// Capture is unavailable. The reason is a static code.
    Unavailable(&'static str),
}

impl fmt::Debug for CaptureLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => f.write_str("Local"),
            Self::RemoteDesktop => f.write_str("RemoteDesktop"),
            Self::Unavailable(_) => f.write_str("Unavailable(<reason code omitted>)"),
        }
    }
}

impl CaptureLink {
    /// Whether frames can be read.
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Local | Self::RemoteDesktop)
    }

    /// The announced degradation, or `None` on a local link.
    pub fn announcement(&self) -> Option<Degradation> {
        match self {
            Self::Local => None,
            Self::RemoteDesktop => Some(Degradation::new(
                0,
                Fault::RemoteDesktop,
                "capture-link-remote",
            )),
            Self::Unavailable(reason) => Some(Degradation::new(0, Fault::RemoteDesktop, reason)),
        }
    }
}

impl Faulty for CaptureLink {
    fn accepts(&self, fault: Fault) -> bool {
        fault == Fault::RemoteDesktop
    }

    fn inject(&mut self, fault: Fault) {
        debug_assert_eq!(fault, Fault::RemoteDesktop);
        *self = Self::Unavailable("capture-link-remote");
    }

    fn degradation(&self) -> Option<Degradation> {
        self.announcement()
    }

    fn recover(&mut self) -> bool {
        *self = Self::Local;
        self.is_usable()
    }
}

/// A display budget: geometry that must stay representable at any size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenBudget {
    rect: Rect,
    baseline: Rect,
}

impl Default for ScreenBudget {
    fn default() -> Self {
        Self::new(Rect::new(0, 0, 1_920, 1_080))
    }
}

impl ScreenBudget {
    /// A budget for `rect`.
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            baseline: rect,
        }
    }

    /// The current rectangle.
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// Total pixels, saturating rather than overflowing.
    pub fn total_pixels(&self) -> u64 {
        self.rect.area()
    }

    /// A 5K-class display.
    pub fn huge() -> Self {
        Self::new(Rect::new(0, 0, 5_120, 2_880))
    }
}

impl Faulty for ScreenBudget {
    fn accepts(&self, fault: Fault) -> bool {
        fault == Fault::HugeScreen
    }

    fn inject(&mut self, fault: Fault) {
        debug_assert_eq!(fault, Fault::HugeScreen);
        // Extreme coordinates exercise the saturated edge arithmetic.
        self.rect = Rect::new(i32::MAX - 1_000, i32::MIN + 1_000, 8_192, 8_192);
    }

    fn degradation(&self) -> Option<Degradation> {
        if self.rect == self.baseline {
            None
        } else {
            Some(Degradation::new(
                0,
                Fault::HugeScreen,
                "display-beyond-reference",
            ))
        }
    }

    fn recover(&mut self) -> bool {
        self.rect = self.baseline;
        self.rect == self.baseline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_sorts_by_step() {
        let script = Script::new([(9, Fault::HugeScreen), (2, Fault::CacheCorrupt)]);
        assert_eq!(
            script.iter().collect::<Vec<_>>(),
            vec![(2, Fault::CacheCorrupt), (9, Fault::HugeScreen)]
        );
        assert_eq!(script.len(), 2);
        assert!(!script.is_empty());
        let empty = Script::default();
        assert!(empty.is_empty());
    }

    #[test]
    fn a_random_script_is_deterministic() {
        let mut left = Rng::new(7);
        let mut right = Rng::new(7);
        assert_eq!(
            Script::random(&mut left, 40),
            Script::random(&mut right, 40)
        );
    }

    #[test]
    fn guard_catches_a_panic_without_reading_the_payload() {
        let result = guard(|| panic!("boom"));
        assert!(result.is_err());
        let value = guard(|| 7_u32);
        assert_eq!(value, Ok(7));
    }

    #[test]
    fn a_revoked_permission_is_announced_and_recovered() {
        let mut gate = PermissionGate::new(Capability::Observe);
        assert!(gate.is_usable());
        let report = run(
            &mut gate,
            &Script::new([
                (0, Fault::PermissionRevoked),
                (1, Fault::PermissionRevoked),
                (2, Fault::CacheCorrupt),
            ]),
        );
        assert!(report.no_panic_escaped(), "{report}");
        assert!(report.announced_every_fault(), "{report}");
        assert!(report.all_recovered(), "{report}");
        assert_eq!(report.faults(), 2, "the cache fault is not accepted here");
        assert!(gate.is_usable());
    }

    #[test]
    fn an_empty_entry_can_still_be_corrupted() {
        let mut cache = ChecksumCache::new();
        cache.put(1, b"");
        assert_eq!(cache.get(1), Some(&b""[..]));
        assert!(cache.corrupt(1));
        assert_eq!(cache.get(1), None, "the empty entry must read as absent");
        let report = run(&mut cache, &Script::new([(0, Fault::CacheCorrupt)]));
        assert!(report.is_clean(), "{report}");
    }

    #[test]
    fn debug_prints_keys_and_counts_but_never_bytes() {
        let mut cache = ChecksumCache::new();
        cache.put(7, b"payload-bytes");
        cache.put(9, b"more");
        let text = format!("{cache:?}");
        assert!(text.contains("ChecksumCache"), "{text}");
        assert!(text.contains("7") && text.contains("9"), "{text}");
        assert!(!text.contains("payload-bytes"), "{text}");
        assert!(!text.contains("more"), "{text}");
    }

    #[test]
    fn a_corrupted_entry_reads_as_absent_then_recovers() {
        let mut cache = ChecksumCache::new();
        cache.put(1, b"baseline");
        assert_eq!(cache.get(1), Some(&b"baseline"[..]));
        let report = run(
            &mut cache,
            &Script::new([(0, Fault::CacheCorrupt), (1, Fault::CacheCorrupt)]),
        );
        assert!(report.is_clean(), "{report}");
        assert!(cache.is_empty(), "recovery cleared the cache");
    }

    #[test]
    fn a_huge_display_stays_representable() {
        let mut screen = ScreenBudget::default();
        let report = run(&mut screen, &Script::new([(0, Fault::HugeScreen)]));
        assert!(report.is_clean(), "{report}");
        let huge = ScreenBudget::huge();
        assert_eq!(huge.total_pixels(), 5_120 * 2_880);
        assert_eq!(huge.rect().right(), 5_120);
        assert_eq!(huge.rect().bottom(), 2_880);
    }

    #[test]
    fn a_remote_link_is_unusable_until_it_recovers() {
        let mut link = CaptureLink::Local;
        assert!(link.is_usable());
        let report = run(&mut link, &Script::new([(0, Fault::RemoteDesktop)]));
        assert!(report.is_clean(), "{report}");
        assert_eq!(link, CaptureLink::Local);
    }

    #[test]
    fn an_unacceptable_fault_is_skipped_rather_than_miscounted() {
        let mut link = CaptureLink::Local;
        let report = run(
            &mut link,
            &Script::new([(0, Fault::CacheCorrupt), (1, Fault::HugeScreen)]),
        );
        assert_eq!(report.faults(), 0);
        assert_eq!(report.steps(), 2);
        assert!(report.is_clean(), "{report}");
    }

    #[test]
    fn a_fault_that_is_not_announced_fails_the_report() {
        struct Quiet;
        impl Faulty for Quiet {
            fn accepts(&self, fault: Fault) -> bool {
                fault == Fault::HugeScreen
            }
            fn inject(&mut self, _fault: Fault) {}
            fn degradation(&self) -> Option<Degradation> {
                None
            }
            fn recover(&mut self) -> bool {
                true
            }
        }
        let report = run(&mut Quiet, &Script::new([(0, Fault::HugeScreen)]));
        assert_eq!(report.faults(), 1);
        assert_eq!(report.unannounced(), 1);
        assert!(!report.announced_every_fault());
    }

    #[test]
    fn a_quiet_fault_is_still_recovered() {
        struct Quiet {
            recoveries: u32,
        }
        impl Faulty for Quiet {
            fn accepts(&self, _fault: Fault) -> bool {
                true
            }
            fn inject(&mut self, _fault: Fault) {}
            fn degradation(&self) -> Option<Degradation> {
                None
            }
            fn recover(&mut self) -> bool {
                self.recoveries += 1;
                true
            }
        }
        let mut quiet = Quiet { recoveries: 0 };
        let report = run(&mut quiet, &Script::new([(0, Fault::HugeScreen)]));
        assert_eq!(
            quiet.recoveries, 1,
            "recovery must run even when the fault was never announced"
        );
        assert_eq!(report.unannounced(), 1);
        assert!(report.all_recovered());
        assert!(
            !report.is_clean(),
            "a missing announcement still fails the run"
        );
    }

    #[test]
    fn a_quiet_fault_that_cannot_recover_is_counted() {
        struct StuckQuiet;
        impl Faulty for StuckQuiet {
            fn accepts(&self, _fault: Fault) -> bool {
                true
            }
            fn inject(&mut self, _fault: Fault) {}
            fn degradation(&self) -> Option<Degradation> {
                None
            }
            fn recover(&mut self) -> bool {
                false
            }
        }
        let report = run(&mut StuckQuiet, &Script::new([(0, Fault::HugeScreen)]));
        assert_eq!(report.unrecovered(), 1);
        assert_eq!(report.unannounced(), 1);
    }

    #[test]
    fn a_panicking_acceptance_check_is_caught_and_counted() {
        struct Suspicious;
        impl Faulty for Suspicious {
            fn accepts(&self, _fault: Fault) -> bool {
                panic!("acceptance check exploded")
            }
            fn inject(&mut self, _fault: Fault) {}
            fn degradation(&self) -> Option<Degradation> {
                None
            }
            fn recover(&mut self) -> bool {
                true
            }
        }
        let report = run(
            &mut Suspicious,
            &Script::new([(0, Fault::CacheCorrupt), (1, Fault::PermissionRevoked)]),
        );
        assert_eq!(report.panics(), 1, "{report}");
        assert_eq!(report.faults(), 0, "the run stops at the panicking step");
        assert!(!report.no_panic_escaped());
    }

    #[test]
    fn debug_omits_reason_codes_even_when_a_caller_leaks_content() {
        let leaked: &'static str = Box::leak("user-screen-text".to_string().into_boxed_str());
        let degradation = Degradation::new(3, Fault::CacheCorrupt, leaked);
        let text = format!("{degradation:?}");
        assert!(!text.contains("user-screen-text"), "{text}");
        assert!(text.contains("Degradation"), "{text}");

        let link = CaptureLink::Unavailable(leaked);
        let text = format!("{link:?}");
        assert!(!text.contains("user-screen-text"), "{text}");

        let mut cache = ChecksumCache::new();
        cache.put(1, b"x");
        let report = run(&mut cache, &Script::new([(0, Fault::CacheCorrupt)]));
        let text = format!("{report:?}");
        assert!(!text.contains("user-screen-text"), "{text}");
        assert!(text.contains("ChaosReport"), "{text}");
    }

    #[test]
    fn a_failed_recovery_is_counted() {
        struct Stuck;
        impl Faulty for Stuck {
            fn accepts(&self, fault: Fault) -> bool {
                fault == Fault::CacheCorrupt
            }
            fn inject(&mut self, _fault: Fault) {}
            fn degradation(&self) -> Option<Degradation> {
                Some(Degradation::new(0, Fault::CacheCorrupt, "stuck"))
            }
            fn recover(&mut self) -> bool {
                false
            }
        }
        let report = run(&mut Stuck, &Script::new([(0, Fault::CacheCorrupt)]));
        assert_eq!(report.unrecovered(), 1);
        assert!(!report.all_recovered());
    }
}
