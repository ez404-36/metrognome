#!/usr/bin/env python3
"""Fetch CC0 metronome click candidates from Freesound, synthesize a few
additional clicks, normalize everything to short mono OGG files, and emit
the Rust asset table + manifest used by the in-app sample selector.

Requires: ffmpeg/ffprobe on PATH. Network access needed for Freesound.
"""

from __future__ import annotations

import html
import math
import re
import struct
import subprocess
import sys
import time
import urllib.parse
import urllib.request
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets" / "audio" / "candidates"
RUST_PATH = ROOT / "src" / "audio" / "candidate_assets.rs"
MANIFEST_PATH = OUT_DIR / "MANIFEST.md"
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 takto-candidate-fetch"
TARGET_PEAK_DB = -1.2
ACCENT_PITCH_FACTOR = 1.25

# query -> (category label, quota)
QUERIES = [
    ("woodblock", "Woodblock", 5),
    ("rimshot", "Rimshot", 3),
    ("sidestick", "Sidestick", 2),
    ("cowbell", "Cowbell", 2),
    ("metronome click", "Metro Click", 3),
]
FILLER_QUERIES = ["percussion click", "wood hit", "stick click"]
TARGET_COUNT = 20


def http_get(url: str, attempts: int = 3) -> bytes:
    for attempt in range(attempts):
        request = urllib.request.Request(url, headers={"User-Agent": UA})
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code == 429 and attempt + 1 < attempts:
                delay = 20 * (attempt + 1)
                print(f"  rate limited, retrying in {delay}s…", file=sys.stderr)
                time.sleep(delay)
                continue
            raise
    raise RuntimeError(f"unreachable: {url}")


def scrape_query(query: str) -> list[dict]:
    url = (
        "https://freesound.org/search/?q="
        + urllib.parse.quote(query)
        + "&f="
        + urllib.parse.quote('license:"Creative Commons 0"')
    )
    page = http_get(url).decode("utf-8", "replace")
    entries = []
    for chunk in page.split('class="bw-search__result"')[1:]:
        player = chunk[:6000]

        def field(name: str) -> str | None:
            match = re.search(rf'data-{name}="([^"]*)"', player)
            return html.unescape(match.group(1)) if match else None

        sound_id = field("sound-id")
        ogg = field("ogg")
        title = field("title") or ""
        duration = float(field("duration") or 0)
        username = field("username") or "unknown"
        if not sound_id or not ogg or duration < 0.04 or duration > 4.5:
            continue
        entries.append(
            {
                "id": sound_id,
                "title": title,
                "username": username,
                "duration": duration,
                "ogg_url": ogg,
                "loop_like": bool(re.search(r"loop|loopable", title, re.I)),
            }
        )
    return entries


def pick_candidates() -> list[dict]:
    seen: set[str] = set()
    picked: list[dict] = []

    def take(entry: dict, category: str) -> bool:
        if entry["id"] in seen:
            return False
        seen.add(entry["id"])
        entry["category"] = category
        picked.append(entry)
        return True

    for query, category, quota in QUERIES:
        time.sleep(2)
        entries = sorted(
            scrape_query(query), key=lambda e: (e["loop_like"], e["duration"])
        )
        for entry in entries[:quota]:
            take(entry, category)

    for query in FILLER_QUERIES:
        if len(picked) >= TARGET_COUNT:
            break
        time.sleep(2)
        entries = sorted(
            scrape_query(query), key=lambda e: (e["loop_like"], e["duration"])
        )
        for entry in entries:
            if len(picked) >= TARGET_COUNT:
                break
            take(entry, "Click")

    return picked[:TARGET_COUNT]


def download_preview(entry: dict, dest: Path) -> bool:
    candidates = [
        entry["ogg_url"].replace("-lq.ogg", "-hq.ogg"),
        entry["ogg_url"],
        entry["ogg_url"].replace("-lq.ogg", "-lq.mp3"),
    ]
    for url in candidates:
        try:
            data = http_get(url)
        except Exception as error:  # noqa: BLE001
            print(f"  download failed {url}: {error}", file=sys.stderr)
            continue
        if len(data) > 800:
            dest.write_bytes(data)
            return True
    return False


