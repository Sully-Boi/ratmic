//! Global hotkey registration. The OS hotkey manager lives entirely inside a
//! listener thread (so it never crosses the Tauri state Send/Sync boundary).
//! The thread flips the shared `effects_enabled` flag and emits `effects-state`.

use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tauri::{AppHandle, Emitter};

use crate::events::{EffectsStateEvent, EVENT_EFFECTS_STATE};
use crate::settings::{HotkeyConfig, HotkeyMode};

/// Handle stored in AppState. Holds only Send+Sync primitives.
pub struct HotkeyHandle {
    stop: Arc<AtomicBool>,
    listener: Option<thread::JoinHandle<()>>,
}

fn build_hotkey(cfg: &HotkeyConfig) -> Result<HotKey> {
    let code = Code::from_str(&cfg.code)
        .map_err(|_| anyhow!("unrecognized key code: {}", cfg.code))?;
    let mut mods = Modifiers::empty();
    if cfg.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if cfg.alt {
        mods |= Modifiers::ALT;
    }
    if cfg.shift {
        mods |= Modifiers::SHIFT;
    }
    let mods = if mods.is_empty() { None } else { Some(mods) };
    Ok(HotKey::new(mods, code))
}

/// Apply a single hotkey event: flip/set the effects flag per mode and emit the
/// effects-state event so the UI reflects it.
fn handle_hotkey_event(
    event: GlobalHotKeyEvent,
    hotkey_id: u32,
    mode: HotkeyMode,
    effects_enabled: &Arc<AtomicBool>,
    app: &AppHandle,
) {
    if event.id != hotkey_id {
        return;
    }
    let new_state = match mode {
        HotkeyMode::Toggle => {
            if matches!(event.state, HotKeyState::Pressed) {
                Some(!effects_enabled.load(Ordering::Relaxed))
            } else {
                None
            }
        }
        HotkeyMode::Hold => match event.state {
            HotKeyState::Pressed => Some(true),
            HotKeyState::Released => Some(false),
        },
    };
    if let Some(enabled) = new_state {
        effects_enabled.store(enabled, Ordering::Relaxed);
        let _ = app.emit(EVENT_EFFECTS_STATE, EffectsStateEvent { enabled });
    }
}

impl HotkeyHandle {
    pub fn register(
        cfg: &HotkeyConfig,
        effects_enabled: Arc<AtomicBool>,
        app: AppHandle,
    ) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (setup_tx, setup_rx) = mpsc::channel::<Result<(), String>>();
        let cfg = cfg.clone();
        let mode = cfg.mode;

        let listener = {
            let stop = stop.clone();
            thread::Builder::new()
                .name("ratmic-hotkey-listener".into())
                .spawn(move || {
                    // Create + register the manager ON this thread.
                    let manager = match GlobalHotKeyManager::new() {
                        Ok(m) => m,
                        Err(e) => {
                            let _ = setup_tx.send(Err(format!("hotkey init failed: {e}")));
                            return;
                        }
                    };
                    let hotkey = match build_hotkey(&cfg) {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = setup_tx.send(Err(e.to_string()));
                            return;
                        }
                    };
                    if let Err(e) = manager.register(hotkey) {
                        let _ = setup_tx.send(Err(format!("hotkey register failed: {e}")));
                        return;
                    }
                    let hotkey_id = hotkey.id();
                    let _ = setup_tx.send(Ok(()));

                    let rx = GlobalHotKeyEvent::receiver();

                    // On Windows, global-hotkey delivers WM_HOTKEY to this thread's
                    // message queue, so we MUST run a win32 message pump here for
                    // events to arrive (per the crate's docs). PeekMessage is
                    // non-blocking so we can also poll the stop flag + event channel.
                    #[cfg(windows)]
                    {
                        use windows_sys::Win32::UI::WindowsAndMessaging::{
                            DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
                        };
                        let mut msg: MSG = unsafe { std::mem::zeroed() };
                        while !stop.load(Ordering::Relaxed) {
                            unsafe {
                                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                                    let _ = TranslateMessage(&msg);
                                    DispatchMessageW(&msg);
                                }
                            }
                            while let Ok(event) = rx.try_recv() {
                                handle_hotkey_event(event, hotkey_id, mode, &effects_enabled, &app);
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    }

                    #[cfg(not(windows))]
                    {
                        while !stop.load(Ordering::Relaxed) {
                            if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                                handle_hotkey_event(event, hotkey_id, mode, &effects_enabled, &app);
                            }
                        }
                    }
                    // `manager` drops here, releasing the OS hotkey.
                })
                .map_err(|e| anyhow!("spawning hotkey listener: {e}"))?
        };

        // Wait for the thread's setup result.
        match setup_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                stop,
                listener: Some(listener),
            }),
            Ok(Err(e)) => {
                let _ = listener.join();
                Err(anyhow!(e))
            }
            Err(_) => Err(anyhow!("hotkey listener thread exited during setup")),
        }
    }

    pub fn unregister(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.listener.take() {
            let _ = h.join();
        }
    }
}
