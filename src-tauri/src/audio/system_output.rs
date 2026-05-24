//! cpal-based system output device backend.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use super::format::{f32_to_i16, mono_to_interleaved, AudioFormat};
use super::output_backend::AudioOutputBackend;
use super::resampler::LinearResampler;
use super::ring_buffer::{AudioRing, RingProducer};

pub struct SystemDeviceBackend {
    name: String,
    device: cpal::Device,
    stream: Option<Stream>,
    producer: Option<RingProducer>,
    format: Option<AudioFormat>,
    /// Output ring capacity: 4× a 512-sample buffer × max 2 channels = 4096 samples.
    ring_capacity: usize,
    /// Scratch for upmix.
    scratch: Vec<f32>,
    /// Resamples the worker's source-rate (INTERNAL_SAMPLE_RATE) samples to the
    /// device's native rate. Created in `open` once the device rate is known.
    resampler: Option<LinearResampler>,
    /// Scratch buffer for resampled samples.
    rs_scratch: Vec<f32>,
}

// cpal::Stream on Windows contains a *mut () (WASAPI internals) which is !Send.
// SystemDeviceBackend is only accessed through a parking_lot::Mutex, so actual
// cross-thread access is serialised. The stream itself is never moved across threads
// after construction — the Mutex guarantees exclusive access.
unsafe impl Send for SystemDeviceBackend {}

impl SystemDeviceBackend {
    pub fn new(device: cpal::Device) -> Self {
        let name = device.name().unwrap_or_else(|_| "<unknown>".into());
        Self {
            name,
            device,
            stream: None,
            producer: None,
            format: None,
            ring_capacity: 4096,
            scratch: Vec::with_capacity(4096),
            resampler: None,
            rs_scratch: Vec::with_capacity(4096),
        }
    }
}

impl AudioOutputBackend for SystemDeviceBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn open(&mut self, format: AudioFormat) -> Result<()> {
        let supported = self
            .device
            .default_output_config()
            .context("getting default output config")?;
        let device_rate = supported.sample_rate().0;
        let config: StreamConfig = supported.config();
        let channels = config.channels;

        let (producer, mut consumer) = AudioRing::new(self.ring_capacity);

        let err_fn = |e| log::error!("output stream error: {e}");

        let stream = match supported.sample_format() {
            SampleFormat::F32 => self.device.build_output_stream(
                &config,
                move |out: &mut [f32], _| {
                    let n = consumer.pop(out);
                    if n < out.len() {
                        for s in &mut out[n..] {
                            *s = 0.0;
                        }
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => {
                let mut tmp: Vec<f32> = vec![0.0; 8192];
                self.device.build_output_stream(
                    &config,
                    move |out: &mut [i16], _| {
                        if tmp.len() < out.len() {
                            tmp.resize(out.len(), 0.0);
                        }
                        let n = consumer.pop(&mut tmp[..out.len()]);
                        let mut i16_buf = Vec::with_capacity(out.len());
                        f32_to_i16(&tmp[..n], &mut i16_buf);
                        for (i, s) in i16_buf.iter().enumerate() {
                            out[i] = *s;
                        }
                        for s in &mut out[n..] {
                            *s = 0;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(anyhow!("unsupported output sample format: {:?}", other));
            }
        }
        .context("building output stream")?;

        stream.play().context("starting output stream")?;

        self.stream = Some(stream);
        self.producer = Some(producer);
        self.format = Some(AudioFormat {
            sample_rate: config.sample_rate.0,
            channels,
        });
        self.resampler = Some(LinearResampler::new(format.sample_rate, device_rate));
        Ok(())
    }

    fn write(&mut self, samples: &[f32]) -> Result<usize> {
        let fmt = self
            .format
            .ok_or_else(|| anyhow!("backend not opened"))?;

        // Resample source-rate (48k) → device rate if needed.
        let needs_resample = self
            .resampler
            .as_ref()
            .map(|r| !r.is_identity())
            .unwrap_or(false);
        if needs_resample {
            let r = self.resampler.as_mut().unwrap();
            r.process(samples, &mut self.rs_scratch);
        }
        let resampled: &[f32] = if needs_resample {
            &self.rs_scratch
        } else {
            samples
        };

        let producer = self
            .producer
            .as_mut()
            .ok_or_else(|| anyhow!("backend not opened"))?;
        if fmt.channels <= 1 {
            return Ok(producer.push(resampled));
        }
        mono_to_interleaved(resampled, fmt.channels, &mut self.scratch);
        Ok(producer.push(&self.scratch))
    }

    fn close(&mut self) {
        if let Some(s) = self.stream.take() {
            let _ = s.pause();
        }
        self.producer = None;
        self.format = None;
    }
}
