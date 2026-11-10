# Audio assets

- `candidates/` — the built-in sample bank used by the in-app sample
  selector. All Freesound entries are license "Creative Commons 0"; each
  file is a processed high-quality CDN preview (mono 44.1 kHz Vorbis q5,
  silence-trimmed, peak-normalized, accent = 1.25x pitch-up). Per-file
  sources, titles and authors are listed in `candidates/MANIFEST.md`.
  Generated `gen_*` clicks are synthesized by `scripts/build_candidate_bank.py`
  (no external license). Before promoting a kept sample to a permanent
  preset, re-export it from the original Freesound file.
