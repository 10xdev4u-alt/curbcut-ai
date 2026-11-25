# Models

> The model zoo, budgets, quantization, and fallback chain. All models run on-device. Model files are pinned by hash and signed.

---

## 1. Model inventory

| ID | Task | Model | Precision | Size | EP priority | License |
| --- | --- | --- | --- | --- | --- | --- |
| `det-v2` | Interactable element detection | OmniParser v2 `icon_detect_v3` (YOLOv9-E) | int8 | ~120 MB | CoreML / DirectML / CUDA / OpenVINO / XNNPACK | MIT |
| `ocr-v6` | Text detection + recognition | PP-OCRv6 (via RapidOCR ONNX) | int8 | ~30 MB | same | Apache-2.0 |
| `cap-v2` | Icon captioning | Florence-2-base | int8 | ~150 MB | same | MIT |
| `llm-v1` | Simplification / summary | Qwen2.5-1.5B-Instruct | int4 | ~1.2 GB | CoreML / DirectML / CUDA / XNNPACK | Apache-2.0 |
| `lid-v1` | Language ID | fastText `lid.176` | fp32 | ~130 MB | CPU | CC-BY-SA-3.0 (verify) |
| `tts-v1` | Speech | Piper voices (en_US, en_GB, …) | fp32/int8 | ~60 MB/voice | CPU | MIT/GPL (verify) |
| `asr-v1` | Voice commands | whisper.cpp `small` / a 10M keyword spotter | int8 | ~250 MB / ~10 MB | CPU | MIT |

Sizes are download sizes; quantized in-memory footprints are smaller. The `llm-v1` int4 model is the only large download and is optional (rules-only mode is the fallback).

`asr-v1` is a planned local model; no code loads it today. See
[`docs/VOICE_PIPELINE.md`](VOICE_PIPELINE.md).

---

## 2. Latency budgets

Measured on 1080p capture, active window, warm models.

| Stage | p50 | p95 | Budget |
| --- | --- | --- | --- |
| Capture (active window) | 8 ms | 20 ms | 33 ms |
| Detection (int8) | 45 ms | 90 ms | 120 ms |
| OCR (Latin) | 60 ms | 140 ms | 200 ms |
| Icon caption (cached miss) | 180 ms | 350 ms | 500 ms |
| Simplification (200 words, int4) | 700 ms | 1500 ms | 2000 ms |
| Plan build | 10 ms | 40 ms | 60 ms |
| Render (overlay) | 8 ms | 20 ms | 33 ms |
| Hover-to-speak (end to end) | 120 ms | 250 ms | 400 ms |

Budgets are checked in CI on three reference machines. A regression > 20% fails the build.

---

## 3. Execution providers

| OS | 1st | 2nd | 3rd | Fallback |
| --- | --- | --- | --- | --- |
| macOS | CoreML | CPU (XNNPACK) | — | fp32 CPU |
| Windows | DirectML | CUDA/TensorRT | OpenVINO | fp32 CPU |
| Linux | CUDA/TensorRT | OpenVINO | ROCm | fp32 CPU |
| ARM | QNN/NNAPI | XNNPACK | — | fp32 CPU |

Provider order is decided at load time; a failure to initialize falls through. The user can override in settings (advanced).

---

## 4. Quantization policy

- **Detection:** int8 (accuracy drop < 0.5% expected). Validate per release.
- **OCR:** int8. Validate per language; CJK may need fp16 if CER regresses.
- **Captioning:** int8. Validate on the icon fixture set.
- **LLM:** int4 (AWQ/GPTQ-style). Validate with the meaning-preservation benchmark; if it fails, ship fp16 on Apple Silicon and int8 elsewhere.
- **TTS:** ship the upstream precision; do not requantize without a listening test.
- **ASR:** int8. Validate on the command set.

Every quantization is a separate model artifact with its own hash, benchmark row, and rollback path.

---

## 5. Fallback chain

1. **Full pipeline** (detector + OCR + caption + LLM).
2. **Rules-only** (detector + OCR + heuristics; no LLM). Used when the LLM is missing, fails, or is disabled.
3. **A11y-only** (tree + heuristics; no vision). Used when screen capture is denied, the screen is protected, or the detector crashes.
4. **Static overlay** (no perception; just the user's saved profile and manual tools). Used when everything else fails.

The fallback is announced in the UI ("CurbCut is running in rules-only mode") and never silent.

---

## 6. Model update policy

- Models are pinned by SHA-256 in `models/manifest.json` and authenticated by a
  release signature. A model that fails either check is never loaded; see
  [`MODEL_GUARD.md`](MODEL_GUARD.md).
- Updates ship with app updates; no silent model swaps.
- A new model must beat the current one on the benchmark by a margin (≥ 1% mAP / ≥ 0.5% CER) or it does not ship.
- Rollback: keep the previous model for one release; a user can pin a model version in settings.

---

## 7. Training and fine-tuning

- **Detector:** start from OmniParser v2; fine-tune on our desktop fixture set (Phase 5) with a focus on small icons, dense toolbars, and dark themes.
- **OCR:** use PP-OCRv6 as-is; fine-tune only if a language underperforms.
- **Captioning:** fine-tune on icon + context pairs from the fixture set; keep the prompt contract stable.
- **Simplifier:** fine-tune on a curated parallel corpus of dense → plain text, with hard constraints (numbers, negation, entities). Publish the corpus if licensing allows.
- **Evaluation:** every model change runs the full benchmark + a human-rated sample. No model ships on benchmark numbers alone.

---

## 8. Model card (template)

```markdown
# Model card: det-v2

- Task: interactable element detection
- Architecture: YOLOv9-E
- Training data: ... (license, consent, known biases)
- Quantization: int8 PTQ
- Benchmarks: mAP 0.xx on fixture-vN; per-OS latency table
- Known failures: small icons < 12 px, low-contrast-on-low-contrast, dark-on-dark
- License: MIT
- Hash: sha256:...
- Rollback: det-v1
```

Every model ships with a card. The cards are published in the app's "About models" panel and on the website.
