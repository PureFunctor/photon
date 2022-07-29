pub mod app;

use std::fs::File;

use anyhow::{bail, Context};
use cpal::traits::{DeviceTrait, HostTrait};
use eframe::egui;
use log::error;
use photon::core::{
    audio::SamplesInMemory,
    engine::{Engine, MessageFromEngine, MessageIntoEngine},
};

fn main() -> anyhow::Result<()> {
    let file = File::open("assets/aragami.mp3")?;
    let samples = SamplesInMemory::try_from_file(file)?;

    if samples.sample_rate != 44100 {
        bail!("Unsupported sample rate {}", samples.sample_rate);
    }

    if samples.channels != 2 {
        bail!("Unsupported channel count {}", samples.channels);
    }

    let (into_engine_p, into_engine_c) = rtrb::RingBuffer::<MessageIntoEngine>::new(8);
    let (from_engine_p, from_engine_c) = rtrb::RingBuffer::<MessageFromEngine>::new(8);
    let mut engine = Engine::new(samples.samples, into_engine_c, from_engine_p);

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("No default output device!")?;
    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(44100),
        buffer_size: cpal::BufferSize::Default,
    };

    let _stream = device.build_output_stream(
        &config,
        move |buffer, _| engine.process(buffer),
        |e| error!("Error in stream: {}", e),
    )?;

    let photon = app::PhotonPlayer::new(into_engine_p, from_engine_c);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Photon",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(photon)
        }),
    );
}
