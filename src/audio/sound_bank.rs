use anyhow::{anyhow, Result};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Imported click samples must stay short. The header check runs before any
/// sample data is read so an oversized file cannot blow up memory while it
/// is being decoded.
const MAX_WAV_DURATION_SECONDS: f64 = 5.0;

/// An audition candidate: a normal click plus its derived accent variant,
/// both embedded as OGG Vorbis bytes.
///
/// The accent asset is never played directly: it is only the source the
/// first-beat sound is derived from (see `derive_first_sound`).
pub struct CandidateSound {
    pub label: &'static str,
    pub normal: &'static [u8],
    pub accent: &'static [u8],
}

#[derive(Debug, Clone)]
pub struct SoundBank {
    pub normal: Vec<f32>,
    /// Accent sample used only as the source for `first`; no beat plays it
    /// directly.
    pub first_source: Vec<f32>,
    /// Special sound for the first tick of each measure (brighter + louder
    /// than the plain tick so bar starts stand out).
    pub first: Vec<f32>,
}

impl SoundBank {
    pub fn preset(preset: SoundPreset, sample_rate: u32) -> Self {
        match preset {
            SoundPreset::Classic => Self::embedded(sample_rate).unwrap_or_else(|_| Self {
                normal: synthetic_click(sample_rate, 580.0, 0.052, 0.62),
                first_source: synthetic_click(sample_rate, 820.0, 0.085, 0.88),
                first: synthetic_click(sample_rate, 1_150.0, 0.110, 0.95),
            }),
            SoundPreset::Woodblock => Self {
                normal: synthetic_click(sample_rate, 330.0, 0.038, 0.68),
                first_source: synthetic_click(sample_rate, 540.0, 0.062, 0.92),
                first: synthetic_click(sample_rate, 720.0, 0.085, 0.95),
            },
            SoundPreset::Digital => Self {
                normal: synthetic_click(sample_rate, 1_240.0, 0.022, 0.58),
                first_source: synthetic_click(sample_rate, 1_680.0, 0.034, 0.86),
                first: synthetic_click(sample_rate, 2_300.0, 0.042, 0.90),
            },
            SoundPreset::Soft => Self {
                normal: synthetic_click(sample_rate, 420.0, 0.085, 0.52),
                first_source: synthetic_click(sample_rate, 640.0, 0.115, 0.76),
                first: synthetic_click(sample_rate, 830.0, 0.135, 0.85),
            },
        }
    }

    pub fn embedded(sample_rate: u32) -> Result<Self> {
        let accent =
            decode_ogg_bytes(include_bytes!("../../assets/audio/accent.ogg"), sample_rate)?;
        let mut first = derive_first_sound(&accent, sample_rate);
        emphasize_first_sound(&mut first, &accent);
        Ok(Self {
            normal: decode_ogg_bytes(include_bytes!("../../assets/audio/tick.ogg"), sample_rate)?,
            first,
            first_source: accent,
        })
    }

    pub fn candidate(index: usize, sample_rate: u32) -> Result<Self> {
        let candidate = super::candidate_assets::CANDIDATES
            .get(index)
            .ok_or_else(|| anyhow!("candidate sound index {index} is out of range"))?;
        let raw_accent = decode_ogg_bytes(candidate.accent, sample_rate)?;
        let first = derive_first_sound(&raw_accent, sample_rate);
        Ok(Self {
            normal: decode_ogg_bytes(candidate.normal, sample_rate)?,
            first,
            first_source: raw_accent,
        })
    }

    pub fn candidate_labels() -> Vec<&'static str> {
        super::candidate_assets::CANDIDATES
            .iter()
            .map(|candidate| candidate.label)
            .collect()
    }

    #[cfg_attr(feature = "ui", allow(dead_code))]
    pub fn from_wav_files(
        normal_path: &std::path::Path,
        accent_path: &std::path::Path,
        target_sample_rate: u32,
    ) -> Result<Self> {
        let accent = decode_wav(accent_path, target_sample_rate)?;
        let first = derive_first_sound(&accent, target_sample_rate);
        Ok(Self {
            normal: decode_wav(normal_path, target_sample_rate)?,
            first_source: accent,
            first,
        })
    }
}

