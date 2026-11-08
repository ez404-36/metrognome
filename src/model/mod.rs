use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub const MAX_BEATS: u32 = 16;

/// Per-beat playback state. The application exposes two states: Accent is
/// enabled and Muted is disabled. Muted beats produce no sound at all. The
/// first beat of a measure plays the special "first" sound while it is not
/// muted. `Normal` is retained only to decode saves made by older versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeatState {
    Muted,
    Normal,
    Accent,
}

pub fn default_states(beats: u32) -> Vec<BeatState> {
    (0..beats).map(|_| BeatState::Accent).collect()
}

/// Live application state. Every UI-relevant setting must live here so the
/// persistence snapshot covers it; when a new setting is added, map it in
/// both `AppSettings::from_params` and `AppSettings::apply_to` — the
/// `every_setting_survives_a_save_and_reload` test enumerates the current
/// set and will not pass while any of them is missed.
#[derive(Debug)]
pub struct SharedParams {
    pub running: AtomicBool,
    pub bpm: AtomicU32,
    pub volume: AtomicU32,
    pub beat_count: AtomicU32,
    pub active_mask: AtomicU32,
    pub current_beat: AtomicU32,
    pub preview_generation: AtomicU32,
    /// Number of immediate one-shot clicks requested by the UI.
    pub one_shot_pending: AtomicU32,
    pub highlight_first: AtomicBool,
    /// Selected click sample: 0 is the bundled bank, higher values index
    /// the candidate bank.
    pub sample: AtomicUsize,
    pub sample_rate: AtomicU32,
}

#[allow(dead_code)]
impl SharedParams {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            bpm: AtomicU32::new(140),
            volume: AtomicU32::new(f32::to_bits(0.85)),
            beat_count: AtomicU32::new(4),
            active_mask: AtomicU32::new(0b1111),
            current_beat: AtomicU32::new(0),
            preview_generation: AtomicU32::new(0),
            one_shot_pending: AtomicU32::new(0),
            highlight_first: AtomicBool::new(true),
            sample: AtomicUsize::new(0),
            sample_rate: AtomicU32::new(48_000),
        }
    }

    pub fn bpm(&self) -> u32 {
        self.bpm.load(Ordering::Relaxed).clamp(20, 300)
    }

    pub fn set_bpm(&self, bpm: u32) {
        self.bpm.store(bpm.clamp(20, 300), Ordering::Relaxed);
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed)).clamp(0.0, 1.0)
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume
            .store(f32::to_bits(volume.clamp(0.0, 1.0)), Ordering::Relaxed);
    }

    pub fn beat_count(&self) -> u32 {
        self.beat_count.load(Ordering::Relaxed).clamp(1, MAX_BEATS)
    }

    pub fn set_states(&self, beats: u32, states: &[BeatState]) {
        let beats = beats.clamp(1, MAX_BEATS);
        let mut active_mask = 0_u32;

        for (index, state) in states.iter().enumerate().take(beats as usize) {
            match state {
                BeatState::Accent | BeatState::Normal => {
                    active_mask |= 1 << index;
                }
                BeatState::Muted => {}
            }
        }

        self.beat_count.store(beats, Ordering::Relaxed);
        self.active_mask.store(active_mask, Ordering::Relaxed);
    }

    pub fn states(&self) -> Vec<BeatState> {
        let beats = self.beat_count();
        let active_mask = self.active_mask.load(Ordering::Relaxed);

        (0..beats)
            .map(|index| {
                if active_mask & (1 << index) == 0 {
                    BeatState::Muted
                } else {
                    BeatState::Accent
                }
            })
            .collect()
    }

    pub fn current_beat(&self) -> u32 {
        self.current_beat.load(Ordering::Relaxed)
    }

    pub fn highlight_first(&self) -> bool {
        self.highlight_first.load(Ordering::SeqCst)
    }

    pub fn set_highlight_first(&self, highlighted: bool) {
        self.highlight_first.store(highlighted, Ordering::SeqCst);
    }

    pub fn sample(&self) -> usize {
        self.sample.load(Ordering::SeqCst)
    }

    pub fn set_sample(&self, sample: usize) {
        self.sample.store(sample, Ordering::SeqCst);
    }
}

impl Default for SharedParams {
    fn default() -> Self {
        Self::new()
    }
}

/// On-disk mirror of `SharedParams`. See the note on `SharedParams` about
/// keeping both sides in sync when a new setting appears.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub bpm: u32,
    pub volume: f32,
    pub beats: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub states: Option<Vec<BeatState>>,
    /// Legacy per-beat flags from older versions (true = accented, false =
    /// plain tick). Kept for reading old saves; superseded by `states`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accents: Option<Vec<bool>>,
    #[serde(default = "default_highlight_first")]
    pub highlight_first: bool,
    /// Selected click sample position: 0 is the bundled bank, values above
    /// zero index the candidate bank. `None` comes from older saves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<usize>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bpm: 140,
            volume: 0.85,
            beats: 4,
            states: Some(default_states(4)),
            accents: None,
            highlight_first: true,
            sample: None,
        }
    }
}

fn default_highlight_first() -> bool {
    true
}

