use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
    Mutex, MutexGuard,
};

pub const MAX_BEATS: u32 = 16;
/// Upper volume multiplier. 1.0 is unity (100%); values above it boost the
/// click through the renderer's soft limiter, mirroring GNOME's
/// over-amplification idea.
pub const MAX_VOLUME: f32 = 2.0;

/// Built-in meters, always available in the sizes dialog. The first number
/// is the beat count, the second the written beat unit.
pub const PRESET_METERS: [(u32, u32); 4] = [(2, 4), (3, 4), (4, 4), (6, 8)];

/// Sanity cap on the user-defined meter list.
pub const MAX_CUSTOM_METERS: usize = 8;

/// How many meter chips the main window shows at once.
pub const MAX_VISIBLE_METERS: usize = 4;

/// Beat units a custom meter may use, indexed by the packed code stored in
/// the pattern atomic. The unit is purely the written signature: BPM always
/// means the rate of each tick (see README "How it works").
pub const METER_DENOMINATORS: [u32; 4] = [2, 4, 8, 16];

/// Packed-code fallback for unsupported units (e.g. values from corrupt
/// saves): 4, the most common written unit.
const DEFAULT_DENOMINATOR_CODE: u32 = 1;

pub fn meter_label(beats: u32, denominator: u32) -> String {
    format!("{beats}/{denominator}")
}

pub fn is_preset_meter(beats: u32, denominator: u32) -> bool {
    PRESET_METERS.contains(&(beats, denominator))
}

/// Maps a written unit onto its packed code; unsupported values degrade to
/// the default unit instead of corrupting the atomic.
pub fn denominator_code(denominator: u32) -> u32 {
    METER_DENOMINATORS
        .iter()
        .position(|&unit| unit == denominator)
        .unwrap_or(DEFAULT_DENOMINATOR_CODE as usize) as u32
}

pub fn denominator_from_code(code: u32) -> u32 {
    METER_DENOMINATORS
        .get(code as usize)
        .copied()
        .unwrap_or(METER_DENOMINATORS[DEFAULT_DENOMINATOR_CODE as usize])
}

/// Clamps an arbitrary unit onto the supported set so list entries loaded
/// from disk can never hold an unwritable denominator.
pub fn normalize_denominator(denominator: u32) -> u32 {
    denominator_from_code(denominator_code(denominator))
}

/// Outcome of adding a user-defined meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterChange {
    /// The meter was appended to the custom list.
    Added,
    /// The meter already exists as a preset or custom entry.
    Exists,
    /// The custom list is at its cap.
    Full,
}

/// Outcome of rewriting an existing custom meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterUpdate {
    Updated,
    /// The new value collides with a preset or another custom meter.
    Duplicate,
    /// The index is stale (the entry was removed elsewhere).
    NotFound,
}

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
    /// User-defined meters as `(beats, denominator)` pairs. UI-only state:
    /// the audio thread never reads it, but it lives here so the persistence
    /// snapshot covers it like every other setting.
    custom_meters: Mutex<Vec<(u32, u32)>>,
    /// Meters shown as chips in the main window, in display order. Capped at
    /// `MAX_VISIBLE_METERS`; the active meter must always be a member.
    visible_meters: Mutex<Vec<(u32, u32)>>,
}

#[allow(dead_code)]
impl SharedParams {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            bpm: AtomicU32::new(140),
            volume: AtomicU32::new(f32::to_bits(0.85)),
            pattern: AtomicU32::new(pack_pattern(4, DEFAULT_DENOMINATOR_CODE, 0b1111)),
            current_beat: AtomicU32::new(0),
            preview_generation: AtomicU32::new(0),
            one_shot_pending: AtomicU32::new(0),
            highlight_first: AtomicBool::new(true),
            sample: AtomicUsize::new(0),
            language: AtomicUsize::new(Language::default().index()),
            sample_rate: AtomicU32::new(48_000),
            custom_meters: Mutex::new(Vec::new()),
            visible_meters: Mutex::new(PRESET_METERS.to_vec()),
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

