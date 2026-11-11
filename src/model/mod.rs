use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub const MAX_BEATS: u32 = 16;
/// Upper volume multiplier. 1.0 is unity (100%); values above it boost the
/// click through the renderer's soft limiter, mirroring GNOME's
/// over-amplification idea.
pub const MAX_VOLUME: f32 = 2.0;

/// Per-beat playback state. The application exposes two states: `Enabled`
/// beats sound (the plain tick, or the special first-beat sound when the
/// highlight switch is on), `Muted` beats produce no sound at all.
/// `Normal` is retained only to decode saves made by older versions; it
/// maps to `Enabled` everywhere. The serialized spelling of `Enabled` used
/// to be `accent`, so old settings files still load through the serde alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeatState {
    Muted,
    Normal,
    #[serde(alias = "accent")]
    Enabled,
}

pub fn default_states(beats: u32) -> Vec<BeatState> {
    (0..beats).map(|_| BeatState::Enabled).collect()
}

/// One tempo marking and its inclusive BPM range. Ranges follow the modern
/// convention and are contiguous (each range starts right after the previous
/// one ends), so every BPM maps to exactly one marking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempoRange {
    pub name: &'static str,
    pub min_bpm: u32,
    pub max_bpm: u32,
}

impl TempoRange {
    /// Midpoint of the inclusive range. Selecting a marking from the tempo
    /// selector places the tempo at this point.
    pub fn default_bpm(self) -> u32 {
        (self.min_bpm + self.max_bpm) / 2
    }
}

/// The full tempo table, ordered from slowest to fastest. The names are kept
/// in Italian on purpose: they are the international convention and read the
/// same in every interface language.
pub const TEMPO_MARKINGS: [TempoRange; 10] = [
    TempoRange {
        name: "Grave",
        min_bpm: 20,
        max_bpm: 39,
    },
    TempoRange {
        name: "Largo",
        min_bpm: 40,
        max_bpm: 59,
    },
    TempoRange {
        name: "Larghetto",
        min_bpm: 60,
        max_bpm: 65,
    },
    TempoRange {
        name: "Adagio",
        min_bpm: 66,
        max_bpm: 75,
    },
    TempoRange {
        name: "Andante",
        min_bpm: 76,
        max_bpm: 107,
    },
    TempoRange {
        name: "Moderato",
        min_bpm: 108,
        max_bpm: 119,
    },
    TempoRange {
        name: "Allegro",
        min_bpm: 120,
        max_bpm: 155,
    },
    TempoRange {
        name: "Vivace",
        min_bpm: 156,
        max_bpm: 175,
    },
    TempoRange {
        name: "Presto",
        min_bpm: 176,
        max_bpm: 199,
    },
    TempoRange {
        name: "Prestissimo",
        min_bpm: 200,
        max_bpm: 300,
    },
];

/// Traditional Italian tempo marking for the given tempo. Every boundary
/// belongs to the faster marking (76 is Andante, 108 is Moderato, ...);
/// values below the first threshold (defensive: the engine clamps to
/// 20..=300) also read as Grave.
pub fn tempo_marking(bpm: u32) -> &'static str {
    let mut marking = TEMPO_MARKINGS[0].name;
    for range in TEMPO_MARKINGS {
        if bpm < range.min_bpm {
            break;
        }
        marking = range.name;
    }
    marking
}

/// Interface languages exposed by the settings modal. The array order is the
/// persisted atomic index; `English` is deliberately first so first launches
/// and legacy settings files use English by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh")]
    Chinese,
    #[serde(rename = "hi")]
    Hindi,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "ar")]
    Arabic,
    #[serde(rename = "bn")]
    Bengali,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "ur")]
    Urdu,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "pa")]
    Punjabi,
    #[serde(rename = "mr")]
    Marathi,
    #[serde(rename = "te")]
    Telugu,
    #[serde(rename = "tr")]
    Turkish,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "vi")]
    Vietnamese,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "id")]
    Indonesian,
    #[serde(rename = "pl")]
    Polish,
    #[serde(rename = "uk")]
    Ukrainian,
    #[serde(rename = "nl")]
    Dutch,
    #[serde(rename = "th")]
    Thai,
    #[serde(rename = "sw")]
    Swahili,
}

