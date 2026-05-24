//! cpal-based input stream that drains samples into an output `RingProducer`.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use super::format::{downmix_to_mono, i16_to_f32, AudioFormat};
use super::resampler::LinearResampler;
use super::ring_buffer::RingProducer;

pub struct InputStream {
    _stream: Stream,
    pub device_format: AudioFormat,
}

impl InputStream {
    pub fn open(
        device: &cpal::Device,
        mut out_producer: RingProducer,
        target_rate: u32,
    ) -> Result<Self> {
        let supported = device
            .default_input_config()
            .context("getting default input config")?;
        let input_rate = supported.sample_rate().0;
        let device_format = AudioFormat {
            sample_rate: input_rate,
            channels: supported.channels(),
        };
        let config: StreamConfig = supported.config();
        let channels = config.channels;

        let err_fn = |e| log::error!("input stream error: {e}");

        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                let mut scratch_mono: Vec<f32> = Vec::with_capacity(2048);
                let mut scratch_rs: Vec<f32> = Vec::with_capacity(2048);
                let mut resampler = LinearResampler::new(input_rate, target_rate);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        downmix_to_mono(data, channels, &mut scratch_mono);
                        resampler.process(&scratch_mono, &mut scratch_rs);
                        let _ = out_producer.push(&scratch_rs);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I16 => {
                let mut scratch_f32: Vec<f32> = Vec::with_capacity(2048);
                let mut scratch_mono: Vec<f32> = Vec::with_capacity(2048);
                let mut scratch_rs: Vec<f32> = Vec::with_capacity(2048);
                let mut resampler = LinearResampler::new(input_rate, target_rate);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        i16_to_f32(data, &mut scratch_f32);
                        downmix_to_mono(&scratch_f32, channels, &mut scratch_mono);
                        resampler.process(&scratch_mono, &mut scratch_rs);
                        let _ = out_producer.push(&scratch_rs);
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(anyhow!("unsupported input sample format: {:?}", other));
            }
        }
        .context("building input stream")?;

        stream.play().context("starting input stream")?;

        Ok(Self {
            _stream: stream,
            device_format,
        })
    }
}
