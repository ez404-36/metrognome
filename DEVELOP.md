# Development guide

This document contains the setup, build, test, and release workflow for Metrognome.

## Prerequisites

Install the GTK development dependencies on Debian or Ubuntu:

```sh
./scripts/install-deps.sh
```

## Build and run

Build and run the desktop UI:

```sh
cargo run --features ui
```

On GNOME or Wayland, launch the application from the desktop session rather than a plain SSH environment.

## Test

Run the audio-core tests:

```sh
cargo test
```

CLI smoke tests are useful without a desktop session:

```sh
cargo run -- --preset woodblock
cargo run -- --normal ./normal.wav --accent ./accent.wav
```

## Release process

Pushing a `v<version>` tag that matches the version in `Cargo.toml` builds the Linux artifacts and publishes a GitHub Release with them.

| Artifact | Description |
|---|---|
| `metrognome_<version>-1_amd64.deb` | Debian package built with `cargo-deb`; runtime dependencies are derived from the linked libraries, so it targets the Ubuntu 24.04-era GNOME stack it was built against. |
| `metrognome_<version>_x86_64.AppImage` | Portable AppImage that bundles GTK 4, libadwaita, and ALSA; requires a host glibc at least as new as the build runner's (Ubuntu 24.04). |

The pipeline lives in `.github/workflows/`:

- [`ci.yml`](.github/workflows/ci.yml) — fmt, clippy (`ui` feature), tests, and a GUI build on every PR and push to `master`.
- [`release.yml`](.github/workflows/release.yml) — checks the tag against `Cargo.toml`, builds the `.deb` and `.AppImage`, attaches them to a draft Release, and publishes it once everything landed. A manual `workflow_dispatch` run builds the artifacts without creating a Release.
- [`auto-tag-release.yml`](.github/workflows/auto-tag-release.yml) — pushes the `v<version>` tag automatically when the version is bumped in `Cargo.toml` on `master`. For fully automatic tagging, add a `RELEASE_TAG_PAT` repository secret (a PAT with `Contents: read and write`); without it, tags are still pushed but cannot trigger `release.yml` (GitHub's workflow-triggering-workflow restriction), so re-run `release.yml` manually or re-push the tag from a local clone.
