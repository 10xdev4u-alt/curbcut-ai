# CurbCut AI — Deep Research

> Status: planning input, 2026-11-14. Every factual claim has a source or is marked as a judgement. Where the evidence is thin, it says so.

---

## 1. Problem statement

### 1.1 The gap

Digital accessibility has a center of gravity: **blindness**. The tooling, the standards, the legal frameworks, and the public imagination all orbit screen readers, contrast ratios, alt text, and keyboard navigation. These matter enormously and must keep improving.

Cognitive accessibility is different:

- It is **invisible and heterogeneous**. ADHD, dyslexia, dyscalculia, autism, traumatic brain injury, dementia, intellectual disability, and low literacy each impose different barriers, often on the same person.
- It is **poorly served by standards**. WCAG 2.2 says plainly that it "will not address every user need for people with these disabilities." The W3C's cognitive guidance (COGA) is a **Working Group Note**, not a Recommendation, and is explicitly supplemental.
- It is **underdiagnosed and stigmatized**. Users are less likely to have a formal diagnosis, less likely to ask for accommodations, and more likely to be told to "just try harder."
- It is **commercially neglected**. The overlay market chased compliance (and got sued). The reading-support market (Read&Write, Immersive Reader, Helperbird) is mostly web- or document-bound. Nothing treats the whole desktop as a surface.

### 1.2 The human cost

From W3C COGA and the cognitive accessibility user research:

- Impaired working memory means some users can hold **1–3 items** at a time. A 12-step checkout is impossible.
- Impaired processing speed means a dense form with inline errors is not "a bit hard," it is a wall.
- Attention differences mean a single carousel, autoplay video, or toast notification can end a task.
- Reading differences mean long paragraphs, justified text, and low-contrast grey text are not aesthetic choices, they are access barriers.
- Memory differences mean passwords, access codes, and multi-step instructions need to be visible, not recalled.

The general population hits the same barriers under stress, fatigue, distraction, or a small screen. That is the curb cut.

### 1.3 Why now

1. **On-device vision is finally good enough.** OmniParser v2 (Microsoft, 2025) uses a YOLOv9-E interactable-region detector plus a caption model to turn screenshots into structured elements, and reports 39.5% on ScreenSpot Pro. It runs locally and is MIT-licensed for the detector. This makes "understand any app" viable without a cloud VLM.
2. **OCR is cheap and offline.** PaddleOCR PP-OCRv6 (Apache-2.0) supports 50 languages in one model and reports a 5.2× CPU inference speedup over PP-OCRv5; RapidOCR ships ONNX conversions of the same weights.
3. **Local TTS is invisible-good.** Piper (now OHF-Voice/piper1-gpl) is a fast local neural TTS with ONNX voices. Nothing about read-aloud requires the cloud.
4. **Small language models can simplify text locally.** A 1.5–3B instruction model quantized to int4 can rewrite a paragraph at reading level in well under a second on a modern CPU. ONNX GenAI or llama.cpp makes this deployable.
5. **The law moved.** The EU European Accessibility Act applied from 28 June 2025 for newly marketed products and services. EN 301 549 is the harmonized standard. Employers and procurers now have a reason to buy.

---

## 2. Prior art and positioning

### 2.1 Screen readers (the incumbent)

NVDA, JAWS, VoiceOver, Narrator, Orca. Excellent for blind users. Wrong tool for most cognitive users: they linearize, they read everything, they do not simplify, they do not dim, they do not chunk, and they assume you can hold a lot in memory. CurbCut must **cooperate** with them, not replace them. Concretely: CurbCut must be a good citizen on the a11y bus, must not fight the screen reader for audio focus, and must expose its own controls through the platform a11y API.

### 2.2 Reading and literacy support