/// Derives the special "first tick" sound from an accent hit by pitching it
/// up ~35% and lifting its gain, so the measure downbeat reads distinctly.
pub fn derive_first_sound(accent: &[f32], sample_rate: u32) -> Vec<f32> {
    let pitched_source_rate = (sample_rate as f32 * 1.35).round() as u32;
    let pitched = resample_linear(accent.to_vec(), pitched_source_rate, sample_rate);

    pitched.into_iter().map(|sample| sample * 1.4).collect()
}

/// Raises the Classic downbeat to a consistent loudness. The bundled accent
/// has a higher crest factor than the candidate accents, so a fixed gain made
/// its first hit almost indistinguishable once the final output was clipped.
fn emphasize_first_sound(first: &mut [f32], first_source: &[f32]) {
    let target_rms = rms(first_source) * 2.5;
    let current_rms = rms(first).max(f32::EPSILON);
    let gain = (target_rms / current_rms).min(2.5);

    for sample in first {
        // Soft-limit the boosted transient instead of clipping it at the DAC.
        *sample = ((*sample * gain).tanh() * 0.95).clamp(-0.95, 0.95);
    }
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    (samples.iter().map(|sample| *sample * *sample).sum::<f32>() / samples.len() as f32).sqrt()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ui", allow(dead_code))]
pub enum SoundPreset {
    Classic,
    Woodblock,
    Digital,
    Soft,
}

#[cfg_attr(feature = "ui", allow(dead_code))]
impl SoundPreset {
    pub const ALL: [Self; 4] = [Self::Classic, Self::Woodblock, Self::Digital, Self::Soft];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Woodblock => "Woodblock",
            Self::Digital => "Digital",
            Self::Soft => "Soft",
        }
    }
}

pub fn decode_wav(path: &std::path::Path, target_sample_rate: u32) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).map_err(|error| anyhow!("{error}"))?;
    let spec = reader.spec();
    let source_sample_rate = spec.sample_rate;
    let channels = usize::from(spec.channels.max(1));

    // Reject overlong files from the header alone, before allocating.
    let header_seconds = f64::from(reader.duration()) / f64::from(source_sample_rate.max(1));
    if !header_seconds.is_finite() || header_seconds > MAX_WAV_DURATION_SECONDS {
        return Err(anyhow!(
            "WAV file is {header_seconds:.1} s long; imported clicks are limited to \
             {MAX_WAV_DURATION_SECONDS:.0} s"
        ));
    }

    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|sample| {
                let sample = sample.map_err(|error| anyhow!("{error}"))?;
                Ok(f32::from(sample) / f32::from(i16::MAX))
            })
            .collect::<Result<_>>()?,
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|error| anyhow!("{error}")))
            .collect::<Result<_>>()?,
    };

    if interleaved.is_empty() {
        return Err(anyhow!("WAV file contains no samples"));
    }

    let mono = downmix_to_mono(interleaved, channels);
    let decoded_seconds = mono.len() as f64 / f64::from(source_sample_rate.max(1));
    if decoded_seconds > MAX_WAV_DURATION_SECONDS {
        return Err(anyhow!(
            "WAV file is {decoded_seconds:.1} s long; imported clicks are limited to \
             {MAX_WAV_DURATION_SECONDS:.0} s"
        ));
    }
    let resampled = resample_linear(mono, source_sample_rate, target_sample_rate);
    normalize_peak(resampled)
}

fn downmix_to_mono(samples: Vec<f32>, channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples;
    }

    samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

pub fn decode_ogg_bytes(bytes: &[u8], target_sample_rate: u32) -> Result<Vec<f32>> {
    let source = MediaSourceStream::new(
        Box::new(std::io::Cursor::new(bytes.to_vec())),
        Default::default(),
    );
    let mut hint = Hint::new();
    hint.with_extension("ogg");

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| anyhow!("OGG probe failed: {error}"))?;

    let mut format_reader = probed.format;
    let track = format_reader
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("OGG file has no supported audio track"))?
        .clone();

    let source_sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("OGG track has no sample rate"))?;
    let channels = track
        .codec_params
        .channels
        .map(|channels| channels.count())
        .unwrap_or(1)
        .max(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|error| anyhow!("OGG decoder init failed: {error}"))?;

    let mut interleaved = Vec::new();
    loop {
        let packet = match format_reader.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(error) => return Err(anyhow!("OGG decode failed: {error}")),
        };

        if packet.track_id() != track.id {
            continue;
        }

        let decoded = decoder
            .decode(&packet)
            .map_err(|error| anyhow!("OGG packet decode failed: {error}"))?;
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        interleaved.extend_from_slice(buffer.samples());
    }

    if interleaved.is_empty() {
        return Err(anyhow!("OGG file contains no samples"));
    }

    let mono = downmix_to_mono(interleaved, channels);
    let resampled = resample_linear(mono, source_sample_rate, target_sample_rate);
    normalize_peak(resampled)
}