    /// Reads the meter as one atomic value: `(beats, denominator)`.
    pub fn meter(&self) -> (u32, u32) {
        let packed = self.pattern.load(Ordering::Relaxed);
        (
            pattern_count(packed),
            denominator_from_code(pattern_denominator_code(packed)),
        )
    }

    /// Low-level read for the audio thread: `(active_mask, beat_count)`.
    pub fn pattern(&self) -> (u32, u32) {
        let packed = self.pattern.load(Ordering::Relaxed);
        (packed & PATTERN_MASK, pattern_count(packed))
    }

    pub fn set_meter(&self, beats: u32, denominator: u32) {
        let packed = self.pattern.load(Ordering::Relaxed);
        let beats = beats.clamp(1, MAX_BEATS);
        // Bits beyond the new beat count would be invisible to the scheduler,
        // but clearing them keeps the atomic self-describing.
        let active_mask = (packed & PATTERN_MASK) & ((1_u32 << beats) - 1);
        self.pattern.store(
            pack_pattern(beats, denominator_code(denominator), active_mask),
            Ordering::Relaxed,
        );
    }

    /// One atomic switch to a new meter with a fresh pattern, so the audio
    /// thread can never observe a beat count from one meter with a mask from
    /// another.
    pub fn set_meter_and_states(&self, beats: u32, denominator: u32, states: &[BeatState]) {
        self.pattern.store(
            pack_pattern(
                beats,
                denominator_code(denominator),
                states_to_mask(beats, states),
            ),
            Ordering::Relaxed,
        );
    }

    /// Rewrites the active mask while keeping the current meter. Used by the
    /// beat toggles, which never change the meter.
    pub fn set_states(&self, beats: u32, states: &[BeatState]) {
        let denominator = pattern_denominator_code(self.pattern.load(Ordering::Relaxed));
        self.pattern.store(
            pack_pattern(beats, denominator, states_to_mask(beats, states)),
            Ordering::Relaxed,
        );
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

    /// Poisoning only happens on a panic while holding the guard; recovering
    /// the (still structurally valid) list beats crashing the UI thread.
    fn lock_meters<'a>(&self, list: &'a Mutex<Vec<(u32, u32)>>) -> MutexGuard<'a, Vec<(u32, u32)>> {
        list.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether the pair is a preset or a user-defined meter.
    pub fn known_meter(&self, beats: u32, denominator: u32) -> bool {
        is_preset_meter(beats, denominator) || self.custom_meters().contains(&(beats, denominator))
    }

    pub fn custom_meters(&self) -> Vec<(u32, u32)> {
        self.lock_meters(&self.custom_meters).clone()
    }

    /// Meters shown as chips, in display order.
    pub fn visible_meters(&self) -> Vec<(u32, u32)> {
        self.lock_meters(&self.visible_meters).clone()
    }

    /// Enables or disables one chip. Returns `false` (and changes nothing)
    /// when the meter is unknown or the row is already full.
    pub fn set_meter_visible(&self, beats: u32, denominator: u32, visible: bool) -> bool {
        if !self.known_meter(beats, denominator) {
            return false;
        }

        let mut visible_meters = self.lock_meters(&self.visible_meters);
        if visible {
            if visible_meters.contains(&(beats, denominator)) {
                return true;
            }
            if visible_meters.len() >= MAX_VISIBLE_METERS {
                return false;
            }
            visible_meters.push((beats, denominator));
            true
        } else {
            visible_meters.retain(|&meter| meter != (beats, denominator));
            true
        }
    }

    /// Reorders the visible chips (`remove(from)` + `insert(to)` semantics).
    pub fn move_visible_meter(&self, from: usize, to: usize) {
        let mut visible_meters = self.lock_meters(&self.visible_meters);
        if from >= visible_meters.len() || to >= visible_meters.len() || from == to {
            return;
        }
        let meter = visible_meters.remove(from);
        visible_meters.insert(to, meter);
    }