- **Read&Write (Texthelp)** — literacy toolbar, read-aloud, dictionary, simplification. Web + desktop, but it attaches to documents and browsers, not arbitrary apps.
- **Immersive Reader (Microsoft)** — excellent focus mode, spacing, syllables, read-aloud. Locked inside Microsoft products.
- **Helperbird** — browser extension, dyslexia fonts, overlays, read-aloud. Web only.
- **BeeLine Reader** — color-gradient reading. Web only, and the evidence for it is mixed.
- **NaturalReader, Speechify** — TTS, mostly cloud.

None of these treat a native app, a terminal, a game, or a remote desktop as a surface.

### 2.3 Focus and attention tools

- **Freedom, Cold Turkey, StayFocusd** — blockers. They remove distractions but do not re-present content.
- **Browser reader modes** — simplify a page, but only in the browser and only if the page is article-like.
- **macOS Reduce Motion / Increase Contrast** — system-wide but static and shallow.

### 2.4 GUI agents (the adjacent threat)

OmniParser + a VLM, OpenAdapt, UFO, Windows Agent Arena, Claude Computer Use. These are **agents**: they act on the UI. CurbCut is the opposite: it **presents** the UI to a human and only acts when the human asks. The agent work is a supplier (detection models) and a cautionary tale (latency, reliability, trust).

### 2.5 Accessibility overlays (the cautionary tale)

accessiBe raised $58M, sold a one-line JavaScript widget as ADA compliance, was condemned by the National Federation of the Blind, and in January 2025 the FTC ordered it to pay $1M for deceptive claims. The category is toxic for good reasons: overlays that promise compliance without changing code are usually lying, and they often break the assistive tech they claim to help.

CurbCut's positioning must be explicitly anti-accessiBe: user-licensed not site-licensed, local not SaaS, measured not marketed, cross-app not web-only, and honest that it improves usability rather than conferring compliance.

---

## 3. Market

### 3.1 Demand-side

- ~1 in 5 adults in the US has a learning disability (NCLD).
- ADHD affects roughly 5% of adults globally; dyslexia roughly 5–10%.
- The global assistive technology market is sized in the tens of billions and growing, but the cognitive slice is under-penetrated.
- Employers are the near-term buyer: accommodations, retention, DEI, and EAA-driven procurement. Individuals are the long-term buyer.

### 3.2 Buyer personas

1. **Individual user** — wants it to "just work," will pay $5–15/mo or a one-time license, hates subscriptions for accessibility tools, values privacy.
2. **Employer / IT** — needs MDM, SSO-less local install, no data leaving the endpoint, measurable accommodations, no legal exposure.
3. **Procurer / public sector** — needs EN 301 549 / WCAG 2.2 mapping, VPAT, documentation, local procurement compliance.
4. **School / university disability office** — needs bulk licensing, simple setup, works on lab machines and student laptops.

### 3.3 Competitive moat

- **Cross-app perception** is hard and defensible: it needs capture + vision + a11y-merge + a render layer that works over arbitrary pixels. No incumbent has all four.
- **Local-first trust** is a moat in enterprise and government.
- **The profile graph** (app-specific layouts, learned simplifications, memory aids) gets better with use and is hard to clone.
- **Advisory panel credibility** is a moat against the next accessiBe.

---

## 4. Regulatory and standards landscape

### 4.1 WCAG 2.2

The normative standard. Relevant criteria for CurbCut (not exhaustive):

- 1.3.1 Info and Relationships — our simplified view must preserve structure.
- 1.4.8 / 1.4.12 Visual Presentation / Text Spacing — spacing, contrast, line length.
- 2.2.1 / 2.2.2 Timing Adjustable / Pause, Stop, Hide — we can pause and hide.
- 2.4.1 Bypass Blocks — our icon grid is a bypass.
- 3.1.5 Reading Level — we provide plain-language alternatives.
- 3.2.6 Consistent Help — our help must be consistent.
- 3.3.7 Redundant Entry — we must not make users re-enter data.
- 4.1.2 Name, Role, Value — our own UI must expose these.

Crucially: **CurbCut does not make a third-party app conform.** It provides an alternative perceptual channel. That is a legitimate accommodation but not a conformance claim. We will say this in the UI, in the docs, and in the installer.

