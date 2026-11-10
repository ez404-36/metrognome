use std::sync::atomic::Ordering;

use super::sound_bank::SoundBank;
use crate::model::SharedParams;

const VOICE_COUNT: usize = 8;

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
            // Playback is sample-scheduled in the audio callback, so the
            // first hit can start with the very next rendered frame. Adding
            // a fixed delay here only makes Play feel unresponsive.
            self.next_hit = self.sample_position as f64;
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
            let (active_mask, beat_count) = params.pattern();

            // A preview plays a complete measure at the current tempo and
            // rhythm, so all of its voices are scheduled up front.
            for beat_index in 0..beat_count {
                let highlight_first = params.highlight_first.load(Ordering::Relaxed);
                let kind = if beat_index == 0 && highlight_first {
                    VoiceKind::First
                } else {
                    VoiceKind::Normal
                };

                let active = active_mask & (1 << beat_index) != 0;
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
                let (active_mask, beat_count) = params.pattern();
                let beat_index = self.beat % beat_count;
                let active = active_mask & (1 << beat_index) != 0;
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
    fn first_start_starts_on_the_next_rendered_frame() {
        let params = SharedParams::new();
        params.set_bpm(120);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params
            .running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let bank = SoundBank {
            normal: vec![0.1; 64],
            first_source: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 20_000];

        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert!((output[0] - 0.9 * gain).abs() < 1e-4);
        assert!(output[1..64]
            .iter()
            .all(|sample| (*sample - 0.9 * gain).abs() < 1e-4));
        // current_beat reports the beat that was just scheduled.
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
        // Ten events at 9600 frames each.
        let mut output = vec![0.0_f32; 96_000];

        scheduler.render(&params, &bank, &mut output, 2);

        // 300 BPM gives an event every 9600 frames starting immediately. After ten events,
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
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Muted,
            ],
        );
        params.running.store(true, Ordering::Relaxed);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            first_source: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 40_000];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert!((output[0] - 0.9 * gain).abs() < 1e-4); // beat 0 sounds (first)
        assert!(output[11_999].abs() < 1e-6); // muted beat 1: no sound at all
        assert!((output[24_000] - 0.1 * gain).abs() < 1e-4); // beat 2 sounds normally
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
                BeatState::Enabled,
                BeatState::Normal,
                BeatState::Enabled,
                BeatState::Normal,
            ],
        );
        params.running.store(true, Ordering::Relaxed);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            first_source: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 40_000];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        // Playback starts with the current buffer; 240 BPM = 12_000 frames/beat.
        assert!((output[0] - 0.9 * gain).abs() < 1e-4);
        assert!(output[1..64].iter().all(|s| (*s - 0.9 * gain).abs() < 1e-4));
        assert!((output[12_000] - 0.1 * gain).abs() < 1e-4); // unaccented beat 1 plays normal
        assert!((output[24_000] - 0.1 * gain).abs() < 1e-4); // beat 2 plays normally
    }

    #[test]
    fn first_beat_special_sound_can_be_disabled() {
        let params = SharedParams::new();
        params.set_bpm(240);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(
            4,
            &[
                BeatState::Enabled,
                BeatState::Normal,
                BeatState::Enabled,
                BeatState::Normal,
            ],
        );
        params.running.store(true, Ordering::Relaxed);
        params.set_highlight_first(false);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            first_source: vec![0.5; 64],
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
                BeatState::Enabled,
                BeatState::Normal,
                BeatState::Enabled,
                BeatState::Muted,
            ],
        );
        params.preview_generation.store(1, Ordering::Relaxed);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            first_source: vec![0.5; 64],
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
            first_source: vec![0.9; 64],
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
        params.set_states(4, &[BeatState::Enabled; 4]);
        params.set_highlight_first(false);
        params.preview_generation.store(1, Ordering::SeqCst);

        let bank = SoundBank {
            normal: vec![0.1; 64],
            first_source: vec![0.5; 64],
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

    #[test]
    fn tempo_change_while_running_keeps_phase_without_restart_or_burst() {
        let params = SharedParams::new();
        params.set_bpm(120); // 12_000 frames per beat at 48 kHz.
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.running.store(true, Ordering::Relaxed);
        let bank = SoundBank {
            normal: vec![1.0; 8],
            first_source: vec![1.0; 8],
            first: vec![1.0; 8],
        };
        let mut scheduler = Scheduler::new(0, 4);

        // Immediate playback puts hits at 0, 24_000 and 48_000; the next hit
        // is pending at 72_000 when this buffer ends.
        let mut first = vec![0.0_f32; 72_000];
        scheduler.render(&params, &bank, &mut first, 1);

        params.set_bpm(240); // period halves to 12_000 frames per beat.
        let mut second = vec![0.0_f32; 48_000]; // frames 72_000..120_000
        scheduler.render(&params, &bank, &mut second, 1);

        let gain = params.volume();
        // The pending hit lands where it was scheduled under the old tempo,
        // then the new period applies. No restart gap, no burst.
        for hit in [72_000, 84_000, 96_000] {
            let sample = second[hit - 72_000];
            assert!((sample - gain).abs() < 1e-4, "expected hit at {hit}");
        }
        for gap in [78_000, 90_000, 102_000, 114_000] {
            assert!(
                second[gap - 72_000].abs() < 1e-6,
                "expected silence at {gap}"
            );
        }
    }

    #[test]
    fn meter_change_while_running_keeps_tempo_and_wraps_beat_cycle() {
        let params = SharedParams::new();
        params.set_bpm(240); // 12_000 frames per beat at 48 kHz.
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.set_states(4, &[BeatState::Enabled; 4]);
        params.running.store(true, Ordering::Relaxed);
        let bank = SoundBank {
            normal: vec![0.2; 4],
            first_source: vec![0.2; 4],
            first: vec![0.8; 4],
        };
        let mut scheduler = Scheduler::new(0, 4);

        // One full 4/4 measure: hits at 0..36_000, the next downbeat at
        // 48_000, and its successor pending at 60_000.
        let mut first = vec![0.0_f32; 48_000];
        scheduler.render(&params, &bank, &mut first, 1);
        let gain = params.volume();
        assert!((first[0] - 0.8 * gain).abs() < 1e-4); // 4/4 downbeat
        assert!((first[24_000] - 0.2 * gain).abs() < 1e-4);

        // Switch to 2/4 mid-stream: the packed pattern updates atomically and
        // the hit grid keeps its tempo phase while the beat cycle wraps.
        params.set_states(2, &[BeatState::Enabled; 2]);
        let mut second = vec![0.0_f32; 60_000]; // frames 48_000..108_000
        scheduler.render(&params, &bank, &mut second, 1);

        // Beat counter is at 4, so 4 % 2 = 0 is the 2/4 downbeat.
        assert!((second[0] - 0.8 * gain).abs() < 1e-4); // 48_000
        assert!((second[12_000] - 0.2 * gain).abs() < 1e-4); // 60_000
        assert!((second[24_000] - 0.8 * gain).abs() < 1e-4); // 72_000
        assert!((second[36_000] - 0.2 * gain).abs() < 1e-4); // 84_000
    }

    #[test]
    fn sound_bank_swap_mid_tail_is_safe_and_releases_the_voice() {
        let params = SharedParams::new();
        params.set_bpm(120);
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.running.store(true, Ordering::Relaxed);
        let long_bank = SoundBank {
            normal: vec![0.5; 200],
            first_source: vec![0.5; 200],
            first: vec![0.5; 200],
        };
        let short_bank = SoundBank {
            normal: vec![0.25; 10],
            first_source: vec![0.25; 10],
            first: vec![0.25; 10],
        };
        let mut scheduler = Scheduler::new(0, 4);

        // Startup hit at 12_000; the render stops mid-tail at position 100.
        let mut first = vec![0.0_f32; 12_100];
        scheduler.render(&params, &long_bank, &mut first, 1);

        // Swap to the shorter bank while the voice is still playing: the
        // voice reads the new bank, releases without panic, and the tail
        // stays silent instead of replaying the old sample.
        let mut second = vec![0.0_f32; 50];
        scheduler.render(&params, &short_bank, &mut second, 1);
        assert!(second.iter().all(|sample| sample.abs() < 1e-6));
    }

    #[test]
    fn batched_one_shots_are_all_consumed_and_mixed() {
        let params = SharedParams::new();
        params.sample_rate.store(48_000, Ordering::Relaxed);
        params.one_shot_pending.store(3, Ordering::SeqCst);

        let bank = SoundBank {
            normal: vec![0.25; 64],
            first_source: vec![0.5; 64],
            first: vec![0.9; 64],
        };
        let mut scheduler = Scheduler::new(0, 4);
        let mut output = vec![0.0_f32; 256];
        scheduler.render(&params, &bank, &mut output, 1);

        let gain = params.volume();
        assert_eq!(params.one_shot_pending.load(Ordering::SeqCst), 0);
        // Three overlapping one-shots mix into one sample value.
        assert!((output[0] - 0.75 * gain).abs() < 1e-4);
        assert_eq!(output[64], 0.0);
    }
}