    /// Replaces the visible list, dropping unknown meters and duplicates and
    /// enforcing the cap. Used by both the sizes dialog and the load path.
    pub fn set_visible_meters(&self, meters: &[(u32, u32)]) {
        let mut kept: Vec<(u32, u32)> = Vec::with_capacity(MAX_VISIBLE_METERS);
        for &(beats, denominator) in meters {
            if self.known_meter(beats, denominator) && !kept.contains(&(beats, denominator)) {
                kept.push((beats, denominator));
            }
            if kept.len() == MAX_VISIBLE_METERS {
                break;
            }
        }
        *self.lock_meters(&self.visible_meters) = kept;
    }

    /// Replaces the custom list, dropping preset duplicates and normalizing
    /// values. Used by the load path; the dialog goes through `add`/`update`.
    pub fn set_custom_meters(&self, meters: &[(u32, u32)]) {
        let mut kept: Vec<(u32, u32)> = Vec::with_capacity(MAX_CUSTOM_METERS);
        for &(beats, denominator) in meters {
            let beats = beats.clamp(1, MAX_BEATS);
            let denominator = normalize_denominator(denominator);
            if !is_preset_meter(beats, denominator) && !kept.contains(&(beats, denominator)) {
                kept.push((beats, denominator));
            }
            if kept.len() == MAX_CUSTOM_METERS {
                break;
            }
        }
        *self.lock_meters(&self.custom_meters) = kept;
        // The visible list may now reference dropped meters; re-sanitize it.
        let visible = self.visible_meters();
        self.set_visible_meters(&visible);
    }

    /// Adds a user-defined meter. Existing presets and duplicates keep the
    /// list clean so the dialog can just enable their visibility instead.
    pub fn add_custom_meter(&self, beats: u32, denominator: u32) -> MeterChange {
        let beats = beats.clamp(1, MAX_BEATS);
        let denominator = normalize_denominator(denominator);
        let mut custom_meters = self.lock_meters(&self.custom_meters);
        if is_preset_meter(beats, denominator) || custom_meters.contains(&(beats, denominator)) {
            return MeterChange::Exists;
        }
        if custom_meters.len() >= MAX_CUSTOM_METERS {
            return MeterChange::Full;
        }
        custom_meters.push((beats, denominator));
        MeterChange::Added
    }

    /// Rewrites one custom meter. The visible chip keeps its position; the
    /// active meter follows its chip so the audio pattern always matches a
    /// meter that still exists.
    pub fn update_custom_meter(&self, index: usize, beats: u32, denominator: u32) -> MeterUpdate {
        let beats = beats.clamp(1, MAX_BEATS);
        let denominator = normalize_denominator(denominator);
        let old = {
            let mut custom_meters = self.lock_meters(&self.custom_meters);
            if index >= custom_meters.len() {
                return MeterUpdate::NotFound;
            }
            let current = custom_meters[index];
            if current == (beats, denominator) {
                return MeterUpdate::Updated;
            }
            if is_preset_meter(beats, denominator)
                || custom_meters
                    .iter()
                    .enumerate()
                    .any(|(other, &meter)| other != index && meter == (beats, denominator))
            {
                return MeterUpdate::Duplicate;
            }
            custom_meters[index] = (beats, denominator);
            current
        };

        let mut visible_meters = self.lock_meters(&self.visible_meters);
        if let Some(position) = visible_meters.iter().position(|&meter| meter == old) {
            visible_meters[position] = (beats, denominator);
        }
        drop(visible_meters);

        if self.meter() == old {
            self.set_meter(beats, denominator);
        }
        MeterUpdate::Updated
    }

    /// Removes a custom meter (and its chip). Returns the removed pair so the
    /// dialog can pick a fallback when it was the active meter.
    pub fn remove_custom_meter(&self, index: usize) -> Option<(u32, u32)> {
        let removed = {
            let mut custom_meters = self.lock_meters(&self.custom_meters);
            if index >= custom_meters.len() {
                return None;
            }
            custom_meters.remove(index)
        };
        self.lock_meters(&self.visible_meters)
            .retain(|&meter| meter != removed);
        Some(removed)
    }