### 4.2 COGA (W3C cognitive guidance)

The design bible. Eight objectives, dozens of patterns. We map our features to them in [`A11Y_MAPPING.md`](A11Y_MAPPING.md). Highest-leverage patterns for us:

- Support simplification (4.9.3)
- Let users control when content moves or changes (4.9.1)
- Limit interruptions (4.6.1)
- Make short critical paths (4.6.2)
- Avoid too much content (4.6.3)
- Do not rely on memory (4.7)
- Provide summaries of long documents (4.4.8)
- Separate each instruction (4.4.9)
- Use white space (4.4.10)
- Provide human help (4.8.1)

### 4.3 EU EAA / EN 301 549

- Applies from 28 June 2025 to newly marketed in-scope products and services.
- Self-assessment + technical documentation; EN 301 549 gives presumption of conformity.
- Accessibility widgets are explicitly called out as **not sufficient on their own**. This is a risk for us only if we market as a compliance widget. We do not.

### 4.4 US

ADA Title II (state/local) and Title III (public accommodations), Section 508 (federal), Section 504 (education), and a wave of state laws. Litigation continues to target web properties, not desktop overlays, but the trend is broader.

### 4.5 Privacy law

Screen contents can contain health, financial, and personal data. GDPR/CCPA apply if any data leaves the device. Our design makes that a non-issue: **no content leaves the device.** We still need a privacy policy, a data inventory, and a DPIA for the optional cloud features.

---

## 5. Technical landscape

### 5.1 Screen capture

| OS | API | Notes |
| --- | --- | --- |
| macOS 12.3+ | ScreenCaptureKit | Stream-based, per-window or per-display, efficient, needs Screen Recording permission. |
| macOS (fallback) | CGDisplayStream / CGWindowListCreateImage | Older, more permission friction. |
| Windows 10 1803+ | Windows.Graphics.Capture | Efficient, needs Graphics Capture permission (consent dialog). |
| Windows (fallback) | GDI BitBlt / DXGI Desktop Duplication | BitBlt is slow; DXGI is good but can fail on protected content. |
| Linux (Wayland) | PipeWire + xdg-desktop-portal | The only sane Wayland path. Needs portal consent. |
| Linux (X11) | XShmGetImage / XCB | Works, but X11 is fading. |

Hard cases: DRM-protected video, secure desktops (UAC on Windows, login window on macOS), remote desktop sessions, and hardware overlays. We must detect and gracefully degrade ("CurbCut can't see this window") rather than pretend.

### 5.2 Accessibility trees

| OS | API | Binding |
| --- | --- | --- |
| macOS | AXUIElement (Application Services / Cocoa) | C/Objective-C, wrapped in Rust via `objc2` / `core-foundation`. |
| Windows | UI Automation (COM) | `windows-rs` / `UIAutomationClient.h`. |
| Linux | AT-SPI over D-Bus (libatspi) | `zbus` + `atspi` crate. |

The tree is a **semantic accelerator**, not the source of truth. Many apps (Electron, Qt, games, canvas UIs, remote desktops) have a thin or empty tree. Vision covers the gaps. The merge is the interesting engineering problem.

### 5.3 Vision models

| Task | Candidate | License | Notes |
| --- | --- | --- | --- |
| Interactable element detection | OmniParser v2 `icon_detect_v3` (YOLOv9-E) | MIT | SOTA on ScreenSpot Pro; ONNX export needed. |
| Fallback detector | YOLOv11n / YOLOv9c | GPL/AGPL (Ultralytics) | Watch licensing; use a MIT/Apache alternative or train our own. |
| OCR | PaddleOCR PP-OCRv6 (via RapidOCR ONNX) | Apache-2.0 | 50 languages, fast on CPU. |
| Icon captioning | Florence-2-base / Qwen2.5-VL-3B | MIT / Apache | Generates "what this icon does". |
| Text simplification | Qwen2.5-1.5B-Instruct int4 / T5-small paraphrase | Apache / MIT | Local, no network. |
| Reading level | Flesch-Kincaid + SMOG + trained classifier | n/a | Deterministic, testable. |
| Saliency / distraction | heuristics + optional small model | n/a | Motion detection is cheaper and more reliable. |

