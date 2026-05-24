//! Abstraction over output destinations.
//!
//! v1: SystemDeviceBackend (cpal output stream).
//! v2: RatMicVirtualMicBackend (first-party virtual driver).
//!
//! The audio engine talks only to this trait; swapping backends does not affect
//! effects, presets, or UI.

use anyhow::Result;

use super::format::AudioFormat;

pub trait AudioOutputBackend: Send {
    fn name(&self) -> &str;
    fn open(&mut self, format: AudioFormat) -> Result<()>;
    /// Submit processed samples (interleaved if multi-channel).
    /// Returns the number of samples actually written.
    fn write(&mut self, samples: &[f32]) -> Result<usize>;
    fn close(&mut self);
}
