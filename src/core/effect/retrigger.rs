//! Repeats a short duration of samples while active.
//!
//! # Overview
//!
//! This audio effect transforms the track from:
//! ```text
//! A B C D E F G H I J K L
//! ```
//! into:
//! ```text
//!  start            end
//!    v               v
//! A B C D C D C D C D K L
//!    |   |   |   |   |
//!    +---+---+---+---+
//!        retrigger
//! ```
use std::sync::Arc;

/// The parameters consumed by [`Retrigger`].
#[derive(Debug, Clone, Copy)]
pub struct RetriggerParameters {
    /// The starting index of the repetition.
    pub repeat_start: usize,
    /// The ending index of the repetition.
    pub repeat_end: usize,
    /// The threshold for fading between repetitions.
    ///
    /// By default, this is set to 441 or about 10ms in 441000 Hz. If
    /// the total duration of the samples being repeated is smaller,
    /// then the 1/4th and 3/4th points are used.
    pub fade_threshold: usize,
    /// Determines how much of the repeated samples is mixed with the
    /// original audio.
    ///
    /// A value of `1.0` will fully mute the original track while the
    /// "default" value of `0.8` will let some pass through.
    pub mix_factor: f32,
}

impl RetriggerParameters {
    /// Creates a new [`RetriggerParameters`].
    ///
    /// # Example
    ///
    /// If you want to repeat the 16th notes of 256 BPM track with
    /// some of the original track playing through:
    ///
    /// ```rust
    /// # use photon::core::effect::retrigger::*;
    /// let repeat_duration = 60.0 / 256.0 * 4.0 / 16.0;
    /// let _ = RetriggerParameters::new(0, repeat_duration, 0.8);
    /// ```
    pub fn new(repeat_start: usize, repeat_duration: f64, mix_factor: f32) -> Self {
        let repeat_samples = (repeat_duration * 44100.0) as usize;
        let repeat_end = repeat_start + repeat_samples as usize;
        let fade_threshold = (repeat_samples as usize / 4).min(441);
        let mix_factor = mix_factor.clamp(0.0, 1.0);
        Self {
            repeat_start,
            repeat_end,
            fade_threshold,
            mix_factor,
        }
    }

    /// Compute the fade factor given the current index of the
    /// retrigger. This value is used for fading in and out of
    /// repetitions to allow for smoother transitions.
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

/// The retrigger DSP and its internal state.
#[derive(Debug)]
pub struct Retrigger {
    /// The stream of audio samples.
    pub samples: Arc<Vec<f32>>,
    /// The parameters for the effect.
    pub parameters: Option<RetriggerParameters>,
    /// The current index of the effect.
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
    /// Initializes the [`Retrigger`] i.e. turning it on
    pub fn initialize(&mut self, parameters: RetriggerParameters) {
        self.parameters = Some(parameters);
        self.index = Some(parameters.repeat_start);
    }

    /// Deinitializes the [`Retrigger`] i.e. turning it off
    pub fn deinitialize(&mut self) {
        self.parameters = None;
        self.index = None;
    }

    /// Applies the effect to the `buffer`, with the `track_index`
    /// used for mixing the original track.
    ///
    /// This is a no-op if the [`Retrigger`] is deinitialized.
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

            let (retrigger_0, retrigger_1) = if current_index * 2 >= self.samples.len() {
                (0.0, 0.0)
            } else {
                (
                    fade_factor * self.samples[current_index * 2] * parameters.mix_factor,
                    fade_factor * self.samples[current_index * 2 + 1] * parameters.mix_factor,
                )
            };

            let (original_0, original_1) = if (track_index + index) * 2 >= self.samples.len() {
                (0.0, 0.0)
            } else {
                (
                    self.samples[(track_index + index) * 2] * (1.0 - parameters.mix_factor),
                    self.samples[(track_index + index) * 2 + 1] * (1.0 - parameters.mix_factor),
                )
            };

            buffer[index * 2] = retrigger_0 + original_0;
            buffer[index * 2 + 1] = retrigger_1 + original_1;

            current_index += 1;
        }
        self.index = Some(current_index);
    }
}