ONNX Runtime execution providers: CoreML (macOS), DirectML (Windows), QNN/NNAPI (mobile), OpenVINO (Intel), CUDA/TensorRT (NVIDIA), XNNPACK (CPU fallback). Quantization: int8 for detection, int4 for the LLM, fp16 where accuracy matters.

### 5.4 TTS

Piper (OHF-Voice/piper1-gpl) is the default: local, ONNX, MIT/GPL depending on the build. Alternatives: `espeak-ng` (robotic, zero cost, great for privacy), `kokoro` (newer neural voices). We should ship espeak as an instant fallback and Piper for quality.

### 5.5 Text-to-meaning

- **Rule layer** — jargon dictionary, acronym expansion, number formatting, date formatting, reading level scoring. Deterministic and testable.
- **Model layer** — local instruct model for simplification, summarization, icon labeling, and "what will this button do?" Cached per element hash.

### 5.6 Overlay rendering

Tauri window with `transparent`, `decorations: false`, `always_on_top`, `skip_taskbar`. Two strategies:

1. **In-place overlay** — floating panels over the real UI. Low latency, but alignment breaks when the underlying app scrolls or animates.
2. **Simplified view** — a separate window that re-presents the app's content in a clean layout. More work, much better for comprehension. This is the flagship.

Both need a "pin/lock" model and a way to hand focus back to the real app.

### 5.7 Input

- Global shortcuts: Tauri `global-shortcut`.
- Mouse hover tracking: raw input / CGEventTap (macOS), SetWindowsHookEx (Windows), X11/XInput or evdev (Linux). All need accessibility permissions.
- Click-through: hit-testing our overlay; if the user clicks a proxy control, we either click the real control (with consent) or act in our simplified view.
- Voice control: whisper.cpp / `whisper-rs` for command keywords, plus TTS for confirmation.

---

## 6. Key risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| Permission friction (screen recording, accessibility, microphone) | High | First-run guided setup, per-feature toggles, clear "why we need this" copy, graceful degradation. |
| False confidence ("it says compliant") | High | Never say compliant. Publish limits. Advisory panel review of marketing. |
| Overlay breaks the underlying app | High | Read-only by default. Actions require explicit opt-in per session. |
| Privacy breach from screen data | Critical | Local-only, encrypted store, no logging of content, memory-zeroing, third-party audit. |
| Model hallucination in simplification | Medium | Preserve meaning contract, user-visible "show original", deterministic fallback, human review of samples. |
| Performance / battery | High | Adaptive frame rate, region-of-interest capture, int8/int4 quantization, pause when idle, CPU/GPU budget. |
| Accessibility-tree merge bugs | Medium | Property-based tests, differential testing against a screen reader, fuzzing with synthetic trees. |
| Platform fragmentation | High | Adapter trait + feature flags, CI on real macOS/Windows/Linux runners, VM farm. |
| Regulatory drift | Medium | Standards watchdog, EN 301 549 mapping kept in-repo, legal review. |
| Community trust (open source + paid) | Medium | Clear license, public roadmap, paid features are enterprise-only (MDM, sync, support), core stays free. |

---

## 7. Licensing analysis

Options:

- **AGPL-3.0 core** — prevents proprietary relicensing, forces improvements back. Risk: enterprise avoidance.
- **MPL-2.0 core** — file-level copyleft, friendlier to embedding. Weaker protection.
- **Apache-2.0 core + commercial enterprise add-on** — maximum adoption, weakest protection, easiest to be cloned.
- **Dual license (AGPL + commercial)** — the Qt/Better SQLite model. Best for sustainability, needs a legal entity and a CLA.

