# Limitations

> What CurbCut honestly cannot do. This page is generated from the same data as
> the accessibility mapping and the draft VPAT, so a limitation cannot be
> dropped from one place and left in another. `CC-097`.

## Published limitations

1. We cannot see protected/DRM content or secure desktops.
2. We cannot reach into a native app's internals; we cover pixels, not code.
3. Detection is probabilistic; we publish mAP and CER and a per-app confidence indicator.
4. Simplification can be wrong; we check meaning and always show the original.
5. We do not support every language equally; we publish the language matrix.
6. Linux Wayland support depends on the portal; some distros are beta.
7. We are not a screen reader and do not replace one.
8. We are not a compliance tool and do not make apps conform.

## How to read these

Each limitation is a scope statement, not a defect. A limitation is changed by
building the feature, not by editing this page: edit
`crates/curbcut-cli/src/vpat.rs`, then run
`python3 scripts/render_a11y_mapping.py`.

A limitation that is fixed moves to the release notes and is removed here. A
limitation that grows becomes a `Partially Supports` row in
[`VPAT.md`](VPAT.md).
