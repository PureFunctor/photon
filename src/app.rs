pub mod widgets;

use eframe::egui;
use photon::core::engine::{MessageFromEngine, MessageIntoEngine};
use rtrb::{Consumer, Producer};

use self::widgets::{EffectPad, EffectPadEvent};

pub struct PhotonPlayer {
    into_engine: Producer<MessageIntoEngine>,
    from_engine: Consumer<MessageFromEngine>,
    active_retrigger: Option<f64>,
    active_trance_gate: Option<f64>,
}

impl PhotonPlayer {
    pub fn new(
        into_engine: Producer<MessageIntoEngine>,
        from_engine: Consumer<MessageFromEngine>,
    ) -> Self {
        Self {
            into_engine,
            from_engine,
            active_retrigger: None,
            active_trance_gate: None,
        }
    }

    pub fn play(&mut self) {
        self.into_engine.push(MessageIntoEngine::Play).unwrap();
    }

    pub fn pause(&mut self) {
        self.into_engine.push(MessageIntoEngine::Pause).unwrap();
    }

    pub fn retrigger(&mut self, factor: f64, event: EffectPadEvent) {
        match event {
            EffectPadEvent::On => {
                if self.active_retrigger.is_none() || self.active_retrigger != Some(factor) {
                    self.active_retrigger = Some(factor);
                    self.into_engine
                        .push(MessageIntoEngine::RetriggerOn {
                            repeat_duration: 60.0 / 196.0 * 4.0 / factor,
                            mix_factor: 0.9,
                        })
                        .unwrap();
                }
            }
            EffectPadEvent::Off => {
                if self.active_retrigger == Some(factor) {
                    self.into_engine
                        .push(MessageIntoEngine::RetriggerOff)
                        .unwrap();
                    self.active_retrigger = None;
                }
            }
        };
    }

    pub fn trance_gate(&mut self, factor: f64, event: EffectPadEvent) {
        match event {
            EffectPadEvent::On => {
                if self.active_trance_gate.is_none() || self.active_trance_gate != Some(factor) {
                    self.active_trance_gate = Some(factor);
                    self.into_engine
                        .push(MessageIntoEngine::TranceGateOn {
                            gate_duration: 60.0 / 196.0 * 4.0 / factor,
                            mix_factor: 0.9,
                        })
                        .unwrap();
                }
            }
            EffectPadEvent::Off => {
                if self.active_trance_gate == Some(factor) {
                    self.into_engine
                        .push(MessageIntoEngine::TranceGateOff)
                        .unwrap();
                    self.active_trance_gate = None;
                }
            }
        };
    }
}

impl eframe::App for PhotonPlayer {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        while let Ok(_message) = self.from_engine.pop() {}

        egui::TopBottomPanel::top("top-panel").show(ctx, |ui| {
            ui.heading("photon - interactive music player");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Play").clicked() {
                    self.play();
                }
                if ui.button("Pause").clicked() {
                    self.pause();
                }
            });
            ui.separator();
            egui::TopBottomPanel::bottom("bottom-panel")
                .frame(egui::Frame::default().inner_margin(10.0))
                .min_height(120.0)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            EffectPad::new(
                                "Re4",
                                egui::Key::Q,
                                egui::Color32::from_rgb(154, 204, 234),
                            )
                            .show(ui, |event| self.retrigger(4.0, event));
                            EffectPad::new(
                                "Re8",
                                egui::Key::W,
                                egui::Color32::from_rgb(187, 231, 177),
                            )
                            .show(ui, |event| self.retrigger(8.0, event));
                            EffectPad::new(
                                "Re16",
                                egui::Key::E,
                                egui::Color32::from_rgb(248, 189, 79),
                            )
                            .show(ui, |event| self.retrigger(16.0, event));
                            EffectPad::new(
                                "Re32",
                                egui::Key::R,
                                egui::Color32::from_rgb(243, 145, 179),
                            )
                            .show(ui, |event| self.retrigger(32.0, event));
                        });
                        ui.vertical(|ui| {
                            EffectPad::new(
                                "Gt4",
                                egui::Key::A,
                                egui::Color32::from_rgb(154, 204, 234),
                            )
                            .show(ui, |event| self.trance_gate(4.0, event));
                            EffectPad::new(
                                "Gt8",
                                egui::Key::S,
                                egui::Color32::from_rgb(187, 231, 177),
                            )
                            .show(ui, |event| self.trance_gate(8.0, event));
                            EffectPad::new(
                                "Gt16",
                                egui::Key::D,
                                egui::Color32::from_rgb(248, 189, 79),
                            )
                            .show(ui, |event| self.trance_gate(16.0, event));
                            EffectPad::new(
                                "Gt32",
                                egui::Key::F,
                                egui::Color32::from_rgb(243, 145, 179),
                            )
                            .show(ui, |event| self.trance_gate(32.0, event));
                        });
                    });
                });
        });
    }
}
