use std::fs::File;

use anyhow::{bail, Context};
use cpal::traits::{DeviceTrait, HostTrait};
use eframe::{egui, App};
use log::{error, info};
use photon::core::{
    audio::SamplesInMemory,
    engine::{Engine, MessageFromEngine, MessageIntoEngine},
};
use rtrb::{Consumer, Producer};

fn main() -> anyhow::Result<()> {
    let file = File::open("assets/erin.flac")?;
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

    let photon = Photon::new(into_engine_p, from_engine_c);
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

/// The application struct.
struct Photon {
    into_engine: Producer<MessageIntoEngine>,
    from_engine: Consumer<MessageFromEngine>,
}

impl Photon {
    fn new(
        into_engine: Producer<MessageIntoEngine>,
        from_engine: Consumer<MessageFromEngine>,
    ) -> Self {
        Self {
            into_engine,
            from_engine,
        }
    }
}

impl App for Photon {
    fn update(&mut self, ctx: &eframe::egui::Context, _: &mut eframe::Frame) {
        while let Ok(_message) = self.from_engine.pop() {
            continue;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("photon - interactive music player");
            ui.separator();
            if ui.button("Play").clicked() {
                info!("Sending play signal to engine...");
                self.into_engine.push(MessageIntoEngine::Play).unwrap();
            };
            if ui.button("Pause").clicked() {
                info!("Sending pause signal to engine...");
                self.into_engine.push(MessageIntoEngine::Pause).unwrap();
            };
        });
    }
}
