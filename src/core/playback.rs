use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
};

use anyhow::{bail, Context};
use cpal::{
    traits::{DeviceTrait, HostTrait},
    BufferSize, OutputCallbackInfo, SampleRate, StreamConfig,
};
use log::error;
use symphonia::core::sample::Sample;

use super::audio::SamplesInMemory;

/// The current state of the stream.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlaybackState {
    /// The number of frames already processed.
    pub start_offset: usize,
    /// Whether or not we should send samples.
    pub playing: bool,
}

/// Events related to playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackEvent {
    Play,
    Pause,
    Restart,
}

#[derive(Debug)]
pub enum ToClosure {
    Initialize(SamplesInMemory),
    Playback(PlaybackEvent),
}

#[derive(Debug)]
pub enum FromClosure {}

/// The closure around the `Stream` instance.
pub struct Closure {
    /// The handle of the thread containing the stream.
    handle: JoinHandle<anyhow::Result<()>>,
    /// An event sender for communication into the closure.
    to_closure: Sender<ToClosure>,
    /// An event receiver for communication from the closure.
    from_closure: Receiver<FromClosure>,
}

impl Closure {
    pub fn new() -> Self {
        let (to_closure_s, to_closure_r) = mpsc::channel();
        let (_from_closure_s, from_closure_r) = mpsc::channel();

        let handle: JoinHandle<anyhow::Result<()>> = std::thread::spawn(move || {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .context("No default output device!")?;

            let samples = if let ToClosure::Initialize(samples) =
                to_closure_r.recv().context("Timed out!")?
            {
                samples
            } else {
                bail!("First event to stream closure is not initialize!");
            };

            let config = StreamConfig {
                channels: samples.channels as u16,
                sample_rate: SampleRate(samples.sample_rate as u32),
                buffer_size: BufferSize::Default,
            };

            let mut state = PlaybackState {
                start_offset: 0,
                playing: false,
            };

            let (callback_s, callback_r) = mpsc::channel();
            let _stream = device.build_output_stream(
                &config,
                move |buffer: &mut [f32], _: &OutputCallbackInfo| {
                    for callback_e in callback_r.try_iter() {
                        match callback_e {
                            PlaybackEvent::Play => state.playing = true,
                            PlaybackEvent::Pause => state.playing = false,
                            PlaybackEvent::Restart => state.start_offset = 0,
                        }
                    }
                    if state.playing {
                        samples.copy_from_onto(state.start_offset, buffer);
                        state.start_offset += buffer.len();
                    } else {
                        buffer.iter_mut().for_each(|sample| *sample = f32::MID);
                    }
                },
                |e| error!("{}", e),
            )?;

            for to_closure_e in to_closure_r.iter() {
                match to_closure_e {
                    ToClosure::Initialize(_) => bail!("can't reinitialize!"),
                    ToClosure::Playback(callback_e) => callback_s.send(callback_e)?,
                };
            }

            Ok(())
        });

        Self {
            handle,
            to_closure: to_closure_s,
            from_closure: from_closure_r,
        }
    }
}

impl Default for Closure {
    fn default() -> Self {
        Self::new()
    }
}