    /// Whether `(beats, denominator)` would collide with a preset or another
    /// custom meter. `skip_index` excludes one custom entry (the one being
    /// edited).
    pub fn meter_conflicts(&self, skip_index: Option<usize>, beats: u32, denominator: u32) -> bool {
        let beats = beats.clamp(1, MAX_BEATS);
        let denominator = normalize_denominator(denominator);
        if is_preset_meter(beats, denominator) {
            return true;
        }
        self.custom_meters()
            .into_iter()
            .enumerate()
            .any(|(index, meter)| Some(index) != skip_index && meter == (beats, denominator))
    }
}

const PATTERN_MASK: u32 = 0xFFFF;
const PATTERN_COUNT_SHIFT: u32 = 16;
const PATTERN_COUNT_MASK: u32 = 0x1F;
const PATTERN_DENOMINATOR_SHIFT: u32 = 21;
const PATTERN_DENOMINATOR_MASK: u32 = 0x7;

fn pack_pattern(count: u32, denominator_code: u32, active_mask: u32) -> u32 {
    (count.clamp(1, MAX_BEATS) << PATTERN_COUNT_SHIFT)
        | ((denominator_code & PATTERN_DENOMINATOR_MASK) << PATTERN_DENOMINATOR_SHIFT)
        | (active_mask & PATTERN_MASK)
}

fn pattern_count(packed: u32) -> u32 {
    ((packed >> PATTERN_COUNT_SHIFT) & PATTERN_COUNT_MASK).clamp(1, MAX_BEATS)
}

fn pattern_denominator_code(packed: u32) -> u32 {
    (packed >> PATTERN_DENOMINATOR_SHIFT) & PATTERN_DENOMINATOR_MASK
}

