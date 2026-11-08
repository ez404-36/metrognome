use std::sync::atomic::Ordering;

use super::sound_bank::SoundBank;
use crate::model::SharedParams;

const VOICE_COUNT: usize = 8;
const STARTUP_DELAY_SECONDS: f64 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VoiceKind {
    Normal,
    First,
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    kind: VoiceKind,
    position: usize,
    start_position: u64,
}

#[derive(Debug)]
pub struct Scheduler {
    sample_position: u64,
    next_hit: f64,
    beat: u32,
    was_running: bool,
    preview_generation: u32,
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
            preview_generation: 0,
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
            self.voices = [None; VOICE_COUNT];
            self.next_hit =
                self.sample_position as f64 + sample_rate as f64 * STARTUP_DELAY_SECONDS;
        }

        self.was_running = running;
        let end = self.sample_position + output.len() as u64 / channels.max(1) as u64;

        // A UI tap is an immediate one-shot. Take the whole pending count in
        // one operation so clicks made between audio callbacks are not lost.
        let one_shot_count = params.one_shot_pending.fetch_and(0, Ordering::SeqCst);
        for _ in 0..one_shot_count {
            if let Some(free_voice) = self.voices.iter_mut().find(|voice| voice.is_none()) {
                *free_voice = Some(Voice {
                    kind: VoiceKind::Normal,
                    position: 0,
                    start_position: self.sample_position,
                });
            }
        }

        let preview_generation = params.preview_generation.load(Ordering::SeqCst);
        if self.preview_generation != preview_generation {
            self.preview_generation = preview_generation;
            let beat_count = params.beat_count();

            // A preview plays a complete measure at the current tempo and
            // rhythm, so all of its voices are scheduled up front.
            for beat_index in 0..beat_count {
                let active = params.active_mask.load(Ordering::Relaxed) & (1 << beat_index) != 0;
                let highlight_first = params.highlight_first.load(Ordering::Relaxed);
                let kind = if beat_index == 0 && highlight_first {
                    VoiceKind::First
                } else {
                    VoiceKind::Normal
                };

                if !active {
                    continue;
                }

                let offset = (beat_index as f64 * period).round() as u64;
                let start_position = self.sample_position.saturating_add(offset);
                if let Some(free_voice) = self.voices.iter_mut().find(|voice| voice.is_none()) {
                    *free_voice = Some(Voice {
                        kind,
                        position: 0,
                        start_position,
                    });
                }
            }
        }

        if running {
            while (self.next_hit as u64) < end {
                let beat_count = params.beat_count();
                let beat_index = self.beat % beat_count;
                let active = params.active_mask.load(Ordering::Relaxed) & (1 << beat_index) != 0;
                let highlight_first = params.highlight_first.load(Ordering::Relaxed);
                let kind = if beat_index == 0 && highlight_first {
                    VoiceKind::First
                } else {
                    VoiceKind::Normal
                };
                let hit_position = self.next_hit as u64;

                params.current_beat.store(beat_index, Ordering::Relaxed);
                self.beat = self.beat.wrapping_add(1);
                self.next_hit += period;

                // Muted beats stay completely silent but still advance the
                // beat counter so the indicator keeps moving through them.
                if !active {
                    continue;
                }

                if let Some(free_voice) = self.voices.iter_mut().find(|voice| voice.is_none()) {
                    *free_voice = Some(Voice {
                        kind,
                        position: 0,
                        start_position: hit_position,
                    });
                }
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

                let source = match voice.kind {
                    VoiceKind::First => bank.first.as_slice(),
                    VoiceKind::Normal => bank.normal.as_slice(),
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
                let source_length = match voice.kind {
                    VoiceKind::First => bank.first.len(),
                    VoiceKind::Normal => bank.normal.len(),
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
    use crate::audio::sound_bank::SoundPreset;
    use crate::model::BeatState;

    #[test]
    fn first_start_is_delayed_after_stream_activation() {
        let params = SharedParams::new();
        params.set_bpm(120);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params
            .running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let bank = SoundBank::preset(SoundPreset::Classic, 48_000);
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 20_000];

        scheduler.render(&params, &bank, &mut output, 1);

        assert!(output[..12_000].iter().all(|sample| sample.abs() < 0.001));
        assert!(output[12_000..].iter().any(|sample| sample.abs() > 0.001));
        assert_eq!(params.current_beat(), 0);
    }

    #[test]
    fn high_tempo_is_scheduled_at_sample_boundaries() {
        let params = SharedParams::new();
        params.set_bpm(300);
        params
            .running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let bank = SoundBank::preset(SoundPreset::Classic, 48_000);
        let mut scheduler = Scheduler::new(0, 4);
        // Start delay plus ten events at 9600 frames each.
        let mut output = vec![0.0_f32; 108_001];

        scheduler.render(&params, &bank, &mut output, 2);

        // 300 BPM gives an event every 9600 frames after the start delay. After ten events,
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
        let bank = SoundBank::preset(SoundPreset::Classic, 48_000);
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

    #[test]
    fn unaccented_beats_play_the_normal_sound() {
        let params = SharedParams::new();
        params.set_bpm(120);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(
            4,
            &[
                BeatState::Normal,
                BeatState::Normal,
                BeatState::Normal,
                BeatState::Normal,
            ],
        );
        params
            .running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let bank = SoundBank::preset(SoundPreset::Classic, 48_000);
        let mut scheduler = Scheduler::new(0, 4);

        // Long enough to pass several event positions after the startup delay.
        let mut output = vec![0.0_f32; 48_000];
        scheduler.render(&params, &bank, &mut output, 1);

        // Downbeat and every following beat sound now; unaccented beats use
        // the normal sound instead of staying silent.
        assert!(output[12_000..].iter().any(|sample| sample.abs() > 0.001));
        assert!(params.current_beat() > 0);
    }

    #[test]
    fn muted_beats_are_completely_silent_but_still_advance() {
        let params = SharedParams::new();
        params.set_bpm(240);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(
            4,
            &[
                BeatState::Accent,
                BeatState::Muted,
                BeatState::Accent,
                BeatState::Muted,
            ],
        );
        params.running.store(true, Ordering::Relaxed);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            accent: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 40_000];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert!((output[12_000] - 0.9 * gain).abs() < 1e-4); // beat 0 sounds (first)
        assert!(output[23_999].abs() < 1e-6); // muted beat 1: no sound at all
        assert!((output[36_000] - 0.1 * gain).abs() < 1e-4); // beat 2 sounds normally
        assert!(params.current_beat() > 0); // beat counter still advances
    }

    #[test]
    fn first_beat_of_each_measure_uses_the_special_sound() {
        let params = SharedParams::new();
        params.set_bpm(240);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(
            4,
            &[
                BeatState::Accent,
                BeatState::Normal,
                BeatState::Accent,
                BeatState::Normal,
            ],
        );
        params.running.store(true, Ordering::Relaxed);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            accent: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 40_000];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        // Startup delay puts the first hit at 12_000; 240 BPM = 12_000 frames/beat.
        assert!((output[12_000] - 0.9 * gain).abs() < 1e-4);
        assert!(output[12_001..12_064]
            .iter()
            .all(|s| (*s - 0.9 * gain).abs() < 1e-4));
        assert!((output[24_000] - 0.1 * gain).abs() < 1e-4); // unaccented beat 1 plays normal
        assert!((output[36_000] - 0.1 * gain).abs() < 1e-4); // beat 2 plays normally
    }

    #[test]
    fn first_beat_special_sound_can_be_disabled() {
        let params = SharedParams::new();
        params.set_bpm(240);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(
            4,
            &[
                BeatState::Accent,
                BeatState::Normal,
                BeatState::Accent,
                BeatState::Normal,
            ],
        );
        params.running.store(true, Ordering::Relaxed);
        params.set_highlight_first(false);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            accent: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 40_000];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert!((output[12_000] - 0.1 * gain).abs() < 1e-4);
    }

    #[test]
    fn preview_generation_plays_one_measure_while_stopped() {
        let params = SharedParams::new();
        params.set_bpm(240);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(
            4,
            &[
                BeatState::Accent,
                BeatState::Normal,
                BeatState::Accent,
                BeatState::Muted,
            ],
        );
        params.preview_generation.store(1, Ordering::Relaxed);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            accent: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 48_000];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert!((output[0] - 0.9 * gain).abs() < 1e-4);
        assert!((output[12_000] - 0.1 * gain).abs() < 1e-4);
        assert!((output[24_000] - 0.1 * gain).abs() < 1e-4);
        assert!(output[36_000..36_064]
            .iter()
            .all(|sample| sample.abs() < 1e-4));
    }

    #[test]
    fn one_shot_request_plays_normal_click_immediately() {
        let params = SharedParams::new();
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.one_shot_pending.store(1, Ordering::SeqCst);

        let bank = SoundBank {
            normal: vec![0.25; 64],
            accent: vec![0.9; 64],
            first: vec![0.5; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 256];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert_eq!(params.one_shot_pending.load(Ordering::SeqCst), 0);
        assert!((output[0] - 0.25 * gain).abs() < 1e-4);
        assert!((output[63] - 0.25 * gain).abs() < 1e-4);
        assert_eq!(output[64], 0.0);
        assert_eq!(output[128], 0.0);
    }

    #[test]
    fn preview_honours_disabled_first_beat_highlight() {
        let params = SharedParams::new();
        params.set_bpm(240);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(4, &[BeatState::Accent; 4]);
        params.set_highlight_first(false);
        params.preview_generation.store(1, Ordering::SeqCst);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            accent: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 48_000];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert!((output[0] - 0.1 * gain).abs() < 1e-4);
        assert!((output[12_000] - 0.1 * gain).abs() < 1e-4);
        assert!((output[24_000] - 0.1 * gain).abs() < 1e-4);
        assert!((output[36_000] - 0.1 * gain).abs() < 1e-4);
    }
}
