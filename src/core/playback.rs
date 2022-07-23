use std::sync::mpsc;

use cpal::{
    traits::{DeviceTrait, HostTrait},
    BufferSize, OutputCallbackInfo, SampleRate, Stream, StreamConfig,
};
use log::error;
use symphonia::core::sample::Sample;

use super::audio::SamplesInMemory;

/// The current state of the stream.
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    /// The number of frames already processed.
    pub start_offset: usize,
    /// Whether or not we should send samples.
    pub playing: bool,
}

/// Events related to playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Play,
    Pause,
    Restart,
}

pub fn initialize(samples: SamplesInMemory) -> anyhow::Result<(Stream, mpsc::Sender<Event>)> {
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();

    let config = StreamConfig {
        channels: samples.channels as u16,
        sample_rate: SampleRate(samples.sample_rate as u32),
        buffer_size: BufferSize::Default,
    };

    {
        let samples = samples.clone();

        let mut state = State {
            start_offset: 0,
            playing: false,
        };

        let (sender, events) = mpsc::channel::<Event>();

        let stream = device.build_output_stream(
            &config,
            move |buffer: &mut [f32], _: &OutputCallbackInfo| {
                for event in events.try_iter() {
                    match event {
                        Event::Play => state.playing = true,
                        Event::Pause => state.playing = false,
                        Event::Restart => state.start_offset = 0,
                    }
                }
                if state.playing {
                    samples.copy_from_onto(state.start_offset, buffer);
                    state.start_offset += buffer.len();
                } else {
                    buffer.iter_mut().for_each(|sample| *sample = f32::MID);
                }
            },
            |e| error!("{:?}", e),
        )?;

        Ok((stream, sender))
    }
}
