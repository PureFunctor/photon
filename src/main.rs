use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    sync::atomic::{self, AtomicBool},
};

use anyhow::{bail, Context};
use cpal::traits::{DeviceTrait, HostTrait};
use eframe::{egui, App};
use enum_iterator::{all, Sequence};
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

struct Photon {
    into_engine: Producer<MessageIntoEngine>,
    from_engine: Consumer<MessageFromEngine>,
    retrigger: HashMap<Note, AtomicBool>,
    trance_gate: HashMap<Note, AtomicBool>,
}

impl Photon {
    fn new(
        into_engine: Producer<MessageIntoEngine>,
        from_engine: Consumer<MessageFromEngine>,
    ) -> Self {
        let retrigger = HashMap::from_iter(
            all::<Note>().zip(std::iter::repeat_with(|| AtomicBool::new(false))),
        );
        let trance_gate = HashMap::from_iter(
            all::<Note>().zip(std::iter::repeat_with(|| AtomicBool::new(false))),
        );
        Self {
            into_engine,
            from_engine,
            retrigger,
            trance_gate,
        }
    }
}

impl App for Photon {
    fn update(&mut self, ctx: &eframe::egui::Context, _: &mut eframe::Frame) {
        let beats_per_minute = 188.0;
        while let Ok(_message) = self.from_engine.pop() {
            continue;
        }
        let retrigger_pairs = &[
            (egui::Key::Q, Note::Fourth),
            (egui::Key::W, Note::Eighth),
            (egui::Key::E, Note::Sixteenth),
            (egui::Key::R, Note::ThirtySecond),
        ];
        for (key, note) in retrigger_pairs.iter() {
            if ctx.input().key_pressed(*key) {
                let is_active = self.retrigger.get(note).unwrap();
                if !is_active.load(atomic::Ordering::SeqCst) {
                    self.into_engine
                        .push(MessageIntoEngine::RetriggerOn {
                            repeat_duration: note.duration(beats_per_minute),
                            mix_factor: 0.8,
                        })
                        .unwrap();
                    is_active.store(true, atomic::Ordering::SeqCst);
                }
            };
            if ctx.input().key_released(*key) {
                self.into_engine
                    .push(MessageIntoEngine::RetriggerOff)
                    .unwrap();
                self.retrigger
                    .get(note)
                    .unwrap()
                    .store(false, atomic::Ordering::SeqCst);
            }
        }
        let retrigger_pairs = &[
            (egui::Key::A, Note::Fourth),
            (egui::Key::S, Note::Eighth),
            (egui::Key::D, Note::Sixteenth),
            (egui::Key::F, Note::ThirtySecond),
        ];
        for (key, note) in retrigger_pairs.iter() {
            if ctx.input().key_pressed(*key) {
                let is_active = self.trance_gate.get(note).unwrap();
                if !is_active.load(atomic::Ordering::SeqCst) {
                    self.into_engine
                        .push(MessageIntoEngine::TranceGateOn {
                            gate_duration: note.duration(beats_per_minute),
                            mix_factor: 1.0,
                        })
                        .unwrap();
                    is_active.store(true, atomic::Ordering::SeqCst);
                }
            };
            if ctx.input().key_released(*key) {
                self.into_engine
                    .push(MessageIntoEngine::TranceGateOff)
                    .unwrap();
                self.trance_gate
                    .get(note)
                    .unwrap()
                    .store(false, atomic::Ordering::SeqCst);
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("photon - interactive music player");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Play").clicked() {
                    info!("Sending play signal to engine...");
                    self.into_engine.push(MessageIntoEngine::Play).unwrap();
                };
                if ui.button("Pause").clicked() {
                    info!("Sending pause signal to engine...");
                    self.into_engine.push(MessageIntoEngine::Pause).unwrap();
                };
                if ui.button("Restart").clicked() {
                    info!("Sending restart signal to engine...");
                };
            });
            ui.separator();
            egui::Grid::new("effect-grid")
                .spacing(egui::vec2(10.0, 10.0))
                .show(ui, |ui| {
                    for (note, key) in all::<Note>().zip(&["Q", "W", "E", "R"]) {
                        effect_box(
                            ui,
                            self.retrigger.get(&note).unwrap(),
                            format!("Retrigger{}", note).as_str(),
                            key,
                        );
                    }
                    ui.end_row();
                    for (note, key) in all::<Note>().zip(&["A", "S", "D", "F"]) {
                        effect_box(
                            ui,
                            self.trance_gate.get(&note).unwrap(),
                            format!("TranceGate{}", note).as_str(),
                            key,
                        );
                    }
                    ui.end_row();
                });
        });
    }
}

pub fn effect_box(ui: &mut egui::Ui, status: &AtomicBool, text: &str, key: &str) {
    let label_color = ui.style().noninteractive().text_color();
    let box_size = egui::vec2(100.0, 100.0);
    let (box_rect, _) = ui.allocate_exact_size(box_size, egui::Sense::click());
    if ui.is_rect_visible(box_rect) {
        let box_color = if status.load(atomic::Ordering::SeqCst) {
            egui::Color32::from_gray(45)
        } else {
            egui::Color32::from_gray(50)
        };
        ui.painter().rect(
            box_rect,
            5.0,
            box_color,
            egui::Stroke::new(0.0, egui::Color32::from_rgb(0, 0, 0)),
        );
        ui.painter().text(
            box_rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::default(),
            label_color,
        );
        ui.painter().text(
            (box_rect.center_bottom() - egui::pos2(0.0, 15.0)).to_pos2(),
            egui::Align2::CENTER_CENTER,
            key,
            egui::FontId::default(),
            label_color,
        );
    }
}

#[derive(Debug, Sequence, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Note {
    // Second,
    Fourth,
    Eighth,
    Sixteenth,
    ThirtySecond,
    // SixtyFourth,
}

impl Note {
    pub fn duration(&self, beats_per_minute: f64) -> f64 {
        let factor = match self {
            Note::Fourth => 4.0,
            Note::Eighth => 8.0,
            Note::Sixteenth => 16.0,
            Note::ThirtySecond => 32.0,
        };
        60.0 / beats_per_minute * 4.0 / factor
    }
}

impl Display for Note {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                // Note::Second => "2nd",
                Note::Fourth => "4th",
                Note::Eighth => "8th",
                Note::Sixteenth => "16th",
                Note::ThirtySecond => "32nd",
                // Note::SixtyFourth => "64th",
            }
        )
    }
}
