# Accessibility audit protocol

> The manual half of the accessibility mapping. Each section below is one WCAG
> 2.2 criterion that no automated test covers. A human executes the steps on a
> real build and records the result in the pull request that changes the row.
>
> **This is a protocol, not a result.** No row is `Supports` until its section
> has been executed and the result recorded. `CC-097`.

## How to run

1. Build and start the app on the platform under test.
2. For each section, perform the steps and record `pass`, `fail`, or `blocked`.
3. Record the build hash, the platform, and the date with the result.
4. A `fail` lowers the row's status or rewrites the behavior text. It never
   raises a row to make the audit pass.
5. File a defect for a `fail`. Do not edit the row to match a broken build.

Record the results in a table with one row per section:

| Criterion | Result | Tester | Build | Notes |
| --- | --- | --- | --- | --- |

## Scope

These procedures evaluate **CurbCut's own UI**: the overlay, the settings
windows, and the tray. They do not evaluate a third-party application, and a
pass here is never a conformance claim about one. See
[`VPAT.md`](VPAT.md) for the framing.

## Platforms

Run the full protocol on macOS, Windows, and Linux. A criterion that passes on
one platform and fails on another is recorded as `fail` until it passes on all
three; the row then says which platforms are affected.

---

## Manual procedures

### 1.3.3 Sensory Characteristics

The overlay must not be understandable by shape, size, or color alone.

1. Open a screen with at least three differently shaped icons.
2. Hide every text label (the app's own labels and ours).
3. Ask a tester who has not seen the screen to name what each control does.
4. Pass when the labels are necessary: a control whose only cue is its shape is
   a fail.

### 1.4.1 Use of Color

No information may be carried by color alone.

1. Open the settings window and switch to a high-contrast or greyscale display
   filter.
2. Walk every state: an active tile, a disabled tile, an error, a selected
   profile.
3. Pass when each state is still distinguishable without hue.

### 1.4.4 Resize Text

Text must scale to 200% without loss of content or function.

1. Set the system text size to its largest value.
2. Open every settings window and the overlay.
3. Zoom the underlying application.
4. Pass when no text is clipped, no control is unreachable, and the overlay
   does not block the app's zoom.

### 2.4.1 Bypass Blocks

A repeating block must be bypassable.

1. Open an app with a large header, nav, and footer.
2. Open the icon grid and the simplified view.
3. Pass when both reach the main content without stepping through the repeated
   block, and the keyboard path does the same.

### 2.4.2 Page Titled

Each surface must have a title that describes it.

1. Open the overlay on an app with a visible window title.
2. Open each settings window.
3. Pass when every surface's title names the app or the settings page, and a
   screen reader announces it.

### 2.4.6 Headings and Labels

Every control must have a visible label or heading.

1. Open every settings window and the overlay.
2. Inspect each control, including icon-only buttons.
3. Pass when each control has a visible text label or an accessible name that a
   screen reader announces.

### 2.4.7 Focus Visible

The focused control must be visibly indicated.

1. Tab through every overlay surface and every settings window.
2. Pass when the focus indicator is visible at every step and has a contrast
   ratio of at least 3:1 against its background.

### 2.5.7 Dragging Movements

A dragging gesture must have a single-pointer alternative.

1. Inspect every control that appears to drag: sliders, panels, the reading
   ruler, the focus lens.
2. Pass when each has a click or keyboard alternative, or when the control does
   not exist.

### 3.2.1 On Focus

Moving focus must not change context.

1. Tab through every overlay surface.
2. Pass when focus alone never submits, navigates, or opens a dialog.

### 3.2.2 On Input

Changing a control must not change context unexpectedly.

1. Toggle every setting and inspect every list and slider.
2. Pass when a change applies only on an explicit action, and every accidental
   change is undoable.

### 3.2.3 Consistent Navigation

Navigation must be identical across apps.

1. Open the overlay on three different applications.
2. Compare the tile order, the header, and the navigation path.
3. Pass when the navigation is the same in all three.

### 3.2.4 Consistent Identification

The same action must use the same words everywhere.

1. Collect the label for each action across the overlay, the settings windows,
   and the help tile.
2. Pass when one action never has two different names.

### 3.2.6 Consistent Help

Help must be in the same place.

1. Open the overlay on three different applications.
2. Locate the help entry point.
3. Pass when help is the same tile in the same position every time.

### 3.3.1 Error Identification

An error must be identified in text.

1. Trigger a validation error: an out-of-range value, a bad import, a failed
   save.
2. Pass when the error names the field or the setting in text, not only by
   color or position.

### 3.3.2 Labels or Instructions

Controls must have visible labels or instructions.

1. Open every settings window.
2. Inspect every control, including the first run and the import flow.
3. Pass when each control has a label or an inline instruction.

### 3.3.3 Error Suggestion

An error should suggest a fix.

1. Trigger the same validation errors as in 3.3.1.
2. Pass when the message suggests the fix, or when the error is
   self-explanatory.

### 3.3.7 Redundant Entry

A value must not be entered twice.

1. Open the profile and import flows.
2. Pass when a previously entered value is reused, or when the second entry is
   not required.

### 3.3.8 Accessible Authentication

Authentication must not rely on a cognitive test.

1. Inspect every login, unlock, or pairing flow we ship.
2. Pass when none uses a CAPTCHA, a memory test, or a code transcribed from
   memory.

### 4.1.3 Status Messages

A status change must be announced.

1. Trigger a status change: a save, an import, a model update, a background
   analysis finishing.
2. Pass when a screen reader announces the change without moving focus.
