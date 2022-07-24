use std::{
    fmt::Write,
    io,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use eframe::{egui, App};
use log::info;
use log_buffer::LogBuffer;
use photon::core::playback::Closure;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};

struct LogBufferWriter(Arc<Mutex<LogBuffer<[u8; 2048]>>>);

impl io::Write for LogBufferWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let log_buffer = &mut *self.0.lock().unwrap();
        log_buffer
            .write_str(std::str::from_utf8(buffer).unwrap())
            .unwrap();
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let log_buffer = Arc::new(Mutex::new(LogBuffer::new([0; 2048])));
    let log_writer = LogBufferWriter(log_buffer.clone());
    CombinedLogger::init(vec![
        TermLogger::new(
            log::LevelFilter::Info,
            Config::default(),
            TerminalMode::Stderr,
            ColorChoice::Always,
        ),
        WriteLogger::new(log::LevelFilter::Info, Config::default(), log_writer),
    ])?;

    let closure = Closure::new();
    let photon = Photon::new(closure, log_buffer);
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
    log_buffer: Arc<Mutex<LogBuffer<[u8; 2048]>>>,
}

impl Photon {
    fn new(closure: Closure, log_buffer: Arc<Mutex<LogBuffer<[u8; 2048]>>>) -> Self {
        Self {
            file: Arc::new(Mutex::new(Status::Nothing)),
            closure,
            log_buffer,
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
                                .context("No file chosen, doing nothing.")?;
                            let mut name = String::new();
                            name.push_str(
                                path.to_str().context("Could not convert from OS string.")?,
                            );
                            Ok(name)
                        })();
                        match result {
                            Ok(result) => file.lock().unwrap().set_loaded(result),
                            Err(error) => info!("{}", error),
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
            ui.set_min_height(128.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(self.log_buffer.lock().unwrap().extract())
                });
        });
    }
}
