//! Switch access and dwell clicking for the overlay.
//!
//! [`SwitchAccess`] drives the overlay from one switch. Two modes cover the two
//! ways a single button is used:
//!
//! - **Auto scan.** The highlight advances by itself every
//!   [`ScanSequence::interval_ms`], and a press selects the highlighted control.
//!   A tick longer than the interval advances several steps and carries the
//!   remainder.
//! - **Step scan.** A press starts a dwell on the highlighted control: release
//!   before the threshold to step to the next one, hold to select it. A tap
//!   steps, a hold selects.
//!
//! [`ScanSequence`] holds the controls in logical order and skips disabled ones.
//! [`DwellTimer`] measures a dwell against a configurable threshold.
//!
//! The controller reads ids and timing only; it never sees screen content. See
//! [`../../../docs/SWITCH_ACCESS.md`](../../../docs/SWITCH_ACCESS.md).

use std::fmt;

/// Identifies one switch-scannable overlay control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScanTargetId(pub u64);

impl ScanTargetId {
    /// The raw numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ScanTargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScanTargetId({})", self.0)
    }
}

/// One switch-scannable overlay control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanItem {
    id: ScanTargetId,
    enabled: bool,
}

impl ScanItem {
    /// Creates an enabled control.
    pub const fn new(id: ScanTargetId) -> Self {
        Self { id, enabled: true }
    }

    /// Sets whether the control takes part in scanning.
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The control's id.
    pub const fn id(&self) -> ScanTargetId {
        self.id
    }

    /// Whether the control takes part in scanning.
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Why a scan sequence is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    /// No control can be scanned.
    NoEnabled,
    /// Two controls share an id.
    DuplicateId,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEnabled => write!(f, "no control can be scanned"),
            Self::DuplicateId => write!(f, "two controls share an id"),
        }
    }
}

impl std::error::Error for ScanError {}

/// The controls to scan, in logical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanSequence {
    items: Vec<ScanItem>,
    interval_ms: u64,
    current: Option<usize>,
}

impl ScanSequence {
    /// Creates a sequence that highlights the first enabled control.
    ///
    /// The interval is at least 1 ms.
    pub fn new(items: Vec<ScanItem>, interval_ms: u64) -> Self {
        let current = items.iter().position(|item| item.enabled);
        Self {
            items,
            interval_ms: interval_ms.max(1),
            current,
        }
    }

    /// The controls in scan order.
    pub fn items(&self) -> &[ScanItem] {
        &self.items
    }

    /// How many controls the sequence holds.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the sequence holds no controls.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many controls can be scanned.
    pub fn enabled_count(&self) -> usize {
        self.items.iter().filter(|item| item.enabled).count()
    }

    /// The auto-scan interval in milliseconds.
    pub const fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// The highlighted control.
    pub fn current(&self) -> Option<&ScanItem> {
        self.current.and_then(|index| self.items.get(index))
    }

    /// The highlighted control's id.
    pub fn current_id(&self) -> Option<ScanTargetId> {
        self.current().map(ScanItem::id)
    }

    /// Highlights a control by id.
    ///
    /// A disabled control is not highlighted.
    pub fn focus(&mut self, id: ScanTargetId) -> bool {
        match self.items.iter().position(|item| item.id == id) {
            Some(index) if self.items[index].enabled => {
                self.current = Some(index);
                true
            }
            _ => false,
        }
    }

    /// Advances to the next enabled control, wrapping.
    pub fn advance(&mut self) -> Option<ScanTargetId> {
        let current = self.current?;
        let start = current;
        let mut index = (current + 1) % self.items.len();
        while index != start {
            if self.items[index].enabled {
                self.current = Some(index);
                return self.current_id();
            }
            index = (index + 1) % self.items.len();
        }
        self.current_id()
    }

    /// Moves to the previous enabled control, wrapping.
    pub fn back(&mut self) -> Option<ScanTargetId> {
        let current = self.current?;
        let start = current;
        let mut index = (current + self.items.len() - 1) % self.items.len();
        while index != start {
            if self.items[index].enabled {
                self.current = Some(index);
                return self.current_id();
            }
            index = (index + self.items.len() - 1) % self.items.len();
        }
        self.current_id()
    }

