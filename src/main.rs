use std::{
    fs::File,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
};

use cpal::Stream;
use eframe::{egui, App};
use photon::core::{
    audio::SamplesInMemory,
    playback::{self, PlaybackEvent},
};

enum StreamEvent {
    Initialize(SamplesInMemory),
    Playback(PlaybackEvent),
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let (_stream_handle, stream_sender) = {
        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut audio_stream: Option<Stream> = None;
            let mut playback_sender: Option<Sender<PlaybackEvent>> = None;

            for event in receiver.iter() {
                match event {
                    StreamEvent::Initialize(samples) => {
                        let (stream, sender) = playback::initialize(samples).unwrap();
                        audio_stream.replace(stream);
                        playback_sender.replace(sender);
                    }
                    StreamEvent::Playback(event) => {
                        if let Some(ref playback_sender) = playback_sender {
                            playback_sender.send(event).unwrap();
                        }
                    }
                }
            }
        });
        (handle, sender)
    };

    let photon = Photon::new(stream_sender);

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

pub enum Status<T> {
    Nothing,
    Loading,
    Loaded(T),
}

impl<T> Status<T> {
    pub fn as_ref(&self) -> Status<&T> {
        match *self {
            Self::Nothing => Status::Nothing,
            Self::Loading => Status::Loading,
            Self::Loaded(ref value) => Status::Loaded(value),
        }
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, Status::Loaded(_))
    }
}

/// The "I just want this done" type.
type Nullable<T> = Arc<Mutex<Status<anyhow::Result<T>>>>;

struct Photon {
    file: Nullable<String>,
    stream: Sender<StreamEvent>,
}

impl Photon {
    fn new(stream_sender: Sender<StreamEvent>) -> Self {
        Self {
            file: Arc::new(Mutex::new(Status::Nothing)),
            stream: stream_sender,
        }
    }
}

impl App for Photon {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Photon - Interactive Music Player");
            ui.add(egui::Separator::default());
            ui.horizontal(|ui| {
                if ui.button("File").clicked() {
                    let file = self.file.clone();
                    let stream = self.stream.clone();
                    std::thread::spawn(move || {
                        *file.lock().unwrap() = Status::Loading;
                        let handle = rfd::FileDialog::new().pick_file().unwrap();
                        let mut name = String::new();
                        name.push_str(handle.file_name().unwrap().to_str().unwrap());
                        let contents = File::open(handle).unwrap();
                        let samples = SamplesInMemory::try_from_file(contents).unwrap();
                        stream.send(StreamEvent::Initialize(samples)).unwrap();
                        *file.lock().unwrap() = Status::Loaded(Ok(name));
                    });
                }
                match self.file.lock().unwrap().as_ref() {
                    Status::Nothing => {}
                    Status::Loading => {
                        ui.spinner();
                    }
                    Status::Loaded(Ok(name)) => {
                        ui.label(name);
                    }
                    _ => {}
                }
            });
            if self.file.lock().unwrap().is_loaded() {
                ui.horizontal(|ui| {
                    if ui.button("Play").clicked() {
                        self.stream
                            .send(StreamEvent::Playback(PlaybackEvent::Play))
                            .unwrap();
                    };
                    if ui.button("Pause").clicked() {
                        self.stream
                            .send(StreamEvent::Playback(PlaybackEvent::Pause))
                            .unwrap();
                    };
                });
            }
        });
    }
}
