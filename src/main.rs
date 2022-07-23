use std::sync::{Arc, Mutex};

use anyhow::Context;
use eframe::{egui, App};
use log::error;
use photon::core::playback::Closure;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let closure = Closure::new();
    let photon = Photon::new(closure);
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

#[derive(Debug, Clone, Copy)]
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

    #[inline]
    pub fn set_loading(&mut self) {
        *self = Status::Loading;
    }

    #[inline]
    pub fn set_loaded(&mut self, value: T) {
        *self = Status::Loaded(value);
    }

    #[inline]
    pub fn is_loaded(&self) -> bool {
        matches!(self, Status::Loaded(_))
    }
}

/// The "I just want this done" type.
type State<T> = Arc<Mutex<Status<T>>>;

struct Photon {
    file: State<String>,
    closure: Closure,
}

impl Photon {
    fn new(closure: Closure) -> Self {
        Self {
            file: Arc::new(Mutex::new(Status::Nothing)),
            closure,
        }
    }
}

impl App for Photon {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Photon - Interactive Music Player");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Choose File").clicked() {
                    let file = self.file.clone();
                    std::thread::spawn(move || {
                        file.lock().unwrap().set_loading();
                        let result: anyhow::Result<String> = (|| {
                            let path = rfd::FileDialog::new()
                                .pick_file()
                                .context("No file picked")?;
                            let mut name = String::new();
                            name.push_str(
                                path.to_str().context("Could not convert from OS string.")?,
                            );
                            Ok(name)
                        })();
                        match result {
                            Ok(result) => file.lock().unwrap().set_loaded(result),
                            Err(error) => error!("{}", error),
                        }
                    });
                }
                if let Status::Loaded(name) = self.file.lock().unwrap().as_ref() {
                    ui.label(name);
                }
            });
        });
        egui::TopBottomPanel::bottom("bottom-panel").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.label("Error Log");
            ui.separator();
            ui.add(
                egui::TextEdit::multiline(&mut "In the beginning, there was darkness...")
                    .code_editor()
                    .desired_width(f32::INFINITY),
            );
        });
    }
}
