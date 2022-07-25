//! Fades the track in and out in succession.
use std::{sync::Arc, time::Duration};

#[derive(Debug, Clone, Copy)]
/// The parameters for the audio effect.
pub struct TranceGateParameters {
    /// The start position of the gate.
    pub gate_start: usize,
    /// The end position of the gate.
    pub gate_end: usize,
    /// The threshold for fading between repetitions.
    pub fade_threshold: usize,
    /// Determines how much of the gate is mixed with the original
    /// audio source. A value of `1.0` fully mutes the source track,
    /// which silences it during the gaps produced by the gate.
    pub mix_factor: f32,
}

impl TranceGateParameters {
    /// Creates a new [`TranceGateParameters`].
    pub fn new(
        gate_start: usize,
        gate_factor: f32,
        beats_per_minute: f32,
        mix_factor: f32,
    ) -> Self {
        let gate_duration = 60.0 / beats_per_minute * 2.0 / gate_factor;
        let gate_samples = (gate_duration * 44100.0) as usize;
        let gate_end = gate_start + gate_samples;
        let fade_threshold = (gate_samples / 2).min(441);
        let mix_factor = mix_factor.clamp(0.0, 1.0);
        Self {
            gate_start,
            gate_end,
            fade_threshold,
            mix_factor,
        }
    }

    /// Compute the fade factor given an index. This number is used
    /// for fading repetitions in and out to allow for smoother
    /// transitions.
    pub fn fade_factor(&self, index: usize) -> f32 {
        let fade = self.fade_threshold;
        let after = self.gate_end - fade;
        let until = self.gate_start + fade;
        if index < until {
            (fade - (until - index) + 1) as f32 / fade as f32
        } else if index > after {
            (fade - (index - after) + 1) as f32 / fade as f32
        } else {
            1.0
        }
    }
}

/// The trance gate effect and its state
#[derive(Debug)]
pub struct TranceGate {
    /// The stream of audio samples.
    samples: Arc<Vec<f32>>,
    /// The parameters for the effect.
    parameters: Option<TranceGateParameters>,
    /// The current index of the gate.
    index: Option<usize>,
    /// Determines if the gate is closed.
    silent: bool,
}

impl TranceGate {
    pub fn new(samples: Arc<Vec<f32>>) -> Self {
        Self {
            samples,
            parameters: None,
            index: None,
            silent: false,
        }
    }
}

impl TranceGate {
    pub fn initialize(&mut self, parameters: TranceGateParameters) {
        self.parameters = Some(parameters);
        self.index = Some(parameters.gate_start);
    }

    pub fn deinitialize(&mut self) {
        self.parameters = None;
        self.index = None;
        self.silent = false;
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
            if current_index >= parameters.gate_end {
                self.silent = !self.silent;
                current_index = parameters.gate_start;
            }
            if self.silent {
                let (original_0, original_1) = if (track_index + index) * 2 >= self.samples.len() {
                    (0.0, 0.0)
                } else {
                    (
                        self.samples[(track_index + index) * 2] * (1.0 - parameters.mix_factor),
                        self.samples[(track_index + index) * 2 + 1] * (1.0 - parameters.mix_factor),
                    )
                };

                buffer[index * 2] = original_0;
                buffer[index * 2 + 1] = original_1;
            } else {
                let fade_factor = parameters.fade_factor(current_index);

                let (gate_0, gate_1) = {
                    (
                        buffer[index * 2] * fade_factor * parameters.mix_factor,
                        buffer[index * 2 + 1] * fade_factor * parameters.mix_factor,
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

                buffer[index * 2] = gate_0 + original_0;
                buffer[index * 2 + 1] = gate_1 + original_1;
            }
            current_index += 1;
        }
        self.index = Some(current_index);
    }
}
