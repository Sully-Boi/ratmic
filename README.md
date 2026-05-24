<div align="center">
  <img src="assets/logo.png" alt="RatMic logo" width="128" height="128" />
  <h1>RatMic</h1>
  <p><strong>Make your microphone sound gloriously, intentionally terrible — in real time.</strong></p>
</div>

RatMic is a Windows desktop app that captures your real microphone, runs it through a chain of cursed audio effects (bitcrushing, packet loss, distortion, hum, broken gate, and more), and feeds the mangled result into Discord, games, OBS, or anything else that takes a mic input.

It is **not** a voice changer. It's a *bad-mic simulator* — for pranks, bits, character voices, or just sounding like you're calling in from a 2007 Skype session in a wind tunnel.

> ⚠️ RatMic intentionally degrades audio, but it will **not** blast your listeners. A fixed limiter sits permanently at the end of the chain and caps output at a safe level.

---

## How it works

```
Real Mic  →  RatMic (effect chain)  →  Virtual Audio Cable  →  Discord / Game / OBS
```

RatMic sends its processed audio to a **virtual audio cable** (a free driver you install separately). You then select that cable as your "microphone" in Discord and friends — so they hear the processed version, while RatMic does the mangling in between.

---

## Requirements

- **Windows 10 or 11**
- A **virtual audio cable** — RatMic does not ship one. Any of these work:
  - [VB-CABLE](https://vb-audio.com/Cable/) (free, simplest — recommended)
  - [VoiceMeeter](https://vb-audio.com/Voicemeeter/) (also from VB-Audio)
  - Any device whose name contains *Cable*, *Virtual*, *Voicemeeter*, or *BlackHole*

---

## Install

1. Install [VB-CABLE](https://vb-audio.com/Cable/) (download, unzip, run the installer as admin, reboot if asked).
2. Download the latest **RatMic installer** from the [Releases page](https://github.com/Sully-Boi/ratmic/releases) (`RatMic_x.x.x_x64-setup.exe` or `.msi`).
3. Run it.

   > 🛡️ The first time you run an unsigned app, Windows SmartScreen may say *"Windows protected your PC."* Click **More info → Run anyway**. (RatMic isn't code-signed yet — signing requires a paid certificate.)

---

## Setup (one time, ~1 minute)

**In RatMic:**
- **Input** → your real microphone
- **Output** → `CABLE Input (VB-Audio Virtual Cable)`

**In Discord (or your game / OBS):**
- Set the **input/microphone** device to `CABLE Output (VB-Audio Virtual Cable)`

That's it. The routing-health dot in RatMic's title bar turns **green** when it detects you're pointed at a virtual cable.

---

## Using it

1. Pick a preset from the left sidebar (try **Xbox 360 Lobby** or **Drive-Thru Speaker**).
2. Hit **▶ START**.
3. Talk. Your victims now hear the cursed version.
4. Want to hear it yourself first? Pick a **Monitor** device (your headphones) and flip **▶ Listen** — you'll hear exactly what they hear, in real time. *(Don't set Monitor to your mic — that causes feedback; RatMic warns you if you do.)*

Tweak any effect by clicking it in the chain, toggle effects on/off with the pill switches, or build your own and save it as a preset.

### Built-in presets

Xbox 360 Lobby · Cheap Headset · Drive-Thru Speaker · Broken Radio · Discord Packet Loss · Deep Fried Mic · Tin Can · Underwater · Fan Noise Hell · 2007 Skype Call

### Effects

Gain · Bandpass/Telephone EQ · Bitcrusher · Clipper/Distortion · Noise (white/hum/crackle) · Packet Loss (with stutter) · Bad Noise Gate · Limiter (fixed, always-on safety)

---

## Building from source

You'll need [Node.js](https://nodejs.org) 20+ and the [Rust toolchain](https://rustup.rs) (stable, MSVC). Building on Windows also needs the **Visual Studio Build Tools** with the *Desktop development with C++* workload.

```bash
# Clone
git clone https://github.com/Sully-Boi/ratmic.git
cd ratmic

# Install JS dependencies
npm install

# Run in dev mode (hot-reload)
npm run tauri dev

# Produce installers (.msi + .exe) in src-tauri/target/release/bundle/
npm run tauri build
```

### Tech stack

- **[Tauri 2](https://tauri.app)** — desktop shell (Rust backend, web frontend)
- **[Svelte 4](https://svelte.dev)** + TypeScript — UI
- **[cpal](https://github.com/RustAudio/cpal)** — real-time audio I/O (WASAPI)
- Hand-rolled DSP for every effect

The audio engine runs on a dedicated worker thread, decoupled from the UI via lock-free ring buffers, targeting ≤30 ms round-trip latency.

---

## Troubleshooting

- **No sound in Discord** — make sure Discord's input is `CABLE Output`, not `CABLE Input`. Input and Output are opposite ends of the same cable.
- **The routing dot is amber** — your selected Output looks like real speakers, not a cable. Discord won't hear RatMic. Re-pick `CABLE Input` as the Output.
- **Robotic echo / doubled audio** — your mic and the virtual cable may be running at different sample rates. Set both to 48000 Hz in Windows Sound settings.
- **App won't launch after building** — confirm the Visual Studio C++ Build Tools are installed (the MSVC linker is required).

---

## License

Personal project — no license granted yet. Ask before redistributing.