def run(command: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(command, capture_output=True, text=True)


def ffmpeg_peak_db(wav_path: Path) -> float | None:
    result = run(
        [
            "ffmpeg",
            "-hide_banner",
            "-i",
            str(wav_path),
            "-af",
            "astats=metadata=0:reset=0",
            "-f",
            "null",
            "-",
        ]
    )
    matches = re.findall(r"Peak level dB:\s*(-?[\d.]+)", result.stderr)
    return float(matches[-1]) if matches else None


def process_sound(raw: Path, normal_out: Path, accent_out: Path) -> bool:
    trimmed = raw.with_suffix(".trimmed.wav")
    trim_filters = (
        "silenceremove=start_periods=1:start_threshold=-50dB,"
        "areverse,"
        "silenceremove=start_periods=1:start_threshold=-60dB:start_silence=0.05,"
        "areverse,"
        "atrim=0:0.8"
    )
    result = run(
        [
            "ffmpeg",
            "-hide_banner",
            "-y",
            "-i",
            str(raw),
            "-af",
            trim_filters,
            "-ac",
            "1",
            "-ar",
            "44100",
            str(trimmed),
        ]
    )
    if result.returncode != 0:
        print(f"  trim failed: {result.stderr[-300:]}", file=sys.stderr)
        return False

    peak = ffmpeg_peak_db(trimmed)
    gain_db = 0.0 if peak is None else round(TARGET_PEAK_DB - peak, 2)

    normal = run(
        [
            "ffmpeg",
            "-hide_banner",
            "-y",
            "-i",
            str(trimmed),
            "-af",
            f"volume={gain_db}dB",
            "-ac",
            "1",
            "-ar",
            "44100",
            "-c:a",
            "libvorbis",
            "-q:a",
            "5",
            str(normal_out),
        ]
    )
    accent = run(
        [
            "ffmpeg",
            "-hide_banner",
            "-y",
            "-i",
            str(trimmed),
            "-af",
            f"volume={gain_db}dB,asetrate=44100*{ACCENT_PITCH_FACTOR},aresample=44100",
            "-ac",
            "1",
            "-ar",
            "44100",
            "-c:a",
            "libvorbis",
            "-q:a",
            "5",
            str(accent_out),
        ]
    )
    if normal.returncode != 0 or accent.returncode != 0:
        print(
            f"  encode failed: {normal.stderr[-200:]} / {accent.stderr[-200:]}",
            file=sys.stderr,
        )
        return False
    return True


def write_wav(path: Path, samples: list[float], sample_rate: int = 44100) -> None:
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(sample_rate)
        frames = bytearray()
        for value in samples:
            clamped = max(-0.95, min(0.95, value))
            frames += struct.pack("<h", int(clamped * 32767))
        handle.writeframes(bytes(frames))


def normalize_peak(samples: list[float], target: float = 0.95) -> list[float]:
    peak = max(abs(v) for v in samples) or 1.0
    return [v * target / peak for v in samples]


def synth_click(
    sample_rate: int,
    attack_noise_ms: float,
    partials: list[tuple[float, float, float]],
    duration: float,
) -> list[float]:
    """partials: list of (frequency, initial amplitude, tau decay seconds)."""
    count = int(sample_rate * duration)
    noise_for = int(sample_rate * attack_noise_ms / 1000)
    rng_state = 12345
    out = [0.0] * count
    for index in range(count):
        time = index / sample_rate
        value = 0.0
        for frequency, amplitude, tau in partials:
            value += (
                amplitude
                * math.exp(-time / tau)
                * math.sin(2 * math.pi * frequency * time)
            )
        if index < noise_for:
            rng_state = (1103515245 * rng_state + 12345) & 0x7FFFFFFF
            noise = (rng_state / 0x3FFFFFFF) - 1.0
            value += noise * 0.5 * (1.0 - index / noise_for)
        fade_out = min(1.0, (duration - time) / 0.004)
        out[index] = value * fade_out
    return normalize_peak(out)


def synth_chirp(
    sample_rate: int,
    frequency_start: float,
    frequency_end: float,
    amplitude: float,
    tau: float,
    duration: float,
) -> list[float]:
    count = int(sample_rate * duration)
    out = [0.0] * count
    for index in range(count):
        time = index / sample_rate
        progress = time / duration
        phase = 2 * math.pi * (
            frequency_start * time
            + (frequency_end - frequency_start) * time * progress / 2
        )
        value = amplitude * math.exp(-time / tau) * math.sin(phase)
        fade_out = min(1.0, (duration - time) / 0.004)
        out[index] = value * fade_out
    return normalize_peak(out)


def encode_wav_to_ogg(wav: Path, out: Path) -> None:
    result = run(
        [
            "ffmpeg",
            "-hide_banner",
            "-y",
            "-i",
            str(wav),
            "-ac",
            "1",
            "-ar",
            "44100",
            "-c:a",
            "libvorbis",
            "-q:a",
            "5",
            str(out),
        ]
    )
    if result.returncode != 0:
        raise RuntimeError(f"ffmpeg failed for {wav}: {result.stderr[-300:]}")


def generate_synthetic(work: Path) -> list[dict]:
    generated = []
    specs = {
        "gen_soft": "Synth Soft",
        "gen_wood": "Synth Woodblock",
    }
    for slug, label in specs.items():
        for variant in ("normal", "accent"):
            if slug == "gen_soft":
                if variant == "normal":
                    samples = synth_click(44100, 2.0, [(640.0, 0.7, 0.040)], 0.22)
                else:
                    samples = synth_click(44100, 2.5, [(880.0, 0.75, 0.034)], 0.20)
            elif slug == "gen_wood":
                if variant == "normal":
                    samples = synth_click(
                        44100,
                        1.5,
                        [(1250.0, 0.8, 0.018), (1870.0, 0.5, 0.012), (2680.0, 0.3, 0.008)],
                        0.14,
                    )
                else:
                    samples = synth_click(
                        44100,
                        1.5,
                        [(1560.0, 0.8, 0.018), (2340.0, 0.5, 0.012), (3350.0, 0.3, 0.008)],
                        0.14,
                    )
            else:
                if variant == "normal":
                    samples = synth_chirp(44100, 1800.0, 900.0, 0.7, 0.030, 0.12)
                else:
                    samples = synth_chirp(44100, 2400.0, 1200.0, 0.7, 0.030, 0.12)
            wav = work / f"{slug}_{variant}.wav"
            write_wav(wav, samples)
            out_name = f"{slug}.ogg" if variant == "normal" else f"{slug}_accent.ogg"
            encode_wav_to_ogg(wav, OUT_DIR / out_name)
        generated.append({"slug": slug, "label": label, "synthetic": True})
    return generated


def emit_rust(entries: list[dict]) -> None:
    lines = [
        "// GENERATED by scripts/build_candidate_bank.py — do not edit by hand.",
        "use super::sound_bank::CandidateSound;",
        "",
        "pub const CANDIDATES: &[CandidateSound] = &[",
    ]
    for entry in entries:
        slug = entry["slug"]
        label = entry["label"].replace('"', "'")
        lines.append(
            f'    CandidateSound {{ label: "{label}", '
            f'normal: include_bytes!("../../assets/audio/candidates/{slug}.ogg"), '
            f'accent: include_bytes!("../../assets/audio/candidates/{slug}_accent.ogg") '
            "},"
        )
    lines.append("];")
    RUST_PATH.write_text("\n".join(lines) + "\n")


def emit_manifest(fs_entries: list[dict], synth_entries: list[dict]) -> None:
    rows = [
        "# Candidate sample bank",
        "",
        "Audition candidates for the metronome click. All Freesound entries are",
        "license:Creative Commons 0 (CC0); files are processed high-quality CDN",
        "previews (mono, 44.1 kHz, Vorbis q5, silence-trimmed, peak-normalized,",
        "accent derived by 1.25x pitch-up). Before shipping a kept sample",
        "permanently, re-export from the original file for maximum quality.",
        "",
        "| Selector label | Source | Title (Freesound) | Author | Duration |",
        "| --- | --- | --- | --- | --- |",
    ]
    for entry in fs_entries:
        slug = entry["slug"]
        label = entry["label"]
        url = f"https://freesound.org/s/{entry['id']}/"
        rows.append(
            f"| {label} | [{url}]({url}) | {entry['title']} | {entry['username']} | {entry['duration']:.2f}s |"
        )
        rows.append(f"| | files: `{slug}.ogg` + `{slug}_accent.ogg` | | | |")
    rows.append("")
    rows.append("## Generated (scripts/build_candidate_bank.py)")
    for entry in synth_entries:
        rows.append(f"- {entry['label']}: `{entry['slug']}.ogg` + `{entry['slug']}_accent.ogg`")
    MANIFEST_PATH.write_text("\n".join(rows) + "\n")


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    work = Path("/tmp/takto_candidates")
    work.mkdir(parents=True, exist_ok=True)

    print("Scraping Freesound CC0 search results…")
    fs_picked = pick_candidates()
    print(f"Picked {len(fs_picked)} candidates")

    letters: dict[str, int] = {}
    entries: list[dict] = []
    for entry in fs_picked:
        index = letters.get(entry["category"], 0)
        letters[entry["category"]] = index + 1
        suffix = chr(ord("A") + index)
        category_slug = entry["category"].lower().replace(" ", "")
        slug = f"fs{entry['id']}_{category_slug}{suffix.lower()}"
        label = f"{entry['category']} {suffix}"
        raw = work / f"{entry['id']}.raw"
        print(f"Processing {label}: {entry['title']}")
        if not download_preview(entry, raw):
            print("  skipped: no downloadable preview", file=sys.stderr)
            continue
        if not process_sound(raw, OUT_DIR / f"{slug}.ogg", OUT_DIR / f"{slug}_accent.ogg"):
            print("  skipped: processing failed", file=sys.stderr)
            continue
        entry = {**entry, "slug": slug, "label": label}
        entries.append(entry)

    print("Generating synthetic clicks…")
    synth_entries = generate_synthetic(work)
    entries.extend(synth_entries)

    emit_rust(entries)
    emit_manifest(entries[: len(entries) - len(synth_entries)], synth_entries)
    print(f"Wrote {len(entries)} candidates to {OUT_DIR}")
    print(f"Rust table: {RUST_PATH}")


if __name__ == "__main__":
    main()
