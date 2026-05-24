//! Top-level audio engine: owns input stream, worker thread, output backend, effect chain.

use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::effects::chain::EffectChain;

use super::devices::{find_input_device, find_output_device};
use super::format::AudioFormat;
use super::input_stream::InputStream;
use super::meters::{Meter, MeterValue};
use super::output_backend::AudioOutputBackend;
use super::ring_buffer::{AudioRing, RingConsumer};
use super::system_output::SystemDeviceBackend;

pub const INTERNAL_SAMPLE_RATE: u32 = 48_000;
const INPUT_RING_CAPACITY: usize = 8192;
const WORKER_CHUNK_SAMPLES: usize = 480;
const METER_TICK_MS: u64 = 16;

#[derive(Debug, Clone, Copy)]
pub struct MeterSnapshot {
    pub input: MeterValue,
    pub output: MeterValue,
    pub limiter_activity_pct: f32,
}

pub trait MeterSink: Send + 'static {
    fn push(&self, snap: MeterSnapshot);
}

pub struct AudioEngine {
    _input: InputStream,
    worker_handle: Option<thread::JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    backend: Arc<Mutex<Box<dyn AudioOutputBackend>>>,
    monitor_slot: Arc<Mutex<Option<Box<dyn AudioOutputBackend>>>>,
    monitor_enabled: Arc<AtomicBool>,
    pub chain: Arc<Mutex<EffectChain>>,
}

// cpal::Stream on Windows/WASAPI contains a PhantomData<*mut ()> that makes it
// !Send. AudioEngine never actually sends the Stream across threads — it only
// creates and drops it on the same logical owner. The worker thread receives
// only Arc/AtomicBool handles. Safe to assert Send + Sync here.
unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

impl AudioEngine {
    pub fn start<S: MeterSink + 'static>(
        input_id: &str,
        output_id: &str,
        monitor_device_id: Option<&str>,
        monitor_enabled_initial: bool,
        effects_enabled: Arc<AtomicBool>,
        meter_sink: S,
    ) -> Result<Self> {
        if input_id == output_id {
            return Err(anyhow!("input and output device must differ"));
        }
        let input_device = find_input_device(input_id)
            .with_context(|| format!("opening input device {input_id}"))?;
        let output_device = find_output_device(output_id)
            .with_context(|| format!("opening output device {output_id}"))?;

        let (in_prod, in_cons) = AudioRing::new(INPUT_RING_CAPACITY);
        let input_stream = InputStream::open(&input_device, in_prod, INTERNAL_SAMPLE_RATE)
            .context("opening input stream")?;

        let mut backend = SystemDeviceBackend::new(output_device);
        backend
            .open(AudioFormat {
                sample_rate: INTERNAL_SAMPLE_RATE,
                channels: 1,
            })
            .context("opening output backend")?;
        let backend: Arc<Mutex<Box<dyn AudioOutputBackend>>> =
            Arc::new(Mutex::new(Box::new(backend)));

        // Monitor backend lives in a hot-swappable slot so the device can be
        // changed — or first selected — while the engine is already running.
        let monitor_slot: Arc<Mutex<Option<Box<dyn AudioOutputBackend>>>> =
            Arc::new(Mutex::new(None));
        if let Some(mon_id) = monitor_device_id {
            match open_monitor_backend(mon_id) {
                Ok(b) => {
                    log::info!("monitor backend opened: {mon_id}");
                    *monitor_slot.lock() = Some(b);
                }
                Err(e) => log::warn!("could not open monitor backend '{mon_id}': {e}"),
            }
        }

        let monitor_enabled = Arc::new(AtomicBool::new(monitor_enabled_initial));

        // Default chain on startup: empty (just the auto-appended Limiter).
        // A preset load — or manual Add Effect — populates it.
        let mut chain = EffectChain::new(INTERNAL_SAMPLE_RATE);
        chain.rebuild_from_slots(INTERNAL_SAMPLE_RATE, vec![]);
        let chain = Arc::new(Mutex::new(chain));

        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let stop = stop.clone();
            let backend = backend.clone();
            let monitor_slot = monitor_slot.clone();
            let monitor_enabled = monitor_enabled.clone();
            let chain = chain.clone();
            let effects_enabled = effects_enabled.clone();
            thread::Builder::new()
                .name("ratmic-audio-worker".into())
                .spawn(move || {
                    worker_loop(in_cons, backend, monitor_slot, monitor_enabled, chain, effects_enabled, meter_sink, stop);
                })
                .context("spawning audio worker")?
        };

        log::info!("audio engine started with chain ({} slots)", chain.lock().len());

        Ok(Self {
            _input: input_stream,
            worker_handle: Some(worker),
            stop_flag: stop,
            backend,
            monitor_slot,
            monitor_enabled,
            chain,
        })
    }

    pub fn stop(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        self.backend.lock().close();
        if let Some(mut mon) = self.monitor_slot.lock().take() {
            mon.close();
        }
        log::info!("audio engine stopped");
    }

    /// Enable or disable the monitor listen-back in real time (no restart needed).
    pub fn set_monitor_enabled(&self, enabled: bool) {
        self.monitor_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Open, change, or close the monitor backend on a running engine.
    /// Pass `None` to close it. The new device is opened *before* the slot lock
    /// is taken, so a failure leaves the existing monitor untouched.
    pub fn set_monitor_device(&self, device_id: Option<&str>) -> Result<()> {
        let new_backend = match device_id {
            Some(id) => Some(open_monitor_backend(id)?),
            None => None,
        };
        let mut slot = self.monitor_slot.lock();
        if let Some(mut old) = slot.take() {
            old.close();
        }
        *slot = new_backend;
        Ok(())
    }

    /// Returns true if a monitor backend is currently open.
    pub fn has_monitor_backend(&self) -> bool {
        self.monitor_slot.lock().is_some()
    }

    /// Atomically replace the chain (used by preset load).
    pub fn replace_chain(
        &self,
        effect_specs: Vec<(String, bool, serde_json::Value)>,
    ) -> anyhow::Result<()> {
        let mut new_slots: Vec<(Box<dyn crate::effects::Effect>, bool)> = Vec::new();
        for (type_name, enabled, params) in effect_specs {
            match crate::effects::registry::make_effect(&type_name, &params, INTERNAL_SAMPLE_RATE) {
                Ok(e) => new_slots.push((e, enabled)),
                Err(err) => log::warn!("skipping unknown effect '{}': {}", type_name, err),
            }
        }
        let mut guard = self.chain.lock();
        guard.rebuild_from_slots(INTERNAL_SAMPLE_RATE, new_slots);
        Ok(())
    }

    /// Add a fresh effect of the given type to the chain (inserted before limiter).
    pub fn add_effect(&self, type_name: &str, enabled: bool) -> anyhow::Result<()> {
        let effect = crate::effects::registry::make_effect(
            type_name,
            &serde_json::json!({}),
            INTERNAL_SAMPLE_RATE,
        )?;
        let mut guard = self.chain.lock();
        guard.insert_before_limiter(effect, enabled);
        Ok(())
    }

    /// Remove the slot at the given index. Returns true on success, false if
    /// the index is the Limiter or out of range.
    pub fn remove_effect(&self, index: usize) -> bool {
        let mut guard = self.chain.lock();
        guard.remove(index)
    }

    /// Reorder a non-limiter slot from `from` to `to`. Returns true on success.
    pub fn reorder_effects(&self, from: usize, to: usize) -> bool {
        let mut guard = self.chain.lock();
        guard.move_slot(from, to)
    }
}

