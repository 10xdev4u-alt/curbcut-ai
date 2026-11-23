# Zeroizing buffers

Content that must not survive its owner is stored in `curbcut_core::secure`.
A buffer is overwritten with `GUARD_BYTE` when it is dropped, so a frame, a
recognized phrase, or a sealed cache value cannot be read back from freed
memory.

This is a memory-hygiene control, not a security boundary. It reduces the
window in which content sits in freed allocations; it does not make capture
safe, and it is not a substitute for not storing content at all.

## The two types

| Type | Holds | Use |
| --- | --- | --- |
| `SecureBuffer` | bytes | Pixels, ciphertext, any byte string |
| `SecureText` | UTF-8 bytes | Recognized text, captions |

Both own a `Vec<u8>`, fill it with `GUARD_BYTE` on drop, and implement `Debug`
by printing the **length** and nothing else. Neither implements a logging
format: `Display` exists on `SecureText` so a caller can hand the text to a
renderer, and nothing else.

## Why `SecureText` is not a `String`

Safe Rust cannot reach into a `String`'s heap allocation, so zeroizing one
would require `unsafe`, and the workspace `forbid(unsafe_code)` forbids it.
`SecureText` keeps the bytes in a `Vec<u8>` instead and hands out a `&str`
through `as_str` only while the buffer is alive. The buffer is only ever
filled from a `String`, so `as_str` cannot fail; it returns an empty slice
rather than panicking if that invariant is ever broken.

## The guard byte

`GUARD_BYTE` is the single byte written over released content. The frame ring
in `curbcut-capture` re-exports it, so there is one definition of "released"
across the workspace rather than one per crate.

## Observability

A correct zeroize is invisible, which makes it hard to test. The buffers count
their own drops on a thread-local counter, exposed as `zeroized_on_drop` and
reset with `reset_zeroized_on_drop`. Both are `#[doc(hidden)]`: they are a test
hook, not an API. Thread-local means parallel tests cannot race on a shared
counter.

A test proves the overwrite by dropping a buffer and asserting the counter
moved; it does not read freed memory, which would be undefined behaviour.

## Where it is used

- `curbcut-capture`: `FrameBuffer` zeroizes on release and reuse. `GUARD_BYTE`
  is re-exported from `curbcut_core::secure`.
- `curbcut-vision`: `OcrText.text` and `IconCaption.caption` are `SecureText`.
- `curbcut-store`: the understanding cache holds sealed values in
  `SecureBuffer`, so an evicted entry is overwritten, not merely unlinked.

## Rules

- Never log a buffer's bytes. `Debug` prints a length; use it.
- Read text through `as_str` at the point of use, then let the buffer drop.
- Do not retain a clone past the owner's lifetime.
- Do not add `unsafe` to zeroize a `String`. Store text in `SecureText`.
