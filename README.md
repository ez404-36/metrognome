# Metrognome

Metrognome is a native GNOME-style metronome focused on precise timing,
adjustable volume, accent patterns, and customizable sounds.

## Current implementation status

- Sample-clock audio scheduler.
- Continuous `cpal` output stream.
- BPM 20–300.
- 2/4, 3/4, 4/4, and 6/8 patterns.
- Accent pattern support.
- Master volume.
- Built-in Classic, Woodblock, Digital, and Soft sounds; the Classic preset uses
  embedded CC0 OGG Vorbis samples (Kenney "Interface Sounds", see
  `assets/audio/LICENSE.md`).
- 16-bit PCM and floating-point WAV import, including channel downmix,
  resampling, and peak normalization.
- Tap tempo.
- libadwaita UI skeleton.
- Sound preset menu and WAV import actions.
- Clickable beat indicators for custom accent patterns.
- Settings persistence.
- Unit tests for pattern handling, settings serialization, and high-tempo scheduling.

## Build

The default build runs the audio-core smoke test:

```sh
cargo test
cargo run
```

CLI smoke-test options:

```sh
cargo run -- --preset woodblock
cargo run -- --normal ./normal.wav --accent ./accent.wav
```

To build and run the GTK UI, first install system development packages:

```sh
./scripts/install-deps.sh
```

Then run:

```sh
cargo run --features ui
```

On a GNOME/Wayland session, the binary should be launched from the desktop
session rather than through an SSH-only environment.
