#[cfg(not(feature = "ui"))]
use anyhow::Context;

#[cfg(feature = "ui")]
mod app;
mod audio;
mod model;

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "ui")]
    {
        crate::app::launch()
    }

    #[cfg(not(feature = "ui"))]
    {
        run_audio_smoke_test(std::env::args().skip(1).collect())
    }
}

#[cfg(not(feature = "ui"))]
fn run_audio_smoke_test(arguments: Vec<String>) -> anyhow::Result<()> {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let mut engine = audio::engine::AudioEngine::new()?;
    let settings = model::AppSettings::load();
    settings.apply_to(engine.params());

    apply_audio_options(&engine, &arguments)?;
    println!(
        "Takto audio core ready at {} Hz; playing {} BPM for 5 seconds.",
        engine.sample_rate,
        engine.params().bpm()
    );
    engine.play()?;
    std::thread::sleep(Duration::from_secs(5));
    engine.stop();
    engine.params().running.store(false, Ordering::Relaxed);
    Ok(())
}

#[cfg(not(feature = "ui"))]
fn apply_audio_options(
    engine: &audio::engine::AudioEngine,
    arguments: &[String],
) -> anyhow::Result<()> {
    use audio::sound_bank::{SoundBank, SoundPreset};

    let mut preset = SoundPreset::Woodblock;
    let mut normal_path = None;
    let mut accent_path = None;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--preset" => {
                index += 1;
                let requested = arguments.get(index).context("--preset requires a value")?;
                preset = SoundPreset::ALL
                    .into_iter()
                    .find(|candidate| candidate.title().eq_ignore_ascii_case(requested))
                    .with_context(|| {
                        format!(
                            "unknown sound preset {requested}; available: {}",
                            SoundPreset::ALL
                                .iter()
                                .map(|candidate| candidate.title())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })?;
            }
            "--normal" => {
                index += 1;
                normal_path = Some(
                    arguments
                        .get(index)
                        .context("--normal requires a path")?
                        .clone(),
                );
            }
            "--accent" => {
                index += 1;
                accent_path = Some(
                    arguments
                        .get(index)
                        .context("--accent requires a path")?
                        .clone(),
                );
            }
            argument => anyhow::bail!("unknown argument {argument}"),
        }

        index += 1;
    }

    if normal_path.is_some() || accent_path.is_some() {
        let normal_path = normal_path.context("--normal and --accent must be supplied together")?;
        let accent_path = accent_path.context("--normal and --accent must be supplied together")?;
        let bank = SoundBank::from_wav_files(
            std::path::Path::new(&normal_path),
            std::path::Path::new(&accent_path),
            engine.sample_rate,
        )?;
        engine.set_sound_bank(bank);
        println!("Loaded custom WAV sounds: {normal_path}, {accent_path}");
    } else {
        engine.set_sound_bank(SoundBank::preset(preset, engine.sample_rate));
        println!("Sound preset: {}", preset.title());
    }

    Ok(())
}
