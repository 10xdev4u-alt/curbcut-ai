# Platform Adapters

> Concrete per-OS integration notes: APIs, permissions, failure modes, and the exact Rust crates. This is the doc engineers read before touching `curbcut-capture`, `curbcut-a11y`, `curbcut-input`, or `curbcut-tts`.

---

## 1. macOS

### 1.1 Screen capture — ScreenCaptureKit

- **API:** `ScreenCaptureKit` (macOS 12.3+). `SCShareableContent` → `SCStream` with `SCStreamOutput`.
- **Rust path:** `objc2` + hand-written bindings, or `screencapturekit-rs` if maintained. Ship our own thin wrapper behind the `CaptureAdapter` trait.
- **Permissions:** Screen Recording (`kTCCServiceScreenCapture`). Must be granted in System Settings → Privacy & Security. TCC prompts cannot be automated; we detect and guide.
- **Window vs. display:** Prefer per-window `SCWindow` for the active window; fall back to `SCDisplay` for multi-window.
- **Failure modes:**
  - `SCStreamError` code 6 → no permission. Show the guide.
  - Protected content (DRM, FaceTime) → black frames. Detect and message.
  - App switcher / secure desktop → no content. Degrade.
- **Performance:** `SCStreamConfiguration` with `minimumFrameInterval`, `scalesToFit`, pixel format `kCVPixelFormatType_32BGRA`. Avoid `CGWindowListCreateImage` except as a fallback.

### 1.2 Accessibility — AXUIElement

- **API:** `AXUIElement` (Application Services / Cocoa). `AXUIElementCreateApplication`, `AXUIElementCopyAttributeValue`, `AXUIElementSetAttributeValue`.
- **Rust path:** `objc2-application-services` or `core-foundation` + `cocoa`.
- **Key attributes:** `AXRole`, `AXSubrole`, `AXTitle`, `AXDescription`, `AXValue`, `AXEnabled`, `AXFocused`, `AXPosition`, `AXSize`, `AXChildren`, `AXSelectedChildren`, `AXMainWindow`, `AXFocusedWindow`, `AXExtrasMenuBar`.
- **Permissions:** Accessibility (`kTCCServiceAccessibility`). The app must be in the allow-list; we cannot add ourselves.
- **Notifications:** `AXObserver` for `AXFocusedUIElementChanged`, `AXValueChanged`, `AXFocusedWindowChanged`, `AXWindowCreated`, `AXWindowMoved`, `AXLayoutChanged`.
- **Failure modes:**
  - Electron/Chromium apps expose a tree only if the app enables it; otherwise vision-only.
  - Games and video players: empty tree.
  - Secure fields (`AXSecureTextField`) may redact `AXValue`. Never try to bypass.
- **Hard rule:** we read the tree; we never write `AXValue` or perform `AXPress` without the user's explicit action consent.

### 1.3 Input

- **Global shortcuts:** Tauri `global-shortcut`, or `CGEventTap` for carbon-era shortcuts.
- **Hover:** `CGEventTap` on `kCGEventMouseMoved` (needs Accessibility + Input Monitoring).
- **Click-through:** an `NSPanel` with `ignoresMouseEvents = true` and per-control `hitTest` overrides; or a `CGWindowLevel` overlay.
- **Panic:** `CGEventTap` hotkey + `NSPanel.orderOut`.

### 1.4 TTS

- **Piper:** sidecar binary or `libpiper` bindings; output to `AVAudioEngine`.
- **Fallback:** `AVSpeechSynthesizer` (system voices, offline) — better than espeak on macOS and no download.
- **Audio focus:** `AVAudioSession` category `.playback` with `.duckOthers`; feed a verified announcement start to `coexist::Coexistence::observe` as `ScreenReader::Speaking`, its end back to `ScreenReader::Listening`, and `ScreenReader::Off` only when the screen reader stops running. Read `level()` so an announcement is never talked over. `AXAnnouncement` only requests an announcement and does not report speech start or end, so it is not a speech-window source. See [`COEXISTENCE.md`](COEXISTENCE.md).

### 1.5 Packaging

- `.app` bundle + `.dmg`; notarize with `notarytool`; staple the ticket.
- Hardened runtime + `com.apple.security.cs.allow-jit` only if the LLM runtime needs it (prefer not).
- Update via Tauri updater with a signed tarball.

---

## 2. Windows

### 2.1 Screen capture — Windows.Graphics.Capture

