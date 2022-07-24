use std::{sync::Arc, time::Duration};

#[derive(Debug)]
pub struct Retrigger {
    frames: Arc<Vec<f32>>,
    repeat_start: Option<usize>,
    repeat_end: Option<usize>,
    fade_threshold: Option<usize>,
    index: Option<usize>,
    beats_per_minute: f32,
}

impl Retrigger {
    pub fn new(frames: Arc<Vec<f32>>, beats_per_minute: f32) -> Self {
        Self {
            frames,
            repeat_start: None,
            repeat_end: None,
            fade_threshold: None,
            index: None,
            beats_per_minute,
        }
    }
}

impl Retrigger {
    pub fn initialize(&mut self, repeat_start: usize, repeat_factor: f32) {
        self.index = Some(repeat_start);
        self.repeat_start = Some(repeat_start);
        let repeat_duration =
            Duration::from_secs_f32(60.0 / self.beats_per_minute * 4.0 / repeat_factor);
        let repeat_samples = (repeat_duration.as_secs() * 44100)
            + (repeat_duration.subsec_millis() * 44100 / 1000) as u64;
        self.fade_threshold = Some(repeat_samples.min(441) as usize);
        self.repeat_end = Some(repeat_start + repeat_samples as usize);
    }

    pub fn deinitialize(&mut self) {
        self.repeat_start = None;
        self.repeat_end = None;
        self.fade_threshold = None;
        self.index = None;
    }

    pub fn is_active(&self) -> bool {
        self.repeat_start.is_some()
            && self.repeat_end.is_some()
            && self.fade_threshold.is_some()
            && self.index.is_some()
    }

    pub fn process(&mut self, other: usize, buffer: &mut [f32]) {
        if !self.is_active() {
            return;
        }
        for index in 0..buffer.len() / 2 {
            if self.index.unwrap() >= self.repeat_end.unwrap() {
                self.index = self.repeat_start;
            }

            let factor = {
                let fade = self.fade_threshold.unwrap();
                let after = self.repeat_end.unwrap() - fade;
                let until = self.repeat_start.unwrap() + fade;
                if self.index.unwrap() < until {
                    (fade - (until - self.index.unwrap()) + 1) as f32 / fade as f32
                } else if self.index.unwrap() > after {
                    (fade - (self.index.unwrap() - after) + 1) as f32 / fade as f32
                } else {
                    1.0
                }
            };

            buffer[index * 2] = factor * self.frames[self.index.unwrap() * 2] * 0.80
                + self.frames[(other + index) * 2] * 0.20;
            buffer[index * 2 + 1] = factor * self.frames[self.index.unwrap() * 2 + 1] * 0.80
                + self.frames[(other + index) * 2 + 1] * 0.20;

            self.index = Some(self.index.unwrap() + 1);
        }
    }
}
