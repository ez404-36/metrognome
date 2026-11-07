use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct SoundBank {
    pub normal: Vec<f32>,
    pub accent: Vec<f32>,
}

impl SoundBank {
    pub fn synthetic(sample_rate: u32) -> Self {
        Self::preset(SoundPreset::Classic, sample_rate)
    }

    pub fn preset(preset: SoundPreset, sample_rate: u32) -> Self {
        match preset {
            SoundPreset::Classic => Self {
                normal: synthetic_click(sample_rate, 580.0, 0.052, 0.62, 0.02),
                accent: synthetic_click(sample_rate, 820.0, 0.085, 0.88, 0.02),
            },
            SoundPreset::Woodblock => Self {
                normal: synthetic_click(sample_rate, 330.0, 0.038, 0.68, 0.10),
                accent: synthetic_click(sample_rate, 540.0, 0.062, 0.92, 0.10),
            },
            SoundPreset::Digital => Self {
                normal: synthetic_click(sample_rate, 1_240.0, 0.022, 0.58, 0.0),
                accent: synthetic_click(sample_rate, 1_680.0, 0.034, 0.86, 0.0),
            },
            SoundPreset::Soft => Self {
                normal: synthetic_click(sample_rate, 420.0, 0.085, 0.52, 0.02),
                accent: synthetic_click(sample_rate, 640.0, 0.115, 0.76, 0.02),
            },
        }
    }

    #[cfg_attr(feature = "ui", allow(dead_code))]
    pub fn from_wav_files(
        normal_path: &std::path::Path,
        accent_path: &std::path::Path,
        target_sample_rate: u32,
    ) -> Result<Self> {
        Ok(Self {
            normal: decode_wav(normal_path, target_sample_rate)?,
            accent: decode_wav(accent_path, target_sample_rate)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPreset {
    Classic,
    Woodblock,
    Digital,
    Soft,
}

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
fn synthetic_click(
    sample_rate: u32,
    frequency: f32,
    duration: f32,
    gain: f32,
    noise_gain: f32,
) -> Vec<f32> {
    let sample_count = (sample_rate as f32 * duration).round() as usize;
    let mut samples = Vec::with_capacity(sample_count);
    let mut phase = 0.0_f64;
    let mut noise_state = 0x1234_5678_u32;
    let mut lowpass = 0.0_f32;

    let attack_samples = ((sample_rate as f32 * 0.0015).round() as usize).max(1);
    let release_samples = ((sample_rate as f32 * 0.005).round() as usize).max(1);

    for index in 0..sample_count {
        let time = index as f32 / sample_rate as f32;
        phase += frequency as f64 / sample_rate as f64;

        if phase >= 1.0 {
            phase -= phase.floor();
        }

        noise_state = noise_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let noise = ((noise_state >> 8) as i32 - 0x800_000) as f32 / 0x800_000 as f32;
        let tone = (phase * std::f64::consts::TAU) as f32;
        let noise_envelope = (-time / 0.003).exp();
        let tone_envelope = (-time / (duration * 0.22)).exp();

        lowpass += 0.20 * (noise - lowpass);
        let unshaped = tone * tone_envelope + lowpass * noise_envelope * noise_gain;
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
    fn synthetic_sound_is_short_and_bounded() {
        let bank = SoundBank::synthetic(48_000);

        assert_eq!(bank.normal.len(), 2_496);
        assert!(bank.normal.iter().all(|sample| sample.abs() <= 1.0));
        assert!(bank.accent.iter().any(|sample| sample.abs() > 0.1));
        assert_eq!(bank.normal.last(), Some(&0.0));
    }
}

#[cfg(test)]
mod wav_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn imports_and_resamples_16_bit_wav() {
        let path = PathBuf::from(std::env::temp_dir())
            .join(format!("metrognome-import-test-{}.wav", std::process::id()));
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
    fn presets_are_safe_to_mix() {
        for preset in SoundPreset::ALL {
            let bank = SoundBank::preset(preset, 48_000);
            assert!(bank.normal.iter().all(|sample| sample.abs() <= 1.0));
            assert!(bank.accent.iter().all(|sample| sample.abs() <= 1.0));
            assert!(bank.normal.iter().any(|sample| sample.abs() > 0.05));
            assert!(bank.accent.iter().any(|sample| sample.abs() > 0.05));
            assert_eq!(bank.normal.last(), Some(&0.0));
            assert_eq!(bank.accent.last(), Some(&0.0));
        }
    }
}
