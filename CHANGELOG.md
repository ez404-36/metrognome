# Changelog

All notable changes to Takto are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.7] - 2026-11-12

### Added

- Custom time signatures: any meter from 1/2 to 16/16. The meter row shows up to 4 chips; a pencil button opens a sizes editor where presets and custom meters can be shown or hidden, reordered by drag-and-drop, and custom meters can be added (beats 1–16, beat unit 2/4/8/16 with a live preview), edited in place, or removed. The active meter always keeps its chip, switching meters mid-playback keeps the tempo, and all choices persist across restarts. Old settings files load unchanged: a saved 6 beats resolves to 6/8, everything else to quarter notes. BPM still means the rate of every tick; the beat unit is the written label only.

## [0.1.6] - 2026-11-12

### Fixed

- The v0.1.5 Linux packages were built with a leftover temporary application id (`…Takto.Probe`): the app ran without its name and icon. Rebuilt with the correct `io.github.ez404_36.Takto` id, and the release notes now come from the matching CHANGELOG.md section instead of a placeholder.

## [0.1.5] - 2026-11-11

### Added

- Tempo presets: the traditional Italian tempo term for the current BPM (Grave through Prestissimo, modern non-overlapping ranges) is shown above the tempo slider and follows every tempo change, including tap tempo. Clicking the term opens a compact dropdown, drawn over the slider, listing every marking with its BPM range; the active one is checked, and selecting another sets the tempo to that range's midpoint.
- In-app updates: shortly after launch the app checks the latest published GitHub Release and shows a banner when a newer version exists. One click downloads the matching artifact and installs it — `.deb` installs go through `pkexec apt-get install`, AppImages are replaced in place with a restart prompt, and Flatpak/development builds open the release page instead. A manual "Check for updates" row was added to the settings modal.

## [0.1.4] - 2026-11-10

### Added

- Volume boost up to 200%: the slider goes past unity gain, values are displayed as a percentage, and the loudspeaker icon switches to the over-amplified glyph above 100%. A soft-knee limiter (bit-exact below 0.95) bends louder output smoothly toward full scale so boosted playback never hard-clips; ordinary 0–100% playback is unchanged. (#58af07e)

### Fixed

- Audio engine starts the first tick immediately on playback instead of waiting a full beat interval. (#2c2588a)
- Tempo adjusters (+/− buttons) stay stable around 100 BPM. (#18d5cdc)

### Removed

- The bundled "Classic" click sample: the sample selector now lists only the 18 candidate samples, saved selections that pointed at the bundled bank resolve to the first candidate, and Woodblock is the new default. (#9a68a43)
- Legacy settings migration code. (#2525376)

### Documentation

- READMEs (including localized copies) clarify that timing runs in the real-time audio callback — not in GUI timers or sleep calls — and a new framed app screenshot. (#15d7da2, #02756ea)

## [0.1.3] - 2026-11-10

### Added

- Pinned Flathub manifest for the Flatpak packaging. (#2d6d79b)

### Changed

- Renamed the application from Metrognome to Takto, including the application id, desktop entry, icons and packaging, and regenerated the app screenshot. (#cf45b68)

## [0.1.2] - 2026-11-09

### Added

- Flatpak packaging and a release workflow job that builds a self-contained `.flatpak` bundle on tagged releases. (#c203aba)

### Fixed

- Beat highlight is optically centered: the dot is drawn as a CSS circle child instead of a U+25CF font glyph sitting on the text baseline; muted/accent/playing states are unchanged. (#e3a86a2)

### CI / Packaging

- Release assets upload to the `uploads.github.com` endpoint, fixing asset publishing on tagged releases. (#980df5e)
- Release workflow installs `flatpak-builder` and registers the Flathub remote before building. (#2adbbc3, #3bb6d68)
- Flatpak runtime and SDK are pre-installed with `sudo` in the release job. (#dd35121)
- The Rust extension branch is resolved from SDK metadata instead of being hardcoded. (#77ed801)
- Flatpak directory sources are pinned to their named destinations in the manifest. (#307e297)

### Documentation

- Added a Flathub publication plan. (#63a253b)
- Added a drop shadow to the app screenshot. (#7ab07e2)

## [0.1.1] - 2026-11-09

### Fixed

- Desktop entry is installed as `io.github.metrognome.Metrognome.desktop`, matching the GTK application id, so GNOME Shell on Wayland matches the running window and shows the app icon in the dock instead of the fallback icon; deb and AppImage packaging updated accordingly. (#82066f7)

### Documentation

- Refreshed the app screenshot, aligned README feature highlights, moved the development guide into `DEVELOP.md`, and linked the latest release from the README. (#7e947fb, #d443172, #9d1c283, #69bb517)

## [0.1.0] - 2026-11-08

Initial release (under the original name Metrognome).

### Added

- Precise audio core built on cpal with sample-accurate voice scheduling inside the real-time audio callback, plus a GTK 4 / libadwaita desktop UI (20–300 BPM, accent patterns, tap tempo). (#571200a, #82d6933)
- Embedded CC0 OGG clicks for the built-in preset. (#6d1e49f)
- Selectable click samples and first-beat accent control. (#b7769e4)
- Audio and settings errors surfaced in the UI. (#39c0275)
- Interface language selector with persistent language choice. (#dcfacd4)
- Branding (logo and icons) and release packaging: GitHub Actions CI, automated tag-based releases, and `.deb` + portable AppImage artifacts. (#b1cd9c8)

### Fixed

- Synthetic click noise removed and round tap forced. (#113d405)
- Build against GTK 4.14 and use the libadwaita toolbar layout; tightened desktop layout and centered playback controls. (#22dc3e0, #df84a3f, #d03ec7e, #b787feb)

[Unreleased]: https://github.com/ez404-36/takto/compare/v0.1.7...HEAD
[0.1.7]: https://github.com/ez404-36/takto/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/ez404-36/takto/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/ez404-36/takto/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/ez404-36/takto/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/ez404-36/takto/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/ez404-36/takto/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/ez404-36/takto/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ez404-36/takto/compare/571200a...v0.1.0