fn worker_loop<S: MeterSink>(
    mut consumer: RingConsumer,
    backend: Arc<Mutex<Box<dyn AudioOutputBackend>>>,
    monitor_slot: Arc<Mutex<Option<Box<dyn AudioOutputBackend>>>>,
    monitor_enabled: Arc<AtomicBool>,
    chain: Arc<Mutex<EffectChain>>,
    effects_enabled: Arc<AtomicBool>,
    sink: S,
    stop: Arc<AtomicBool>,
) {
    let mut buffer = vec![0.0_f32; WORKER_CHUNK_SAMPLES];
    let mut in_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let mut out_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let meter_interval = Duration::from_millis(METER_TICK_MS);
    let mut last_meter = std::time::Instant::now();

    // Track limiter activity over the last ~500 ms (50 chunks of 10 ms each).
    const ACTIVITY_HISTORY: usize = 50;
    let mut activity_history = [false; ACTIVITY_HISTORY];
    let mut activity_write = 0_usize;

    while !stop.load(Ordering::Relaxed) {
        let n = consumer.pop(&mut buffer);
        if n == 0 {
            thread::sleep(Duration::from_millis(2));
            continue;
        }
        let chunk = &mut buffer[..n];
        in_meter.process(chunk);

        if effects_enabled.load(Ordering::Relaxed) {
            let mut chain_guard = chain.lock();
            chain_guard.process(chunk);
            let was_active = chain_guard.limiter_was_active();
            drop(chain_guard);
            activity_history[activity_write] = was_active;
        } else {
            // Bypassed: clean passthrough, no limiter activity.
            activity_history[activity_write] = false;
        }
        activity_write = (activity_write + 1) % ACTIVITY_HISTORY;

        out_meter.process(chunk);
        let _ = backend.lock().write(chunk);
        if monitor_enabled.load(Ordering::Relaxed) {
            if let Some(mon) = monitor_slot.lock().as_mut() {
                let _ = mon.write(chunk);
            }
        }

        if last_meter.elapsed() >= meter_interval {
            let activity_count = activity_history.iter().filter(|x| **x).count();
            let limiter_activity_pct =
                (activity_count as f32) / (ACTIVITY_HISTORY as f32) * 100.0;

            sink.push(MeterSnapshot {
                input: in_meter.snapshot(),
                output: out_meter.snapshot(),
                limiter_activity_pct,
            });
            last_meter = std::time::Instant::now();
        }
    }
}

/// Open a `SystemDeviceBackend` for the given output device id at the internal
/// sample rate. Used both at startup and for live monitor-device changes.
fn open_monitor_backend(device_id: &str) -> Result<Box<dyn AudioOutputBackend>> {
    let device = find_output_device(device_id)
        .with_context(|| format!("monitor device '{device_id}' not found"))?;
    let mut backend = SystemDeviceBackend::new(device);
    backend
        .open(AudioFormat {
            sample_rate: INTERNAL_SAMPLE_RATE,
            channels: 1,
        })
        .with_context(|| format!("opening monitor backend '{device_id}'"))?;
    Ok(Box::new(backend))
}
