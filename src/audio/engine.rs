use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{anyhow, Context, Result};
use arc_swap::ArcSwap;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};

use super::scheduler::{OutputSample, Scheduler};
use super::sound_bank::SoundBank;
use crate::model::SharedParams;

pub struct AudioEngine {
    #[allow(dead_code)]
    stream: Stream,
    params: Arc<SharedParams>,
    sound_bank: Arc<ArcSwap<SoundBank>>,
    started: Arc<AtomicBool>,
    pub sample_rate: u32,
}

#[allow(dead_code)]
impl AudioEngine {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default audio output device")?;
        let supported = device
            .default_output_config()
            .context("default output configuration is unavailable")?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let sample_rate = config.sample_rate.0;
        let params = Arc::new(SharedParams::new());
        params.sample_rate.store(sample_rate, Ordering::Relaxed);
        let bank = Arc::new(ArcSwap::new(Arc::new(SoundBank::synthetic(sample_rate))));
        let started = Arc::new(AtomicBool::new(false));
        let error: fn(cpal::StreamError) = audio_stream_error;

        let stream = match sample_format {
            SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config, params.clone(), bank.clone(), error)?
            }
            SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config, params.clone(), bank.clone(), error)?
            }
            SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &config, params.clone(), bank.clone(), error)?
            }
            format => return Err(anyhow!("unsupported output sample format: {format}")),
        };

        Ok(Self {
            stream,
            params,
            sound_bank: bank,
            started,
            sample_rate,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn build_stream<T>(
        device: &cpal::Device,
        config: &StreamConfig,
        params: Arc<SharedParams>,
        bank: Arc<ArcSwap<SoundBank>>,
        error: impl FnMut(cpal::StreamError) + Send + 'static,
    ) -> Result<Stream>
    where
        T: cpal::SizedSample + OutputSample + 'static,
    {
        let mut scheduler = Scheduler::new(0, params.beat_count());
        let channels = config.channels.max(1);

        let stream = device.build_output_stream(
            config,
            move |output: &mut [T], _| {
                let bank = bank.load();
                scheduler.render(&params, &bank, output, channels.into());
            },
            error,
            None,
        )?;

        Ok(stream)
    }

    pub fn params(&self) -> &SharedParams {
        &self.params
    }

    pub fn shared_params(&self) -> Arc<SharedParams> {
        self.params.clone()
    }

    pub fn set_sound_bank(&self, bank: SoundBank) {
        self.sound_bank.store(Arc::new(bank));
    }

    pub fn set_normal_wav(&self, path: &std::path::Path) -> Result<()> {
        let mut bank = self.sound_bank.load().as_ref().clone();
        bank.normal = super::sound_bank::decode_wav(path, self.sample_rate)?;
        self.sound_bank.store(Arc::new(bank));
        Ok(())
    }

    pub fn set_accent_wav(&self, path: &std::path::Path) -> Result<()> {
        let mut bank = self.sound_bank.load().as_ref().clone();
        bank.accent = super::sound_bank::decode_wav(path, self.sample_rate)?;
        self.sound_bank.store(Arc::new(bank));
        Ok(())
    }

    pub fn play(&mut self) -> Result<()> {
        if !self.started.swap(true, Ordering::SeqCst) {
            self.stream.play()?;
        }

        self.params.running.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.params.running.store(false, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.params.running.load(Ordering::Relaxed)
    }
}

fn audio_stream_error(error: cpal::StreamError) {
    eprintln!("audio stream error: {error}");
}