- **API:** `Windows.Graphics.Capture` (`GraphicsCaptureItem`, `Direct3D11CaptureFramePool`, `GraphicsCaptureSession`).
- **Rust path:** `windows-rs` with the `GraphicsCapture` feature, or `windows-capture` crate.
- **Permissions:** a one-time consent dialog ("Allow CurbCut to capture this window?"). Cannot be pre-granted.
- **Fallbacks:**
  - `DXGI` Desktop Duplication (`IDXGIOutputDuplication`) — good for full display, fails on protected content and some full-screen apps.
  - GDI `BitBlt` — slow, last resort.
- **Failure modes:**
  - `GraphicsCaptureAccessRequestResult::Denied` → guide the user.
  - Protected content (Widevine, secure desktop) → black frames. Detect and message.
  - UAC prompts and the lock screen are not capturable. Do not try.
- **Performance:** capture at the display's native rate, but throttle our analysis to 2 fps. Use a `Direct3D11` texture → `oronx`/`image` conversion via a shared buffer; avoid per-frame copies where possible.

### 2.2 Accessibility — UI Automation

- **API:** UI Automation COM (`IUIAutomation`, `IUIAutomationElement`, `IUIAutomationTreeWalker`, control patterns).
- **Rust path:** `windows-rs` with `Win32_UI_Accessibility`.
- **Key properties:** `Name`, `ControlType`, `LocalizedControlType`, `IsEnabled`, `HasKeyboardFocus`, `IsOffscreen`, `BoundingRectangle`, `IsPassword`, `IsContentElement`, `IsControlElement`.
- **Key patterns:** `Invoke`, `Value`, `Toggle`, `SelectionItem`, `ExpandCollapse`, `Scroll`, `Transform`, `Window`.
- **Events:** `FocusChanged`, `PropertyChanged`, `StructureChanged`, `WindowOpened/Closed`.
- **Permissions:** UIA works without a special permission for most apps, but elevating to UI Access (`uiAccess="true"` in the manifest) allows the overlay to sit above elevated windows. That requires signing and installing into `Program Files`. We ship without UI Access by default.
- **Failure modes:**
  - WPF/WinUI expose rich trees; Win32 is partial; Electron depends on the app; games are empty.
  - UIA cannot cross `Run as different user` boundaries.
- **Hard rule:** we never call `Invoke` or `SetValue` without explicit action consent.

### 2.3 Input

- **Global shortcuts:** `RegisterHotKey` / Tauri `global-shortcut`.
- **Hover:** `SetWindowsHookEx(WH_MOUSE_LL)`.
- **Click-through:** `WS_EX_TRANSPARENT` + `WS_EX_LAYERED` + `WS_EX_NOACTIVATE`; per-control hit-testing.
- **Panic:** `RegisterHotKey` + destroy overlay windows.

### 2.4 TTS

- **Piper:** sidecar or bindings; output to WASAPI.
- **Fallback:** `SAPI` (`ISpVoice`) — offline, built in, robotic but instant.
- **Audio focus:** WASAPI session-level ducking; report Narrator announcement windows to `coexist::Coexistence::observe` and read `level()`. See [`COEXISTENCE.md`](COEXISTENCE.md).

### 2.5 Packaging

- `.msi` via WiX or `tauri-bundler`; EV code signing + Azure Trusted Signing.
- Per-user install (no admin) by default; per-machine for enterprise via MDM.
- Update via Tauri updater with signed artifacts.

---

## 3. Linux

### 3.1 Screen capture

