use anyhow::Context;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, StreamConfig,
};
use log::info;
use std::{fs::File, sync::Arc, time::Duration};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::DecoderOptions,
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::Hint,
};

#[derive(Debug, Clone)]
pub struct AudioInMemory {
    pub samples: Arc<Vec<f32>>,
    pub channels: usize,
    pub sample_rate: usize,
}

impl AudioInMemory {
    pub fn from_file(file: File) -> anyhow::Result<Self> {
        let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
        let hint = Hint::new();
        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts = MetadataOptions::default();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .unwrap();
        let mut reader = probed.format;
        let track = reader.default_track().unwrap();
        let decoder_opts = DecoderOptions::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .unwrap();

        let (spec, duration) = {
            let packet = reader
                .next_packet()
                .context("while reading the next packet")?;
            let decoded = decoder
                .decode(&packet)
                .context("while decoding the next packet")?;
            (*decoded.spec(), decoded.capacity() as u64)
        };

        let mut samples = vec![];
        let mut sample_buffer = SampleBuffer::<f32>::new(duration, spec);
        let channels = spec.channels.count();
        let sample_rate = spec.rate as usize;

        let _: Result<(), _> = loop {
            let packet = match reader.next_packet() {
                Ok(packet) => packet,
                Err(error) => break Err(error),
            };
            let decoded = match decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(error) => break Err(error),
            };
            sample_buffer.copy_interleaved_ref(decoded);
            samples.extend_from_slice(sample_buffer.samples());
        };

        let finalize = decoder.finalize();

        if let Some(verify_ok) = finalize.verify_ok {
            if verify_ok {
                info!("Decoder verify OK!");
            } else {
                info!("Decoder verify not OK!");
            }
        }

        let samples = Arc::new(samples);

        Ok(Self {
            samples,
            channels,
            sample_rate,
        })
    }

    pub fn onto(&self, cursor: usize, output: &mut [f32]) {
        let start = cursor;
        let end = cursor + output.len();
        output.copy_from_slice(&self.samples[start..end]);
    }

    pub fn length(&self) -> Duration {
        Duration::from_secs_f64(self.samples.len() as f64 / self.sample_rate as f64 / 2.0)
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let file = File::open("assets/discover_universe.flac").unwrap();
    let audio = AudioInMemory::from_file(file)?;

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config = StreamConfig {
        channels: audio.channels as u16,
        sample_rate: SampleRate(audio.sample_rate as u32),
        buffer_size: BufferSize::Default,
    };

    let stream = {
        let audio = audio.clone();
        let mut cursor = 0;
        device.build_output_stream(
            &config,
            move |output: &mut [f32], _| {
                audio.onto(cursor, output);
                cursor += output.len();
            },
            |_| {},
        )
    }?;

    stream.play()?;

    info!(
        "Main thread is sleeping for {} seconds.",
        audio.length().as_secs()
    );
    std::thread::sleep(audio.length());

    Ok(())
}