    /// Highlights the first enabled control.
    pub fn first(&mut self) -> Option<ScanTargetId> {
        let index = self.items.iter().position(|item| item.enabled)?;
        self.current = Some(index);
        self.current_id()
    }

    /// Highlights the last enabled control.
    pub fn last(&mut self) -> Option<ScanTargetId> {
        let index = self.items.iter().rposition(|item| item.enabled)?;
        self.current = Some(index);
        self.current_id()
    }

    /// Checks the sequence.
    pub fn validate(&self) -> Result<(), ScanError> {
        if self.enabled_count() == 0 {
            return Err(ScanError::NoEnabled);
        }
        for i in 0..self.items.len() {
            for j in (i + 1)..self.items.len() {
                if self.items[i].id == self.items[j].id {
                    return Err(ScanError::DuplicateId);
                }
            }
        }
        Ok(())
    }
}

/// Measures a dwell against a configurable threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DwellTimer {
    threshold_ms: u64,
    elapsed_ms: u64,
}

impl DwellTimer {
    /// The default dwell, 1000 ms.
    pub const DEFAULT_THRESHOLD_MS: u64 = 1000;

    /// Creates a timer with a threshold of at least 1 ms.
    pub fn new(threshold_ms: u64) -> Self {
        Self {
            threshold_ms: threshold_ms.max(1),
            elapsed_ms: 0,
        }
    }

    /// The threshold in milliseconds.
    pub const fn threshold_ms(&self) -> u64 {
        self.threshold_ms
    }

    /// The elapsed milliseconds, capped at the threshold.
    pub const fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }

    /// Advances the dwell.
    ///
    /// Returns `true` once the threshold is reached; the timer stays complete
    /// until [`DwellTimer::reset`].
    pub fn tick(&mut self, delta_ms: u64) -> bool {
        self.elapsed_ms = self
            .elapsed_ms
            .saturating_add(delta_ms)
            .min(self.threshold_ms);
        self.is_complete()
    }

    /// The dwell progress from `0.0` to `1.0`.
    pub fn progress(&self) -> f32 {
        self.elapsed_ms as f32 / self.threshold_ms as f32
    }

    /// Whether the threshold has been reached.
    pub const fn is_complete(&self) -> bool {
        self.elapsed_ms >= self.threshold_ms
    }

    /// Clears the elapsed time.
    pub fn reset(&mut self) {
        self.elapsed_ms = 0;
    }
}

/// How one switch drives the scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwitchMode {
    /// The highlight advances on its own; a press selects.
    AutoScan,
    /// A press starts a dwell; release to step, hold to select.
    StepScan,
}

impl SwitchMode {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AutoScan => "auto-scan",
            Self::StepScan => "step-scan",
        }
    }
}

/// What happened on a switch press or tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchEvent {
    /// Nothing changed.
    None,
    /// The highlight moved.
    ///
    /// A step that leaves the highlight on the same control reports
    /// [`SwitchEvent::None`] instead.
    ScanChanged(ScanTargetId),
    /// The highlighted control was selected.
    Selected(ScanTargetId),
}

/// Drives the overlay from one switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchAccess {
    sequence: ScanSequence,
    dwell: DwellTimer,
    mode: SwitchMode,
    scan_elapsed_ms: u64,
    held: bool,
}

impl SwitchAccess {
    /// Creates a controller with the default dwell threshold.
    pub fn new(sequence: ScanSequence, mode: SwitchMode) -> Self {
        Self {
            sequence,
            dwell: DwellTimer::new(DwellTimer::DEFAULT_THRESHOLD_MS),
            mode,
            scan_elapsed_ms: 0,
            held: false,
        }
    }

    /// Sets the dwell threshold in milliseconds.
    pub fn set_dwell_ms(&mut self, dwell_ms: u64) {
        self.dwell = DwellTimer::new(dwell_ms);
        self.held = false;
    }