/// `(language, ISO 639-1 code, native name)`. Native names are shown so the
/// selector remains usable when the interface is in an unfamiliar language.
pub const LANGUAGES: [(Language, &str, &str); 25] = [
    (Language::English, "EN", "English"),
    (Language::Chinese, "ZH", "中文"),
    (Language::Hindi, "HI", "हिन्दी"),
    (Language::Spanish, "ES", "Español"),
    (Language::French, "FR", "Français"),
    (Language::Arabic, "AR", "العربية"),
    (Language::Bengali, "BN", "বাংলা"),
    (Language::Portuguese, "PT", "Português"),
    (Language::Russian, "RU", "Русский"),
    (Language::Urdu, "UR", "اردو"),
    (Language::German, "DE", "Deutsch"),
    (Language::Japanese, "JA", "日本語"),
    (Language::Punjabi, "PA", "ਪੰਜਾਬੀ"),
    (Language::Marathi, "MR", "मराठी"),
    (Language::Telugu, "TE", "తెలుగు"),
    (Language::Turkish, "TR", "Türkçe"),
    (Language::Korean, "KO", "한국어"),
    (Language::Vietnamese, "VI", "Tiếng Việt"),
    (Language::Italian, "IT", "Italiano"),
    (Language::Indonesian, "ID", "Bahasa Indonesia"),
    (Language::Polish, "PL", "Polski"),
    (Language::Ukrainian, "UK", "Українська"),
    (Language::Dutch, "NL", "Nederlands"),
    (Language::Thai, "TH", "ไทย"),
    (Language::Swahili, "SW", "Kiswahili"),
];

impl Language {
    pub fn from_index(index: usize) -> Self {
        LANGUAGES[index.min(LANGUAGES.len() - 1)].0
    }

    pub fn index(self) -> usize {
        LANGUAGES
            .iter()
            .position(|(language, _, _)| *language == self)
            .unwrap_or(0)
    }

    pub fn selector_label(self) -> String {
        let (_, code, native_name) = LANGUAGES[self.index()];
        format!("{native_name} ({code})")
    }
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
    /// Meter packed into one value so the audio thread can never observe a
    /// torn pair while the UI switches meter mid-playback: bits 0..15 hold
    /// the active mask, bits 16..31 hold the beat count.
    pattern: AtomicU32,
    pub current_beat: AtomicU32,
    pub preview_generation: AtomicU32,
    /// Number of immediate one-shot clicks requested by the UI.
    pub one_shot_pending: AtomicU32,
    pub highlight_first: AtomicBool,
    /// Selected click sample: index into the candidate bank.
    pub sample: AtomicUsize,
    /// Selected interface language as its position in `LANGUAGES`.
    pub language: AtomicUsize,
    pub sample_rate: AtomicU32,
}

