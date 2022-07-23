use std::fs::File;
use std::sync::mpsc::Sender;

use eframe::{egui, App};
use photon::core::playback::Event;
use photon::core::{audio::SamplesInMemory, playback};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let file = File::open("assets/mobius.mp3")?;
    let samples = SamplesInMemory::try_from_file(file)?;
    let (_stream, sender) = playback::initialize(samples)?;
    let photon = Photon {
        playback_events: sender,
    };

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Photon - Interactive Music Player",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(photon)
        }),
    )
}

struct Photon {
    playback_events: Sender<Event>,
}

impl App for Photon {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Photon - Interactive Music Player");
            ui.add(egui::Separator::default());
            ui.horizontal(|ui| {
                if ui.button("Play").clicked() {
                    self.playback_events.send(Event::Play).unwrap();
                }
                if ui.button("Pause").clicked() {
                    self.playback_events.send(Event::Pause).unwrap();
                }
                if ui.button("Restart").clicked() {
                    self.playback_events.send(Event::Restart).unwrap();
                }
            });
        });
    }
}