impl AppSettings {
    pub fn path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|dir| dir.join("metrognome").join("settings.json"))
    }

    pub fn load() -> Self {
        let path = Self::path();

        if let Some(path) = path {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(settings) = serde_json::from_str::<Self>(&text) {
                    return settings;
                }
            }
        }

        Self::default()
    }

    /// Snapshots the complete live state. Every persisted setting must be
    /// read here; adding a `SharedParams` field without adding it here is a
    /// bug caught by `every_setting_survives_a_save_and_reload`.
    #[cfg_attr(not(feature = "ui"), allow(dead_code))]
    pub fn from_params(params: &SharedParams) -> Self {
        Self {
            bpm: params.bpm(),
            volume: params.volume(),
            beats: params.beat_count(),
            states: Some(params.states()),
            accents: None,
            highlight_first: params.highlight_first(),
            sample: Some(params.sample()),
        }
    }

    /// Resolves the exposed two-state pattern. Older Normal states and clear
    /// legacy flags mean an audible beat, so they become Accent.
    pub fn effective_states(&self) -> Vec<BeatState> {
        if let Some(states) = &self.states {
            if !states.is_empty() {
                return states
                    .iter()
                    .map(|&state| match state {
                        BeatState::Normal => BeatState::Accent,
                        state => state,
                    })
                    .collect();
            }
        }

        if let Some(accents) = &self.accents {
            return accents.iter().map(|_| BeatState::Accent).collect();
        }

        default_states(self.beats)
    }

    #[allow(dead_code)]
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn apply_to(&self, params: &SharedParams) {
        params.set_bpm(self.bpm);
        params.set_volume(self.volume);
        params.set_states(self.beats, &self.effective_states());
        params.set_highlight_first(self.highlight_first);
        params.set_sample(self.sample.unwrap_or(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_are_clamped_and_preserved() {
        let params = SharedParams::new();
        params.set_states(
            3,
            &[
                BeatState::Accent,
                BeatState::Muted,
                BeatState::Accent,
                BeatState::Normal,
                BeatState::Accent,
            ],
        );
        let states = params.states();

        assert_eq!(
            states,
            vec![BeatState::Accent, BeatState::Muted, BeatState::Accent]
        );
    }

    #[test]
    fn settings_round_trip() {
        let settings = AppSettings {
            bpm: 183,
            volume: 0.42,
            beats: 3,
            states: Some(vec![BeatState::Accent, BeatState::Muted, BeatState::Normal]),
            accents: None,
            highlight_first: false,
            sample: Some(2),
        };
        let text = serde_json::to_string(&settings).unwrap();
        let loaded: AppSettings = serde_json::from_str(&text).unwrap();

        assert_eq!(loaded.bpm, 183);
        assert_eq!(loaded.volume, 0.42);
        assert_eq!(loaded.states, settings.states);
        assert_eq!(loaded.highlight_first, settings.highlight_first);
        assert_eq!(loaded.sample, settings.sample);
    }

    #[test]
    fn default_settings_are_enabled() {
        let settings = AppSettings::default();

        assert_eq!(
            settings.effective_states(),
            vec![
                BeatState::Accent,
                BeatState::Accent,
                BeatState::Accent,
                BeatState::Accent
            ]
        );
        assert!(settings.highlight_first);
    }

    #[test]
    fn legacy_boolean_saves_map_to_states() {
        let settings = AppSettings {
            bpm: 140,
            volume: 0.8,
            beats: 4,
            states: None,
            accents: Some(vec![true, false, false, false]),
            highlight_first: true,
            sample: None,
        };

        assert_eq!(
            settings.effective_states(),
            vec![
                BeatState::Accent,
                BeatState::Accent,
                BeatState::Accent,
                BeatState::Accent
            ]
        );
    }

    #[test]
    fn older_normal_states_are_exposed_as_enabled() {
        let settings = AppSettings {
            bpm: 140,
            volume: 0.8,
            beats: 3,
            states: Some(vec![BeatState::Accent, BeatState::Normal, BeatState::Muted]),
            accents: None,
            highlight_first: true,
            sample: None,
        };

        assert_eq!(
            settings.effective_states(),
            vec![BeatState::Accent, BeatState::Accent, BeatState::Muted]
        );
    }

    #[test]
    fn every_setting_survives_a_save_and_reload() {
        let source = SharedParams::new();
        source.set_bpm(213);
        source.set_volume(0.35);
        source.set_states(
            6,
            &[
                BeatState::Accent,
                BeatState::Muted,
                BeatState::Accent,
                BeatState::Muted,
                BeatState::Accent,
                BeatState::Accent,
            ],
        );
        source.set_highlight_first(false);
        source.set_sample(4);

        // A JSON round trip simulates writing and reading settings.json.
        let saved = AppSettings::from_params(&source);
        let text = serde_json::to_string(&saved).unwrap();
        let loaded: AppSettings = serde_json::from_str(&text).unwrap();

        let restored = SharedParams::new();
        loaded.apply_to(&restored);

        // This enumeration is the persistence contract: a new UI setting
        // must be added to this list together with `from_params` and
        // `apply_to`, or restarts will silently drop it.
        assert_eq!(restored.bpm(), source.bpm());
        assert_eq!(restored.volume(), source.volume());
        assert_eq!(restored.beat_count(), source.beat_count());
        assert_eq!(restored.states(), source.states());
        assert_eq!(restored.highlight_first(), source.highlight_first());
        assert_eq!(restored.sample(), source.sample());
    }
}
