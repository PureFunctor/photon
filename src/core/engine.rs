//! This module defines photon's audio engine.
//!
//! The [`Engine`] type operates over the raw stream of samples and is generally
//! concerned about applying DSPs and writing samples to the audio
//! sink. Likewise, effects such as
//! [crossfading](https://en.wikipedia.org/wiki/Fade_(audio_engineering)#Crossfading)
//! may be implemented in the future as a higher-level abstraction.
//!
//! [`Engine`]: Engine

use std::sync::Arc;

use rtrb::{Consumer, Producer};

/// Messages into the engine.
#[derive(Debug)]
pub enum MessageIntoEngine {
    Play,
    Pause,
}

/// Messages from the engine.
#[derive(Debug)]
pub enum MessageFromEngine {}

/// The audio engine.
#[derive(Debug)]
pub struct Engine {
    /// The stream of samples.
    ///
    /// Implemented as an `Arc<Vec<f32>>` in the meantime for simplicity. For
    /// instance, DSPs such as the `retrigger` effect benefits from having
    /// pre-cached samples as all it needs to do is hijack the playhead.
    pub samples: Arc<Vec<f32>>,
    /// The sample index.
    ///
    /// This represents the current "canonical" index for the [`samples`]
    /// stream. DSPs such as `retrigger` may maintain their own indices,
    /// effectively overriding playback.
    ///
    /// [`samples`]: Self::samples
    pub index: usize,
    /// Determines if playback is active.
    pub playing: bool,
    /// Total number of samples processed.
    pub total: usize,
    /// A channel for incoming messages.
    pub into_engine: Consumer<MessageIntoEngine>,
    /// A channel for outgoing messages.
    pub from_engine: Producer<MessageFromEngine>,
}

impl Engine {
    /// Creates a new [`Engine`].
    pub fn new(
        samples: Arc<Vec<f32>>,
        into_engine: Consumer<MessageIntoEngine>,
        from_engine: Producer<MessageFromEngine>,
    ) -> Self {
        Self {
            samples,
            index: 0,
            playing: false,
            total: 0,
            into_engine,
            from_engine,
        }
    }
}

impl Engine {
    /// The core callback consumed by the audio thread.
    ///
    /// # Notes
    ///
    /// See [Real-time audio programming 101: time waits for
    /// nothing](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing)
    /// for a crash course on what _not_ to do inside of this function.
    ///
    /// ## tl;dr
    ///
    /// Do not do anything that blocks this from executing, otherwise, the audio
    /// backend will rain vitriol and hellfire down upon the listener. The best
    /// way to alleviate this is to mute the `buffer` by filling it with zeroes
    /// if you expect to wait on some external event.
    pub fn process(&mut self, buffer: &mut [f32]) {
        while let Ok(message) = self.into_engine.pop() {
            match message {
                MessageIntoEngine::Play => self.playing = true,
                MessageIntoEngine::Pause => self.playing = false,
            }
        }
        if !self.playing {
            quiet(buffer);
        } else {
            for (index, sample) in buffer.iter_mut().enumerate() {
                let index = self.index + index;
                if index >= self.samples.len() {
                    *sample = 0.0;
                } else {
                    *sample = self.samples[index];
                }
            }
            self.index += buffer.len();
        }
    }
}

/// Fill a buffer with silence.
pub fn quiet(buffer: &mut [f32]) {
    for sample in buffer.iter_mut() {
        *sample = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rtrb::RingBuffer;

    use super::Engine;

    #[test]
    fn sample_overflow() {
        let samples = Arc::new(vec![1.0; 4]);
        let (_, into_engine) = RingBuffer::new(8);
        let (from_engine, _) = RingBuffer::new(8);
        let mut engine = Engine::new(samples, into_engine, from_engine);
        let mut buffer = vec![0.0; 8];
        engine.playing = true;
        engine.process(&mut buffer);
        assert_eq!(buffer, vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
    }
}
