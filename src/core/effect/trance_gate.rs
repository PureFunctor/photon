//! Ramps the volume down and up given a duration.

/// The parameters consumed by [`TranceGate`].
#[derive(Debug, Clone, Copy)]
pub struct TranceGateParameters {
    /// The length of the gate effect.
    pub gate_length: usize,
    /// The midpoint of the gate effect.
    pub gate_midpoint: usize,
    /// Determines how much of the repeated samples is mixed with the
    /// original audio.
    ///
    /// A value of `1.0` will fully mute the original track while the
    /// "default" value of `0.8` will let some pass through.
    pub mix_factor: f32,
}

impl TranceGateParameters {
    /// Creates a new [`TranceGateParameters`].
    ///
    /// # Example
    ///
    /// If you want a trance gate that completes a full cycle in 8th
    /// notes (or a 16th note each to go down then up) in 256 BPM with
    /// some of the original track playing through:
    ///
    /// ```rust
    /// # use photon::core::effect::trance_gate::*;
    /// let gate_duration = 60.0 / 256.0 * 4.0 / 8.0;
    /// let _ = TranceGateParameters::new(gate_duration, 0.8);
    /// ```
    pub fn new(gate_duration: f64, mix_factor: f32) -> Self {
        let gate_length = (gate_duration * 44100.0) as usize;
        let gate_midpoint = gate_length / 2;
        let mix_factor = mix_factor.clamp(0.0, 1.0);
        Self {
            gate_length,
            gate_midpoint,
            mix_factor,
        }
    }
}

/// The trance gate DSP and its internal state.
#[derive(Debug)]
pub struct TranceGate {
    /// The parameters for the effect.
    parameters: Option<TranceGateParameters>,
    /// The number of samples processsed, used for bookkeeping.
    counter: usize,
}

impl TranceGate {
    pub fn new() -> Self {
        Self {
            parameters: None,
            counter: 0,
        }
    }
}

impl TranceGate {
    /// Initializes the [`TranceGate`] i.e. turning it on
    pub fn initialize(&mut self, parameters: TranceGateParameters) {
        self.parameters = Some(parameters);
        self.counter = 0;
    }

    /// Deinitializes the [`TranceGate`] i.e. turning it off
    pub fn deinitialize(&mut self) {
        self.parameters = None;
        self.counter = 0;
    }

    /// Applies the effect to the `buffer`.
    ///
    /// This is a no-op if the [`TranceGate`] is deinitialized.
    pub fn process(&mut self, _: usize, buffer: &mut [f32]) {
        let parameters = match self.parameters {
            Some(parameters) => parameters,
            None => return,
        };
        for index in 0..buffer.len() / 2 {
            let mut gate_factor = 1.0;

            if self.counter >= parameters.gate_length {
                self.counter = 0;
            }

            if self.counter < parameters.gate_midpoint {
                gate_factor *= (parameters.gate_midpoint - self.counter) as f32
                    / parameters.gate_midpoint as f32;
            } else if self.counter >= parameters.gate_midpoint {
                gate_factor *= (self.counter - parameters.gate_midpoint) as f32
                    / parameters.gate_midpoint as f32;
            }

            // Transform gate_factor such that its baseline is 0.1
            gate_factor = gate_factor * (1.0 - 0.1) + 0.1;
            // Transform gate_factor relative to the mix_factor
            gate_factor = gate_factor * parameters.mix_factor + (1.0 - parameters.mix_factor);

            buffer[index * 2] *= gate_factor;
            buffer[index * 2 + 1] *= gate_factor;

            self.counter += 1;
        }
    }
}