    /// The dwell threshold in milliseconds.
    pub const fn dwell_ms(&self) -> u64 {
        self.dwell.threshold_ms()
    }

    /// The mode.
    pub const fn mode(&self) -> SwitchMode {
        self.mode
    }

    /// Switches mode, clearing any pending dwell.
    pub fn set_mode(&mut self, mode: SwitchMode) {
        self.mode = mode;
        self.dwell.reset();
        self.scan_elapsed_ms = 0;
        self.held = false;
    }

    /// The scan sequence.
    pub fn sequence(&self) -> &ScanSequence {
        &self.sequence
    }

    /// The highlighted control's id.
    pub fn current_id(&self) -> Option<ScanTargetId> {
        self.sequence.current_id()
    }

    /// The dwell progress from `0.0` to `1.0`.
    pub fn dwell_progress(&self) -> f32 {
        self.dwell.progress()
    }

    /// Whether the switch is held down.
    pub const fn is_held(&self) -> bool {
        self.held
    }

    /// A switch press.
    ///
    /// In [`SwitchMode::AutoScan`] the highlighted control is selected at once.
    /// In [`SwitchMode::StepScan`] the press starts a dwell.
    pub fn press(&mut self) -> SwitchEvent {
        match self.mode {
            SwitchMode::AutoScan => self.select_current(),
            SwitchMode::StepScan => {
                self.held = true;
                self.dwell.reset();
                SwitchEvent::None
            }
        }
    }

    /// A switch release.
    ///
    /// In step mode, a press that did not reach the dwell threshold advances the
    /// highlight. In auto-scan mode nothing changes. A step that leaves the
    /// highlight where it was reports [`SwitchEvent::None`].
    pub fn release(&mut self) -> SwitchEvent {
        if self.mode != SwitchMode::StepScan || !self.held {
            return SwitchEvent::None;
        }
        self.held = false;
        self.dwell.reset();
        self.step()
    }

    /// Advances time by `delta_ms`.
    ///
    /// In auto scan the highlight advances once per interval and carries any
    /// remainder, so a delta longer than the interval advances several steps.
    /// In step scan a held switch selects the highlighted control once the
    /// dwell threshold passes.
    pub fn tick(&mut self, delta_ms: u64) -> SwitchEvent {
        match self.mode {
            SwitchMode::AutoScan => {
                self.scan_elapsed_ms = self.scan_elapsed_ms.saturating_add(delta_ms);
                let interval = self.sequence.interval_ms();
                let mut moved = None;
                while self.scan_elapsed_ms >= interval {
                    self.scan_elapsed_ms -= interval;
                    if let SwitchEvent::ScanChanged(id) = self.step() {
                        moved = Some(id);
                    }
                }
                match moved {
                    Some(id) => SwitchEvent::ScanChanged(id),
                    None => SwitchEvent::None,
                }
            }
            SwitchMode::StepScan => {
                if !self.held {
                    return SwitchEvent::None;
                }
                if self.dwell.tick(delta_ms) {
                    self.held = false;
                    let selected = self.sequence.current_id();
                    self.dwell.reset();
                    match selected {
                        Some(id) => SwitchEvent::Selected(id),
                        None => SwitchEvent::None,
                    }
                } else {
                    SwitchEvent::None
                }
            }
        }
    }

    /// Selects the highlighted control.
    fn select_current(&self) -> SwitchEvent {
        match self.sequence.current_id() {
            Some(id) => SwitchEvent::Selected(id),
            None => SwitchEvent::None,
        }
    }

    /// Advances the highlight once, reporting only a real move.
    ///
    /// A sequence with a single enabled control cannot move, so it reports
    /// [`SwitchEvent::None`] rather than a change to the same control.
    fn step(&mut self) -> SwitchEvent {
        let before = self.sequence.current_id();
        match self.sequence.advance() {
            Some(id) if Some(id) != before => SwitchEvent::ScanChanged(id),
            _ => SwitchEvent::None,
        }
    }
}

#[cfg(test)]
mod tests;