- **Wayland (primary):** PipeWire + `xdg-desktop-portal` (`org.freedesktop.portal.ScreenCast`). Use `xdp-gnome`/`xdp-wl` or `ashpd`.
- **Rust path:** `ashpd` (async portal client) + `pipewire` crate (or `libspa` bindings) for the stream.
- **Permissions:** the portal asks the user; no persistent permission by default (restorable via the portal's settings).
- **X11 (fallback):** `x11rb` `GetImage` / `XShmGetImage`.
- **Failure modes:**
  - No portal (headless, minimal GNOME/KDE) → X11 fallback or "unsupported."
  - Remote Wayland (RDP/VNC) → often no portal; degrade.
  - NVIDIA + Wayland quirks → test matrix.
- **Performance:** request the smallest scale that still meets our analysis resolution; use DMA-BUF zero-copy where available.

### 3.2 Accessibility — AT-SPI

- **API:** AT-SPI2 over D-Bus (`org.a11y.atspi.Registry`, `org.a11y.atspi.Accessible`).
- **Rust path:** `atspi` crate (zbus-based) or `zbus` + hand-rolled proxies.
- **Key methods:** `GetRole`, `GetName`, `GetDescription`, `GetValue`, `GetChildren`, `GetState`, `GetExtents` (screen coordinates), `GetFocused`.
- **Events:** `PropertyChange`, `StateChanged`, `ChildrenChanged`, `Focus`.
- **Permissions:** none beyond the user session; but many toolkits (GTK4, Qt, Electron) need `accessibility` enabled. On GTK, set `GTK_A11Y=atspi`; on Qt, `QT_ACCESSIBILITY=1`.
- **Failure modes:**
  - Electron/Chromium on Linux: AT-SPI only if the app enables it.
  - Games, terminals (mostly), and remote apps: empty.
  - X11 vs. Wayland coordinate spaces: use screen coordinates, not window-relative.
- **Hard rule:** read only; never `DoAction` without consent.

### 3.3 Input

- **Global shortcuts:** Tauri `global-shortcut` (works on X11; on Wayland, use the portal's `org.freedesktop.portal.Shortcuts` if available, else a tray + X11 grab).
- **Hover:** X11 `XInput2`; on Wayland, there is no global pointer — use the overlay's own pointer events and accept the limitation.
- **Click-through:** a layer-shell surface (`wlr-layer-shell` via `gtk4-layer-shell`) with an input region that excludes our controls; on X11, `xcb` shape masks.
- **Panic:** media key or a tray menu item.

### 3.4 TTS

- **Piper:** sidecar; output to PulseAudio/PipeWire.
- **Fallback:** `espeak-ng` via `speech-dispatcher` or direct.
- **Audio focus:** PulseAudio stream routing + ducking; report AT-SPI `announcement` events to `coexist::Coexistence::observe` and read `level()`. See [`COEXISTENCE.md`](COEXISTENCE.md).

### 3.5 Packaging

- `.AppImage` (universal), `.deb` (Debian/Ubuntu), Flatpak (Flathub), AUR.
- Flatpak needs the `screen-cast` and `a11y` portals; document the permissions.
- No auto-update on Flatpak (Flathub handles it); Tauri updater for AppImage/deb.

---

## 4. Adapter trait

```rust
#[async_trait]
pub trait CaptureAdapter: Send + Sync {
    async fn enumerate(&self) -> Result<Vec<CaptureTarget>>;
    async fn start(&mut self, target: CaptureTarget, config: CaptureConfig) -> Result<FrameStream>;
    async fn stop(&mut self) -> Result<()>;
    fn permissions(&self) -> Vec<Permission>;
}

#[async_trait]
pub trait A11yAdapter: Send + Sync {
    async fn root(&self, app: AppId) -> Result<A11yNode>;
    async fn subscribe(&self, events: Vec<A11yEvent>) -> Result<EventStream>;
    fn permissions(&self) -> Vec<Permission>;
}
```

A `FakePlatform` implements both and drives the entire test suite.

---

## 5. Permission UX

Every permission has:

1. A plain-language "why."
2. A "not now" path that leaves the feature off.
3. A system-settings deep link.
4. A detection of the current state, so we don't nag.
5. A fallback feature that works without it (e.g., vision-only without a11y).

Never ask for a permission at launch. Ask at the moment the feature is first used.

---

## 6. Coordinate systems and DPI

- All internal rects are in **logical screen coordinates** (points on macOS, DIPs on Windows, logical pixels on Linux).
- Convert to/from physical pixels at the capture boundary.
- Handle mixed-DPI displays: an element's rect is stored in logical coordinates for its display; the renderer converts per-window.
- Multi-display: `DisplayId` is part of every rect key.

---

## 7. Remote desktops and VMs

- RDP/TeamViewer/VNC are captured as pixels; the a11y tree is absent. Vision-only mode.
- Latency is higher; reduce the analysis rate and prefer region-of-interest.
- Never capture a secure desktop or a password field; detect `IsPassword`/`AXSecureTextField` and blank the region in the overlay.

---

## 8. Testing per OS

- **macOS:** a CI runner with a real window server (GitHub Actions `macos-14/15`), synthetic apps (Electron + Swift), permission-state tests.
- **Windows:** `windows-latest` runner, `Electron` + WinUI test apps, `Inspect.exe` dumps as fixtures.
- **Linux:** `ubuntu-latest` with `Xvfb` + a virtual PipeWire session, GTK/Qt test apps, `accerciser` dumps.
- **Golden files:** per-OS element dumps for the same fixture app, with tolerances for platform differences.