#[allow(dead_code)]
impl SharedParams {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            bpm: AtomicU32::new(140),
            volume: AtomicU32::new(f32::to_bits(0.85)),
            pattern: AtomicU32::new(pack_pattern(4, 0b1111)),
            current_beat: AtomicU32::new(0),
            preview_generation: AtomicU32::new(0),
            one_shot_pending: AtomicU32::new(0),
            highlight_first: AtomicBool::new(true),
            sample: AtomicUsize::new(0),
            language: AtomicUsize::new(Language::default().index()),
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
        f32::from_bits(self.volume.load(Ordering::Relaxed)).clamp(0.0, MAX_VOLUME)
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume.store(
            f32::to_bits(volume.clamp(0.0, MAX_VOLUME)),
            Ordering::Relaxed,
        );
    }

    pub fn beat_count(&self) -> u32 {
        pattern_count(self.pattern.load(Ordering::Relaxed))
    }

    /// Reads the meter as one atomic value: `(active_mask, beat_count)`.
    pub fn pattern(&self) -> (u32, u32) {
        let packed = self.pattern.load(Ordering::Relaxed);
        (packed & PATTERN_MASK, pattern_count(packed))
    }

    pub fn set_states(&self, beats: u32, states: &[BeatState]) {
        let beats = beats.clamp(1, MAX_BEATS);
        let mut active_mask = 0_u32;

        for (index, state) in states.iter().enumerate().take(beats as usize) {
            match state {
                BeatState::Enabled | BeatState::Normal => {
                    active_mask |= 1 << index;
                }
                BeatState::Muted => {}
            }
        }

        self.pattern
            .store(pack_pattern(beats, active_mask), Ordering::Relaxed);
    }

    pub fn states(&self) -> Vec<BeatState> {
        let (active_mask, beats) = self.pattern();

        (0..beats)
            .map(|index| {
                if active_mask & (1 << index) == 0 {
                    BeatState::Muted
                } else {
                    BeatState::Enabled
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

    pub fn language(&self) -> Language {
        Language::from_index(self.language.load(Ordering::SeqCst))
    }

    pub fn set_language(&self, language: Language) {
        self.language.store(language.index(), Ordering::SeqCst);
    }
}

const PATTERN_MASK: u32 = 0xFFFF;
const PATTERN_COUNT_SHIFT: u32 = 16;

fn pack_pattern(count: u32, active_mask: u32) -> u32 {
    ((count & PATTERN_MASK) << PATTERN_COUNT_SHIFT) | (active_mask & PATTERN_MASK)
}

fn pattern_count(packed: u32) -> u32 {
    ((packed >> PATTERN_COUNT_SHIFT) & PATTERN_MASK).clamp(1, MAX_BEATS)
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
    /// Selected click sample position: index into the candidate bank.
    /// `None` comes from older saves and resolves to the first candidate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<usize>,
    /// Interface language. The serde default keeps saves made before this
    /// field existed on English.
    #[serde(default)]
    pub language: Language,
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
            language: Language::default(),
        }
    }
}

fn default_highlight_first() -> bool {
    true
}

impl AppSettings {
    pub fn path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|dir| dir.join("takto").join("settings.json"))
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
            language: params.language(),
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
                        BeatState::Normal => BeatState::Enabled,
                        state => state,
                    })
                    .collect();
            }
        }

        if let Some(accents) = &self.accents {
            return accents.iter().map(|_| BeatState::Enabled).collect();
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
        // Write to a sibling temporary file, flush it to disk, then rename.
        // A crash mid-write therefore never leaves a truncated settings.json
        // behind: readers either see the old or the new complete file.
        let temporary = path.with_extension("json.tmp");
        let mut file = std::fs::File::create(&temporary)?;
        std::io::Write::write_all(&mut file, json.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(temporary, path)?;
        Ok(())
    }

    pub fn apply_to(&self, params: &SharedParams) {
        params.set_bpm(self.bpm);
        params.set_volume(self.volume);
        params.set_states(self.beats, &self.effective_states());
        params.set_highlight_first(self.highlight_first);
        params.set_sample(self.sample.unwrap_or(0));
        params.set_language(self.language);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static CONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn tempo_marking_matches_modern_ranges() {
        // Boundaries belong to the faster marking: [min, max).
        assert_eq!(tempo_marking(20), "Grave");
        assert_eq!(tempo_marking(39), "Grave");
        assert_eq!(tempo_marking(40), "Largo");
        assert_eq!(tempo_marking(60), "Larghetto");
        assert_eq!(tempo_marking(66), "Adagio");
        assert_eq!(tempo_marking(76), "Andante");
        assert_eq!(tempo_marking(107), "Andante");
        assert_eq!(tempo_marking(108), "Moderato");
        assert_eq!(tempo_marking(120), "Allegro");
        assert_eq!(tempo_marking(155), "Allegro");
        assert_eq!(tempo_marking(156), "Vivace");
        assert_eq!(tempo_marking(176), "Presto");
        assert_eq!(tempo_marking(200), "Prestissimo");
        assert_eq!(tempo_marking(300), "Prestissimo");
    }

    #[test]
    fn tempo_marking_covers_the_whole_slider_range_without_gaps() {
        let markings: Vec<&str> = TEMPO_MARKINGS.iter().map(|m| m.name).collect();

        for bpm in 20..=300 {
            assert!(
                markings.contains(&tempo_marking(bpm)),
                "unexpected marking for {bpm} BPM"
            );
        }
    }

    #[test]
    fn tempo_markings_are_contiguous_across_the_slider_range() {
        let mut ranges = TEMPO_MARKINGS.iter();

        let first = ranges.next().expect("the tempo table must not be empty");
        assert_eq!(first.min_bpm, 20);

        let mut previous = *first;
        for range in ranges {
            assert_eq!(
                range.min_bpm,
                previous.max_bpm + 1,
                "gap or overlap between {} and {}",
                previous.name,
                range.name
            );
            previous = *range;
        }

        assert_eq!(previous.max_bpm, 300);
    }

    #[test]
    fn tempo_presets_default_to_the_range_midpoint() {
        let default_bpm = |name: &str| {
            TEMPO_MARKINGS
                .iter()
                .find(|marking| marking.name == name)
                .expect("the tempo table should contain every marking")
                .default_bpm()
        };

        assert_eq!(default_bpm("Grave"), 29);
        assert_eq!(default_bpm("Larghetto"), 62);
        assert_eq!(default_bpm("Andante"), 91);
        assert_eq!(default_bpm("Allegro"), 137);
        assert_eq!(default_bpm("Prestissimo"), 250);
    }

    #[test]
    fn states_are_clamped_and_preserved() {
        let params = SharedParams::new();
        params.set_states(
            3,
            &[
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Normal,
                BeatState::Enabled,
            ],
        );
        let states = params.states();

        assert_eq!(
            states,
            vec![BeatState::Enabled, BeatState::Muted, BeatState::Enabled]
        );
    }

    #[test]
    fn pattern_reads_count_and_mask_as_one_atomic_value() {
        let params = SharedParams::new();
        params.set_states(
            6,
            &[
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Muted,
            ],
        );

        // The audio thread must never observe the new beat count together
        // with the old active mask, so both values come from one atomic.
        let (mask, count) = params.pattern();
        assert_eq!(count, 6);
        assert_eq!(mask, 0b010101);
        assert_eq!(
            params.states(),
            vec![
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Muted,
            ]
        );
    }

    #[test]
    fn out_of_range_saved_settings_are_clamped() {
        let settings = AppSettings {
            bpm: 0,
            volume: 5.0,
            beats: 999,
            states: None,
            accents: None,
            highlight_first: true,
            sample: Some(99_999),
            language: Language::English,
        };
        let params = SharedParams::new();
        settings.apply_to(&params);

        assert_eq!(params.bpm(), 20);
        assert_eq!(params.volume(), MAX_VOLUME);
        assert_eq!(params.beat_count(), 16);
        assert_eq!(params.states(), default_states(16));
        // The sample index is restored raw; the UI's load_sample falls back
        // to the first candidate when the candidate index is out of range.
        assert_eq!(params.sample(), 99_999);
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temporary_file() {
        let _config_env = CONFIG_ENV_LOCK.lock().unwrap();
        let config_root =
            std::env::temp_dir().join(format!("takto-config-test-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &config_root);
        std::fs::create_dir_all(&config_root).unwrap();

        let settings = AppSettings::default();
        settings.save().unwrap();

        let path = config_root.join("takto").join("settings.json");
        let saved_text = std::fs::read_to_string(&path).unwrap();
        let loaded: AppSettings = serde_json::from_str(&saved_text).unwrap();
        assert_eq!(loaded, settings);
        assert!(!path.with_extension("json.tmp").exists());

        std::fs::remove_dir_all(&config_root).unwrap();
    }

    #[test]
    fn settings_round_trip() {
        let settings = AppSettings {
            bpm: 183,
            volume: 0.42,
            beats: 3,
            states: Some(vec![
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Normal,
            ]),
            accents: None,
            highlight_first: false,
            sample: Some(2),
            language: Language::Russian,
        };
        let text = serde_json::to_string(&settings).unwrap();
        let loaded: AppSettings = serde_json::from_str(&text).unwrap();
        // New saves spell the enabled state "enabled"; the old "accent"
        // spelling is only read back, never written.
        assert!(text.contains("\"enabled\""));
        assert!(!text.contains("\"accent\""));

        assert_eq!(loaded.bpm, 183);
        assert_eq!(loaded.volume, 0.42);
        assert_eq!(loaded.states, settings.states);
        assert_eq!(loaded.highlight_first, settings.highlight_first);
        assert_eq!(loaded.sample, settings.sample);
        assert_eq!(loaded.language, Language::Russian);
    }

    #[test]
    fn legacy_accent_state_names_still_load() {
        let text = r#"{
            "bpm": 140,
            "volume": 0.8,
            "beats": 4,
            "states": ["accent", "muted", "accent", "muted"],
            "highlight_first": true
        }"#;
        let loaded: AppSettings = serde_json::from_str(text).unwrap();

        // Settings written before the language existed stay on the first-open
        // default instead of changing the user's interface unexpectedly.
        assert_eq!(loaded.language, Language::English);

        assert_eq!(
            loaded.effective_states(),
            vec![
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Muted
            ]
        );
    }

    #[test]
    fn default_settings_are_enabled() {
        let settings = AppSettings::default();

        assert_eq!(
            settings.effective_states(),
            vec![
                BeatState::Enabled,
                BeatState::Enabled,
                BeatState::Enabled,
                BeatState::Enabled
            ]
        );
        assert!(settings.highlight_first);
        assert_eq!(settings.language, Language::English);
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
            language: Language::English,
        };

        assert_eq!(
            settings.effective_states(),
            vec![
                BeatState::Enabled,
                BeatState::Enabled,
                BeatState::Enabled,
                BeatState::Enabled
            ]
        );
    }

    #[test]
    fn older_normal_states_are_exposed_as_enabled() {
        let settings = AppSettings {
            bpm: 140,
            volume: 0.8,
            beats: 3,
            states: Some(vec![
                BeatState::Enabled,
                BeatState::Normal,
                BeatState::Muted,
            ]),
            accents: None,
            highlight_first: true,
            sample: None,
            language: Language::English,
        };

        assert_eq!(
            settings.effective_states(),
            vec![BeatState::Enabled, BeatState::Enabled, BeatState::Muted]
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
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Enabled,
            ],
        );
        source.set_highlight_first(false);
        source.set_sample(4);
        source.set_language(Language::Ukrainian);

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
        assert_eq!(restored.language(), source.language());
    }

    #[test]
    fn language_selector_labels_use_native_names_and_codes() {
        assert_eq!(Language::English.selector_label(), "English (EN)");
        assert_eq!(Language::Russian.selector_label(), "Русский (RU)");
        assert!(LANGUAGES.len() >= 20);

        for (index, (language, _, _)) in LANGUAGES.iter().enumerate() {
            assert_eq!(language.index(), index);
        }
    }
}