fn resample_linear(samples: Vec<f32>, source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() {
        return samples;
    }

    if source_rate == target_rate || source_rate == 0 {
        return samples;
    }

    let source_len = samples.len();
    let target_len = ((samples.len() as f64 * f64::from(target_rate) / f64::from(source_rate))
        .round() as usize)
        .max(1);
    let source_length_limit = source_len.saturating_sub(1) as f64;
    let mut output = Vec::with_capacity(target_len);

    for index in 0..target_len {
        let position = index as f64 * f64::from(source_rate) / f64::from(target_rate);
        let left_index = (position as usize).min(source_len.saturating_sub(1));
        let right_index = (left_index + 1).min(source_len.saturating_sub(1));
        let fraction = if source_len <= 1 {
            0.0
        } else {
            position.min(source_length_limit) - left_index as f64
        };
        let left = samples[left_index];
        let right = samples[right_index];
        output.push(left + (right - left) * fraction as f32);
    }

    output
}

fn normalize_peak(mut samples: Vec<f32>) -> Result<Vec<f32>> {
    let peak = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));

    if !peak.is_finite() || peak <= f32::EPSILON {
        return Err(anyhow!("WAV file has no audible signal"));
    }

    if peak > 0.0 {
        let gain = 0.9 / peak;
        for sample in &mut samples {
            *sample *= gain;
        }
    }

    Ok(samples)
}
fn synthetic_click(sample_rate: u32, frequency: f32, duration: f32, gain: f32) -> Vec<f32> {
    let sample_count = (sample_rate as f32 * duration).round() as usize;
    let mut samples = Vec::with_capacity(sample_count);
    let mut phase = 0.0_f64;
    let attack_samples = ((sample_rate as f32 * 0.0015).round() as usize).max(1);
    let release_samples = ((sample_rate as f32 * 0.005).round() as usize).max(1);

    for index in 0..sample_count {
        let time = index as f32 / sample_rate as f32;
        phase += frequency as f64 / sample_rate as f64;

        if phase >= 1.0 {
            phase -= phase.floor();
        }

        let tone = (phase * std::f64::consts::TAU) as f32;
        let tone_envelope = (-time / (duration * 0.22)).exp();
        let unshaped = tone * tone_envelope;
        let attack = (index as f32 / attack_samples as f32).min(1.0);
        let remaining = sample_count.saturating_sub(index).max(1);
        let release = if remaining <= release_samples {
            (remaining.saturating_sub(1) as f32 / release_samples.max(1) as f32).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let faded = unshaped * gain * attack * release;
        samples.push(faded.clamp(-1.0, 1.0));
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_classic_sound_is_short_and_bounded() {
        let bank = SoundBank::preset(SoundPreset::Classic, 48_000);
        let peak = |samples: &[f32]| {
            samples
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
        };
        let first_rms = rms(&bank.first);
        let first_source_rms = rms(&bank.first_source);

        assert!(
            (400..=900).contains(&bank.normal.len()),
            "unexpected embedded tick length: {}",
            bank.normal.len()
        );
        assert!(bank.normal.iter().all(|sample| sample.abs() <= 1.0));
        assert!(bank.normal.iter().any(|sample| sample.abs() > 0.1));
        assert!(bank.first_source.iter().any(|sample| sample.abs() > 0.1));
        assert!(
            first_rms > first_source_rms * 1.5,
            "Classic first RMS {first_rms} must be clearly above source RMS {first_source_rms}"
        );
        assert!(
            peak(&bank.first) <= 0.95,
            "Classic first hit must stay below full scale"
        );
    }
}

#[cfg(test)]
mod candidate_tests {
    use super::*;

    #[test]
    fn candidates_decode_and_derive_a_louder_first_sound() {
        assert!(SoundBank::candidate_labels().len() >= 18);

        let bank = SoundBank::candidate(0, 48_000).unwrap();

        assert!(!bank.normal.is_empty());
        assert!(!bank.first_source.is_empty());
        assert!(!bank.first.is_empty());
        // The derived first sound is the pitched-up accent source with its
        // gain lifted, so it must read clearly louder than its source.
        assert!(rms(&bank.first) > rms(&bank.first_source));
        assert!(bank.first_source.iter().all(|sample| sample.abs() <= 1.0));
        assert!(bank.normal.iter().all(|sample| sample.abs() <= 1.0));

        assert!(SoundBank::candidate(usize::MAX, 48_000).is_err());
    }
}

#[cfg(test)]
mod wav_tests {
    use super::*;

    #[test]
    fn imports_and_resamples_16_bit_wav() {
        let path =
            std::env::temp_dir().join(format!("takto-import-test-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 24_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();

        for index in 0..2_400 {
            let phase = f64::from(index) / 100.0;
            let value = (phase * std::f64::consts::PI).sin() as f32 * 0.5;
            writer
                .write_sample((value * f32::from(i16::MAX)) as i16)
                .unwrap();
        }
        writer.finalize().unwrap();

        let imported = decode_wav(&path, 48_000).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(imported.len(), 4_800);
        assert!(imported.iter().any(|sample| sample.abs() > 0.4));
        assert!(imported.iter().all(|sample| sample.abs() <= 0.91));
    }

    #[test]
    fn unsupported_bit_depths_fail_cleanly() {
        let path =
            std::env::temp_dir().join(format!("takto-import-test24-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        writer.write_sample(1_000_000_i32).unwrap();
        writer.write_sample(-1_000_000_i32).unwrap();
        writer.finalize().unwrap();

        // 24-bit integer WAV is a common DAW export format; the import must
        // reject it with an error instead of silently producing garbage.
        let imported = decode_wav(&path, 48_000);
        std::fs::remove_file(&path).unwrap();
        assert!(imported.is_err());
    }

    #[test]
    fn empty_and_silent_wav_files_are_rejected() {
        let path = std::env::temp_dir().join(format!(
            "takto-import-test-silent-{}.wav",
            std::process::id()
        ));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        // Digital silence: normalize_peak must reject a file with no signal.
        writer.write_sample(0_i16).unwrap();
        writer.write_sample(0_i16).unwrap();
        writer.finalize().unwrap();

        let imported = decode_wav(&path, 48_000);
        std::fs::remove_file(&path).unwrap();
        assert!(imported.is_err());
    }

    #[test]
    fn wav_files_longer_than_five_seconds_are_rejected() {
        let path =
            std::env::temp_dir().join(format!("takto-import-test-long-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 24_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..24_000 * 6 {
            writer.write_sample(1_000_i16).unwrap();
        }
        writer.finalize().unwrap();

        // The header-based limit must reject the file before its 144 000
        // samples are decoded into memory.
        let imported = decode_wav(&path, 48_000);
        std::fs::remove_file(&path).unwrap();
        let error = imported.unwrap_err().to_string();
        assert!(
            error.contains("limited to 5 s"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn wav_files_within_five_seconds_are_accepted() {
        let path =
            std::env::temp_dir().join(format!("takto-import-test-4s-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 24_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for index in 0..24_000 * 4 {
            let value = if index % 48 == 0 { 1_000 } else { 0 };
            writer.write_sample(value).unwrap();
        }
        writer.finalize().unwrap();

        let imported = decode_wav(&path, 48_000);
        std::fs::remove_file(&path).unwrap();
        assert!(imported.is_ok());
    }

    #[test]
    fn presets_are_safe_to_mix() {
        for preset in SoundPreset::ALL {
            let bank = SoundBank::preset(preset, 48_000);
            assert!(bank.normal.iter().all(|sample| sample.abs() <= 1.0));
            assert!(bank.first_source.iter().all(|sample| sample.abs() <= 1.0));
            assert!(bank.normal.iter().any(|sample| sample.abs() > 0.05));
            assert!(bank.first_source.iter().any(|sample| sample.abs() > 0.05));
        }
    }
}
