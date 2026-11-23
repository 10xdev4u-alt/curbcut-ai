//! Zeroizing byte and text buffers.
//!
//! Content that must not survive its owner is stored in these buffers. A
//! buffer is overwritten with [`GUARD_BYTE`] when it is dropped, so a frame, a
//! recognized phrase, or a sealed cache value cannot be read back from freed
//! memory.
//!
//! Safe Rust cannot reach into a [`String`]'s allocation, so [`SecureText`] is
//! byte-backed rather than a `String` and hands out a `&str` only while it is
//! alive. [`SecureBuffer`] is the byte form; [`SecureText`] is the UTF-8 form.
//! Both print a length, never their bytes, and both count drops so a test can
//! prove the overwrite happened. See
//! [`../../../docs/SECURE_BUFFERS.md`](../../../docs/SECURE_BUFFERS.md).

use std::fmt;

/// Byte written over released bytes.
pub const GUARD_BYTE: u8 = 0;

thread_local! {
    /// Counts buffers dropped on the current thread.
    static ZEROIZED_ON_DROP: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// How many secure buffers have been dropped on this thread.
///
/// A test hook: it makes zeroization observable without `unsafe`.
#[doc(hidden)]
pub fn zeroized_on_drop() -> usize {
    ZEROIZED_ON_DROP.with(|counter| counter.get())
}

/// Resets the per-thread drop counter.
///
/// A test hook.
#[doc(hidden)]
pub fn reset_zeroized_on_drop() {
    ZEROIZED_ON_DROP.with(|counter| counter.set(0));
}

/// An owned byte buffer that overwrites itself with [`GUARD_BYTE`] on drop.
///
/// Use it for pixels, ciphertext, or any byte string that must not be
/// recoverable after release. [`Debug`](fmt::Debug) prints the length and the
/// zeroized flag, never the bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct SecureBuffer {
    data: Vec<u8>,
}

impl SecureBuffer {
    /// Allocates a buffer of `len` bytes, prefilled with [`GUARD_BYTE`].
    pub fn new(len: usize) -> Self {
        Self {
            data: vec![GUARD_BYTE; len],
        }
    }

    /// Copies `bytes` into a buffer that overwrites itself on drop.
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self {
            data: bytes.to_vec(),
        }
    }

    /// Overwrites every byte with [`GUARD_BYTE`].
    pub fn zeroize(&mut self) {
        self.data.fill(GUARD_BYTE);
    }

    /// Whether every byte is the guard byte.
    pub fn is_zeroized(&self) -> bool {
        self.data.iter().all(|byte| *byte == GUARD_BYTE)
    }

    /// The bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// The bytes, mutably.
    ///
    /// Writing through this does not re-validate anything: the caller owns
    /// what it writes, and the buffer still zeroizes itself on drop.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Buffer length in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the buffer has length zero.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Drop for SecureBuffer {
    fn drop(&mut self) {
        self.zeroize();
        ZEROIZED_ON_DROP.with(|counter| counter.set(counter.get() + 1));
    }
}

impl fmt::Debug for SecureBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecureBuffer")
            .field("len", &self.data.len())
            .field("zeroized", &self.is_zeroized())
            .finish()
    }
}

/// UTF-8 text that overwrites itself with [`GUARD_BYTE`] on drop.
///
/// A [`String`] cannot be zeroized safely under `forbid(unsafe_code)`, so the
/// bytes live in a [`Vec<u8>`] and the text is borrowed back as a `&str`. The
/// buffer is only ever filled from a `String`, so [`SecureText::as_str`] cannot
/// fail. [`Debug`](fmt::Debug) prints the byte length, never the text.
#[derive(Clone, PartialEq, Eq)]
pub struct SecureText {
    bytes: Vec<u8>,
}

impl SecureText {
    /// Wraps `text` in a buffer that overwrites itself on drop.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            bytes: text.into().into_bytes(),
        }
    }

    /// The text as a string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// The text bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Text length in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Drop for SecureText {
    fn drop(&mut self) {
        self.bytes.fill(GUARD_BYTE);
        ZEROIZED_ON_DROP.with(|counter| counter.set(counter.get() + 1));
    }
}

impl fmt::Debug for SecureText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecureText")
            .field("len", &self.bytes.len())
            .finish()
    }
}

impl fmt::Display for SecureText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for SecureText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests;
