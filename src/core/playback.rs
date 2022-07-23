use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, OutputCallbackInfo, SampleRate, Stream, StreamConfig,
};
use log::error;

use super::audio::SamplesInMemory;

pub fn initialize(samples: SamplesInMemory) -> anyhow::Result<Stream> {
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();

    let config = StreamConfig {
        channels: samples.channels as u16,
        sample_rate: SampleRate(samples.sample_rate as u32),
        buffer_size: BufferSize::Default,
    };

    let stream = {
        let samples = samples.clone();
        let mut start_offset = 0;
        device.build_output_stream(
            &config,
            move |buffer: &mut [f32], _: &OutputCallbackInfo| {
                samples.copy_from_onto(start_offset, buffer);
                start_offset += buffer.len();
            },
            |e| error!("{:?}", e),
        )
    }?;

    stream.pause()?;

    Ok(stream)
}