fn states_to_mask(beats: u32, states: &[BeatState]) -> u32 {
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

    active_mask
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
    /// Written beat unit of the active meter. `None` comes from saves made
    /// before custom meters existed; `beats == 6` meant 6/8 back then, so the
    /// load path resolves it to 8 instead of silently rewriting it as 6/4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denominator: Option<u32>,
    /// User-defined meters as `(beats, denominator)` pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_signatures: Option<Vec<(u32, u32)>>,
    /// Meters shown as chips in the main window, in display order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_meters: Option<Vec<(u32, u32)>>,
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
            denominator: Some(4),
            custom_signatures: Some(Vec::new()),
            visible_meters: Some(PRESET_METERS.to_vec()),
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
        let (beats, denominator) = params.meter();
        Self {
            bpm: params.bpm(),
            volume: params.volume(),
            beats,
            denominator: Some(denominator),
            custom_signatures: Some(params.custom_meters()),
            visible_meters: Some(params.visible_meters()),
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
        params.set_custom_meters(self.custom_signatures.as_deref().unwrap_or(&[]));
        // Saves made before custom meters existed have no chip list: show the
        // four presets instead of an empty bar. A corrupt list that
        // sanitization empties falls back the same way.
        let visible_meters = self.visible_meters.as_deref().unwrap_or(&PRESET_METERS);
        params.set_visible_meters(visible_meters);
        if params.visible_meters().is_empty() {
            params.set_visible_meters(&PRESET_METERS);
        }
        let denominator = self
            .denominator
            .unwrap_or(if self.beats == 6 { 8 } else { 4 });
        params.set_meter_and_states(self.beats, denominator, &self.effective_states());
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
            denominator: None,
            custom_signatures: None,
            visible_meters: None,
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
            denominator: Some(4),
            custom_signatures: Some(Vec::new()),
            visible_meters: Some(PRESET_METERS.to_vec()),
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
            denominator: None,
            custom_signatures: None,
            visible_meters: None,
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
            denominator: None,
            custom_signatures: None,
            visible_meters: None,
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
        source.set_meter(6, 8);
        source.set_highlight_first(false);
        source.set_sample(4);
        source.set_language(Language::Ukrainian);
        assert_eq!(source.add_custom_meter(7, 8), MeterChange::Added);
        assert_eq!(source.add_custom_meter(5, 4), MeterChange::Added);
        source.set_visible_meters(&[(7, 8), (4, 4), (3, 4), (2, 4)]);
        source.move_visible_meter(0, 2);

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
        assert_eq!(restored.meter(), source.meter());
        assert_eq!(restored.states(), source.states());
        assert_eq!(restored.custom_meters(), source.custom_meters());
        assert_eq!(restored.visible_meters(), source.visible_meters());
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

    #[test]
    fn meter_packing_round_trips_beats_denominator_and_states() {
        let params = SharedParams::new();
        params.set_meter_and_states(
            7,
            8,
            &[
                BeatState::Enabled,
                BeatState::Muted,
                BeatState::Enabled,
                BeatState::Enabled,
                BeatState::Enabled,
                BeatState::Enabled,
                BeatState::Muted,
            ],
        );

        assert_eq!(params.meter(), (7, 8));
        assert_eq!(params.beat_count(), 7);
        assert_eq!(params.pattern(), (0b0111101, 7));

        // The beat toggles rewrite the mask without touching the meter, so
        // 6/4 and 6/8 stay distinguishable chips.
        params.set_states(7, &default_states(7));
        assert_eq!(params.meter(), (7, 8));
        assert_eq!(params.states(), default_states(7));
    }

    #[test]
    fn meter_values_are_clamped() {
        let params = SharedParams::new();
        params.set_meter(0, 5);
        assert_eq!(params.meter(), (1, 4));

        params.set_meter(MAX_BEATS + 10, 16);
        assert_eq!(params.meter(), (MAX_BEATS, 16));

        assert_eq!(normalize_denominator(3), 4);
        assert_eq!(denominator_from_code(99), 4);
    }

    #[test]
    fn legacy_saves_without_denominator_keep_six_eighth() {
        // Old saves stored only a beat count: a 6 was 6/8, everything else
        // read as quarter notes.
        let text = r#"{"bpm": 140, "volume": 0.8, "beats": 6, "highlight_first": true}"#;
        let loaded: AppSettings = serde_json::from_str(text).unwrap();
        let params = SharedParams::new();
        loaded.apply_to(&params);
        assert_eq!(params.meter(), (6, 8));

        let text = r#"{"bpm": 140, "volume": 0.8, "beats": 4, "highlight_first": true}"#;
        let loaded: AppSettings = serde_json::from_str(text).unwrap();
        let params = SharedParams::new();
        loaded.apply_to(&params);
        assert_eq!(params.meter(), (4, 4));
    }

    #[test]
    fn legacy_saves_show_the_default_preset_chips() {
        let text = r#"{"bpm": 140, "volume": 0.8, "beats": 4, "highlight_first": true}"#;
        let loaded: AppSettings = serde_json::from_str(text).unwrap();
        let params = SharedParams::new();
        loaded.apply_to(&params);

        assert_eq!(params.visible_meters(), PRESET_METERS.to_vec());
        assert_eq!(params.meter(), (4, 4));
        assert!(params.custom_meters().is_empty());
    }

    #[test]
    fn corrupt_visible_lists_fall_back_to_presets() {
        // Both entries are unknown meters, so sanitization empties the bar.
        let text = r#"{
            "bpm": 140,
            "volume": 0.8,
            "beats": 4,
            "highlight_first": true,
            "visible_meters": [[9, 9], [8, 3]]
        }"#;
        let loaded: AppSettings = serde_json::from_str(text).unwrap();
        let params = SharedParams::new();
        loaded.apply_to(&params);

        assert_eq!(params.visible_meters(), PRESET_METERS.to_vec());
    }

    #[test]
    fn custom_meter_list_dedupes_and_caps() {
        let params = SharedParams::new();

        // Presets and duplicates never enter the list.
        assert_eq!(params.add_custom_meter(4, 4), MeterChange::Exists);
        assert_eq!(params.add_custom_meter(7, 8), MeterChange::Added);
        assert_eq!(params.add_custom_meter(7, 8), MeterChange::Exists);
        assert_eq!(params.custom_meters(), vec![(7, 8)]);

        // Invalid units normalize onto the supported set.
        assert_eq!(params.add_custom_meter(9, 5), MeterChange::Added);
        assert!(params.custom_meters().contains(&(9, 4)));

        while params.custom_meters().len() < MAX_CUSTOM_METERS {
            let beats = (params.custom_meters().len() + 8) as u32;
            assert_eq!(params.add_custom_meter(beats, 8), MeterChange::Added);
        }
        assert_eq!(params.add_custom_meter(16, 8), MeterChange::Full);
        assert_eq!(params.custom_meters().len(), MAX_CUSTOM_METERS);
    }

    #[test]
    fn update_custom_meter_rejects_duplicates_and_follows_the_active_meter() {
        let params = SharedParams::new();
        assert_eq!(params.add_custom_meter(7, 8), MeterChange::Added);
        assert_eq!(params.add_custom_meter(9, 8), MeterChange::Added);
        params.set_meter_and_states(7, 8, &default_states(7));

        // Colliding with a preset or another custom entry is rejected so the
        // dialog can keep Save disabled instead of guessing a merge.
        assert_eq!(params.update_custom_meter(0, 4, 4), MeterUpdate::Duplicate);
        assert_eq!(params.update_custom_meter(0, 9, 8), MeterUpdate::Duplicate);
        assert_eq!(params.update_custom_meter(9, 8, 8), MeterUpdate::NotFound);

        assert_eq!(params.update_custom_meter(0, 5, 4), MeterUpdate::Updated);
        assert_eq!(params.custom_meters()[0], (5, 4));
        // The chip label followed the edit while it was the active meter.
        assert_eq!(params.meter(), (5, 4));
        assert_eq!(params.beat_count(), 5);

        // Rewriting a meter to its own value is a valid no-op.
        assert_eq!(params.update_custom_meter(0, 5, 4), MeterUpdate::Updated);
    }

    #[test]
    fn remove_custom_meter_cascades_visibility() {
        let params = SharedParams::new();
        assert_eq!(params.add_custom_meter(7, 8), MeterChange::Added);
        // Free a visible slot first: the default row is full with presets.
        params.set_visible_meters(&[(4, 4), (3, 4), (2, 4)]);
        assert!(params.set_meter_visible(7, 8, true));
        assert!(params.visible_meters().contains(&(7, 8)));

        assert_eq!(params.remove_custom_meter(0), Some((7, 8)));
        assert_eq!(params.remove_custom_meter(0), None);
        assert!(!params.visible_meters().contains(&(7, 8)));
    }

    #[test]
    fn visible_meters_are_validated_capped_and_ordered() {
        let params = SharedParams::new();
        assert_eq!(params.add_custom_meter(7, 8), MeterChange::Added);

        // Unknown meters and duplicates are dropped, order is preserved.
        params.set_visible_meters(&[(7, 8), (9, 8), (4, 4), (4, 4), (3, 4), (2, 4), (6, 8)]);
        assert_eq!(
            params.visible_meters(),
            vec![(7, 8), (4, 4), (3, 4), (2, 4)]
        );

        params.move_visible_meter(3, 0);
        assert_eq!(
            params.visible_meters(),
            vec![(2, 4), (7, 8), (4, 4), (3, 4)]
        );

        // Disabling frees the slot for another chip.
        assert!(params.set_meter_visible(2, 4, false));
        assert!(params.set_meter_visible(6, 8, true));
        assert_eq!(params.visible_meters().last(), Some(&(6, 8)));
        assert!(!params.set_meter_visible(1, 1, true));
    }
}
