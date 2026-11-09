# Development guide

This document contains the setup, build, test, and release workflow for Takto.

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
| `takto_<version>-1_amd64.deb` | Debian package built with `cargo-deb`; runtime dependencies are derived from the linked libraries, so it targets the Ubuntu 24.04-era GNOME stack it was built against. |
| `takto_<version>_x86_64.AppImage` | Portable AppImage that bundles GTK 4, libadwaita, and ALSA; requires a host glibc at least as new as the build runner's (Ubuntu 24.04). |
| `takto_<version>_x86_64.flatpak` | Self-contained Flatpak bundle built from [`packaging/flatpak/io.github.ez404_36.Takto.yml`](packaging/flatpak/io.github.ez404_36.Takto.yml); its GNOME runtime carries GTK 4, libadwaita, ALSA, and glibc, so it runs on any distro with flatpak. |

The pipeline lives in `.github/workflows/`:

- [`ci.yml`](.github/workflows/ci.yml) — fmt, clippy (`ui` feature), tests, and a GUI build on every PR and push to `master`.
- `packaging-check` (in [`ci.yml`](.github/workflows/ci.yml)) — validates the Flatpak manifest, AppStream metadata, desktop entry, and that [`packaging/flatpak/cargo-sources.json`](packaging/flatpak/cargo-sources.json) is in sync with `Cargo.lock`.
- [`release.yml`](.github/workflows/release.yml) — checks the tag against `Cargo.toml`, builds the `.deb` and `.AppImage`, attaches them to a draft Release, and publishes it once everything landed. A manual `workflow_dispatch` run builds the artifacts without creating a Release.
- [`auto-tag-release.yml`](.github/workflows/auto-tag-release.yml) — pushes the `v<version>` tag automatically when the version is bumped in `Cargo.toml` on `master`. For fully automatic tagging, add a `RELEASE_TAG_PAT` repository secret (a PAT with `Contents: read and write`); without it, tags are still pushed but cannot trigger `release.yml` (GitHub's workflow-triggering-workflow restriction), so re-run `release.yml` manually or re-push the tag from a local clone.

### Flatpak notes

- **Vendored dependencies.** After changing `Cargo.lock`, regenerate the
  Flatpak source list — CI fails if you forget:

  ```sh
  ./scripts/generate-cargo-sources.py
  ```

- **Runtime version.** The manifest targets `org.gnome.Platform 50`. Flathub
  supports the current runtime and its predecessor; bump `runtime-version`
  when GNOME 52 ships (and re-test locally), otherwise the runtime goes
  end-of-life and users get update warnings.
- **Local build.** The manifest uses `dir` sources relative to its own path,
  so it builds straight from a working tree:

  ```sh
  flatpak-builder --force-clean --user --install \
    build packaging/flatpak/io.github.ez404_36.Takto.yml
  flatpak run io.github.ez404_36.Takto
  ```

  First run downloads the GNOME 50 platform/SDK and the Rust SDK extension
  (~2 GB from Flathub); later builds are incremental.
