use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const MAX_BEATS: u32 = 16;

#[derive(Debug)]
pub struct SharedParams {
    pub running: AtomicBool,
    pub bpm: AtomicU32,
    pub volume: AtomicU32,
    pub beat_count: AtomicU32,
    pub accent_mask: AtomicU32,
    pub current_beat: AtomicU32,
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
            accent_mask: AtomicU32::new(0b0001),
            current_beat: AtomicU32::new(0),
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

    pub fn set_pattern(&self, beats: u32, accents: &[bool]) {
        let beats = beats.clamp(1, MAX_BEATS);
        let mut mask = 0_u32;

        for (index, accented) in accents.iter().enumerate().take(beats as usize) {
            if *accented {
                mask |= 1 << index;
            }
        }

        self.beat_count.store(beats, Ordering::Relaxed);
        self.accent_mask.store(mask, Ordering::Relaxed);
    }

    pub fn pattern(&self) -> (u32, Vec<bool>) {
        let beats = self.beat_count();
        let mask = self.accent_mask.load(Ordering::Relaxed);
        let accents = (0..beats).map(|index| mask & (1 << index) != 0).collect();

        (beats, accents)
    }

    pub fn current_beat(&self) -> u32 {
        self.current_beat.load(Ordering::Relaxed)
    }
}

impl Default for SharedParams {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub bpm: u32,
    pub volume: f32,
    pub beats: u32,
    pub accents: Vec<bool>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bpm: 140,
            volume: 0.85,
            beats: 4,
            accents: vec![true, false, false, false],
        }
    }
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
        params.set_pattern(self.beats, &self.accents);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_is_clamped_and_preserved() {
        let params = SharedParams::new();
        params.set_pattern(3, &[true, false, true, true, true]);
        let (beats, accents) = params.pattern();

        assert_eq!(beats, 3);
        assert_eq!(accents, vec![true, false, true]);
    }

    #[test]
    fn settings_round_trip() {
        let settings = AppSettings {
            bpm: 183,
            volume: 0.42,
            beats: 3,
            accents: vec![true, false, false],
        };
        let text = serde_json::to_string(&settings).unwrap();
        let loaded: AppSettings = serde_json::from_str(&text).unwrap();

        assert_eq!(loaded.bpm, 183);
        assert_eq!(loaded.volume, 0.42);
        assert_eq!(loaded.accents, settings.accents);
    }
}