Recommendation: **AGPL-3.0 for the core, with a commercial exception for OEM/enterprise embedding**, plus a contributor CLA. Revisit at Series A. The accessibility layer should not be ownable in a way that lets someone sell it back to the community as a closed product.

---

## 8. Success metrics

### 8.1 Product

- Task completion rate for each persona on a fixed benchmark suite (e.g., book a doctor, file an expense, read a policy).
- Time-on-task vs. baseline, with a target of −25% for the target personas.
- Cognitive load proxy: NASA-TLX or a simple 3-item self-report after each task.
- Error rate and recovery time.
- Retention: D1/D7/D30, weekly active sessions, profile-switch frequency.

### 8.2 Engineering

- Detection precision/recall on a held-out screen dataset (target: >0.85 mAP for interactable regions).
- OCR CER/WER per language (target: CER < 5% for Latin scripts).
- Simplification meaning-preservation score (human-rated, target: >4/5).
- End-to-end latency p50/p95 (target: p95 < 250 ms for hover-to-speak).
- CPU/RAM/battery budget (target: < 150 MB RAM idle, < 5% CPU idle, < 1%/min battery idle).
- Zero critical CVEs; audit findings closed within SLA.

### 8.3 Trust

- Advisory panel satisfaction (quarterly).
- Number of published limitations.
- Percentage of features that work fully offline (target: 100% of core).

---

## 9. Open questions

1. Do we ship a browser extension first as a wedge, or go straight to desktop? (Recommendation: desktop core, extension later as a thin client.)
2. Do we support mobile? (Recommendation: companion app only, no overlay on mobile — the OS already has strong cognitive features and the overlay model doesn't fit.)
3. Do we build our own detector or fine-tune OmniParser? (Recommendation: start with OmniParser v2 ONNX, fine-tune on our own curated desktop dataset in Phase 5.)
4. What is the pricing model? (Recommendation: free core, paid Pro, enterprise agreement. Decide after beta.)
5. Who is liable for a bad simplification? (Recommendation: never simplify financial, medical, or legal content automatically; show a warning and the original. Legal review required.)

---

## 10. Sources

- W3C, *Web Content Accessibility Guidelines (WCAG) 2.2*, Recommendation, 12 Dec 2024. <https://www.w3.org/TR/WCAG22/>
- W3C COGA TF, *Making Content Usable for People with Cognitive and Learning Disabilities*, WG Note, 29 Apr 2021. <https://www.w3.org/TR/coga-usable/>
- Microsoft Research, *OmniParser for Pure Vision Based GUI Agent*, arXiv 2408.00203. <https://microsoft.github.io/OmniParser/> · <https://github.com/microsoft/OmniParser>
- PaddleOCR, PP-OCRv6 / PaddleOCR-VL. <https://github.com/PaddlePaddle/PaddleOCR>
- RapidOCR (ONNX conversions, Apache-2.0). <https://github.com/RapidAI/RapidOCR>
- Piper / OHF-Voice, local neural TTS. <https://github.com/OHF-Voice/piper1-gpl>
- Microsoft, *UI Automation Overview*. <https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-uiautomationoverview>
- ONNX Runtime docs, execution providers and quantization. <https://onnxruntime.ai/docs/>
- Tauri, *What is Tauri?* and the official plugins workspace. <https://tauri.app/start/> · <https://github.com/tauri-apps/plugins-workspace>
- PipeWire, screen capture on Linux/Wayland. <https://www.pipewire.org/>
- Wikipedia, *accessiBe* (funding, NFB resolutions, FTC order). <https://en.wikipedia.org/wiki/AccessiBe>
- Wikipedia, *European Accessibility Act* (28 June 2025 application, EN 301 549, widget insufficiency). <https://en.wikipedia.org/wiki/European_Accessibility_Act>
- FTC, *Order Requires Online Marketer to Pay $1 Million for Deceptive Claims*, Jan 2025.
