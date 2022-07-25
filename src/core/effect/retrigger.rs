//! Repeats a short duration of samples while active.
use std::{sync::Arc, time::Duration};

#[derive(Debug, Clone, Copy)]
/// The parameters for the audio effect.
pub struct RetriggerParameters {
    /// The start position of the repetition.
    pub repeat_start: usize,
    /// The end position of the repetition.
    pub repeat_end: usize,
    /// The duration of the retrigger effect.
    pub repeat_duration: Duration,
    /// The beat division to snap to.
    pub repeat_factor: f32,
    /// The threshold for fading between repetitions.
    ///
    /// Defaults to 441 or about 10ms in 44100 Hz. If the number of
    /// samples being repeated is smaller, then that is used instead.
    pub fade_threshold: usize,
    /// The beats per minute of the current track for computations.
    pub beats_per_minute: f32,
    /// Determines how the retriggered audio is mixed with the actual
    /// track.
    ///
    /// e.g. A value of `1.0` will only play the retriggered audio
    /// while a value of `0.7` will leave `0.3` for the actual track.
    pub mix_factor: f32,
}

impl RetriggerParameters {
    /// Creates a new [`RetriggerParameters`].
    ///
    /// # Example
    ///
    /// If we want to repeat the 16th notes of 256 BPM track with some
    /// of the actual track playing through:
    ///
    /// ```rust
    /// # use photon::core::effect::retrigger::*;
    /// let _ = RetriggerParameters::new(0, 16.0, 256.0, 0.8);
    /// ```
    pub fn new(
        repeat_start: usize,
        repeat_factor: f32,
        beats_per_minute: f32,
        mix_factor: f32,
    ) -> Self {
        let repeat_duration =
            Duration::from_secs_f32(60.0 / beats_per_minute * 4.0 / repeat_factor);
        let repeat_samples = (repeat_duration.as_secs() * 44100)
            + (repeat_duration.subsec_millis() * 44100 / 1000) as u64;
        let repeat_end = repeat_start + repeat_samples as usize;
        let fade_threshold = (repeat_samples as usize).min(441);
        let mix_factor = mix_factor.max(1.0);
        Self {
            repeat_start,
            repeat_end,
            repeat_duration,
            repeat_factor,
            beats_per_minute,
            fade_threshold,
            mix_factor,
        }
    }

    /// Compute the fade factor given an index. This number is used
    /// for fading repetitions in and out to allow for smoother
    /// transitions.
    pub fn fade_factor(&self, index: usize) -> f32 {
        let fade = self.fade_threshold;
        let after = self.repeat_end - fade;
        let until = self.repeat_start + fade;
        if index < until {
            (fade - (until - index) + 1) as f32 / fade as f32
        } else if index > after {
            (fade - (index - after) + 1) as f32 / fade as f32
        } else {
            1.0
        }
    }
}

/// The retrigger effect and its state.
#[derive(Debug)]
pub struct Retrigger {
    /// The stream of audio samples.
    pub samples: Arc<Vec<f32>>,
    /// The parameters for the effect.
    pub parameters: Option<RetriggerParameters>,
    /// The current index of the retrigger.
    pub index: Option<usize>,
}

impl Retrigger {
    pub fn new(samples: Arc<Vec<f32>>) -> Self {
        Self {
            samples,
            parameters: None,
            index: None,
        }
    }
}

impl Retrigger {
    pub fn initialize(&mut self, parameters: RetriggerParameters) {
        self.parameters = Some(parameters);
        self.index = Some(parameters.repeat_start);
    }

    pub fn deinitialize(&mut self) {
        self.parameters = None;
        self.index = None;
    }

    pub fn process(&mut self, track_index: usize, buffer: &mut [f32]) {
        let parameters = match self.parameters {
            Some(parameters) => parameters,
            None => return,
        };
        let mut current_index = match self.index {
            Some(current_index) => current_index,
            None => return,
        };
        for index in 0..buffer.len() / 2 {
            if current_index >= parameters.repeat_end {
                current_index = parameters.repeat_start;
            }
            let fade_factor = parameters.fade_factor(current_index);
            buffer[index * 2]
                // retrigger track
                = fade_factor * self.samples[current_index * 2] * parameters.mix_factor
                // actual track
                + self.samples[(track_index + index) * 2] * (1.0 - parameters.mix_factor);
            buffer[index * 2 + 1]
                // retrigger track
                = fade_factor * self.samples[current_index * 2 + 1] * parameters.mix_factor
                // actual track
                + self.samples[(track_index + index) * 2 + 1] * (1.0 - parameters.mix_factor);
            current_index += 1;
        }
        self.index = Some(current_index);
    }
}
