use std::sync::atomic::Ordering;

use super::sound_bank::SoundBank;
use crate::model::SharedParams;

const VOICE_COUNT: usize = 8;

#[derive(Clone, Copy, Debug)]
struct Voice {
    accent: bool,
    position: usize,
    start_position: u64,
}

#[derive(Debug)]
pub struct Scheduler {
    sample_position: u64,
    next_hit: f64,
    beat: u32,
    was_running: bool,
    voices: [Option<Voice>; VOICE_COUNT],
}

impl Scheduler {
    pub fn new(sample_position: u64, beat_count: u32) -> Self {
        let period = sample_period(120.0, 48_000);
        let mut scheduler = Self {
            sample_position,
            next_hit: sample_position as f64 + period,
            beat: 0,
            was_running: false,
            voices: [None; VOICE_COUNT],
        };
        scheduler.beat = beat_count.min(1) - 1;
        scheduler
    }

    pub fn render<T: OutputSample>(
        &mut self,
        params: &SharedParams,
        bank: &SoundBank,
        output: &mut [T],
        channels: usize,
    ) {
        let running = params.running.load(Ordering::Relaxed);
        let sample_rate = params.sample_rate.load(Ordering::Relaxed).max(1);
        let bpm = f64::from(params.bpm());
        let period = sample_period(bpm, sample_rate);

        if running && !self.was_running {
            self.beat = 0;
            self.next_hit = self.sample_position as f64 + 1.0;
        }

        self.was_running = running;
        let end = self.sample_position + output.len() as u64 / channels.max(1) as u64;

        if running {
            while (self.next_hit as u64) < end {
                let beat_count = params.beat_count();
                let accent = params.accent_mask.load(Ordering::Relaxed)
                    & (1 << (self.beat % beat_count))
                    != 0;

                if let Some(free_voice) = self.voices.iter_mut().find(|voice| voice.is_none()) {
                    *free_voice = Some(Voice {
                        accent,
                        position: 0,
                        start_position: self.next_hit as u64,
                    });
                }

                params
                    .current_beat
                    .store(self.beat % beat_count, Ordering::Relaxed);
                self.beat = self.beat.wrapping_add(1);
                self.next_hit += period;
            }
        }

        let master_gain = params.volume();
        let frame_start = self.sample_position;

        for (frame_index, frame) in output.chunks_mut(channels.max(1)).enumerate() {
            let current_frame = frame_start + frame_index as u64;
            let mut mixed = 0.0_f32;

            for voice in self.voices.iter_mut().flatten() {
                if current_frame < voice.start_position {
                    continue;
                }

                let source = if voice.accent {
                    bank.accent.as_slice()
                } else {
                    bank.normal.as_slice()
                };

                if voice.position < source.len() {
                    mixed += source[voice.position];
                    voice.position += 1;
                }
            }

            let value = T::from_f32((mixed * master_gain).clamp(-1.0, 1.0));
            frame.fill(value);
        }

        for voice in self.voices.iter_mut() {
            if voice.as_ref().is_some_and(|voice| {
                let source_length = if voice.accent {
                    bank.accent.len()
                } else {
                    bank.normal.len()
                };
                voice.position >= source_length
            }) {
                *voice = None;
            }
        }

        self.sample_position = end;
    }
}

fn sample_period(bpm: f64, sample_rate: u32) -> f64 {
    if bpm < 1.0 {
        return f64::from(sample_rate);
    }

    sample_rate as f64 * 60.0 / bpm
}

pub trait OutputSample: Copy {
    fn from_f32(value: f32) -> Self;
}

impl OutputSample for f32 {
    fn from_f32(value: f32) -> Self {
        value
    }
}

impl OutputSample for i16 {
    fn from_f32(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
    }
}

impl OutputSample for u16 {
    fn from_f32(value: f32) -> Self {
        let normalized = (value * 0.5 + 0.5).clamp(0.0, 1.0);
        (normalized * u16::MAX as f32).round() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_start_is_immediate_accent() {
        let params = SharedParams::new();
        params.set_bpm(240);
        params
            .running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let bank = SoundBank::synthetic(48_000);
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 48];

        scheduler.render(&params, &bank, &mut output, 2);

        assert!(output.iter().any(|sample| sample.abs() > 0.001));
        assert_eq!(params.current_beat(), 0);
    }

    #[test]
    fn high_tempo_is_scheduled_at_sample_boundaries() {
        let params = SharedParams::new();
        params.set_bpm(300);
        params
            .running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let bank = SoundBank::synthetic(48_000);
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 96_000];

        scheduler.render(&params, &bank, &mut output, 2);

        // 300 BPM gives an event every 9600 frames. After ten events,
        // the 4/4 indicator has wrapped back to the downbeat.
        assert_eq!(params.current_beat(), 0);
    }

    #[test]
    fn voice_slots_are_released_after_each_sound_ends() {
        let params = SharedParams::new();
        params.set_bpm(120);
        params
            .running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let bank = SoundBank::synthetic(48_000);
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 144_000];

        scheduler.render(&params, &bank, &mut output, 2);
        // At 120 BPM six beats occur in the final second. Before the fix,
        // voices for the shorter normal sound were never released.
        let tail_is_audible = output[96_000..]
            .chunks(2)
            .any(|frame| frame.iter().any(|sample| sample.abs() > 0.001));
        assert!(tail_is_audible);
    }
}
