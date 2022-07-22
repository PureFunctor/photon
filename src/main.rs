use std::fs::File;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, OutputCallbackInfo, SampleRate, Stream, StreamConfig,
};
use rb::{Producer, RbConsumer, RbProducer, SpscRb, RB};
use symphonia::core::{
    audio::{AudioBufferRef, SampleBuffer, SignalSpec},
    codecs::DecoderOptions,
    formats::{FormatOptions, FormatReader},
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::Hint,
    sample::Sample,
};

pub fn file_reader(file: File) -> Box<dyn FormatReader> {
    let media_source_stream =
        MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
    let hint = Hint::new();
    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts = MetadataOptions::default();
    let probed = symphonia::default::get_probe()
        .format(&hint, media_source_stream, &format_opts, &metadata_opts)
        .unwrap();
    probed.format
}

pub fn play_track(
    reader: &mut Box<dyn FormatReader>,
    device: &Device,
) -> symphonia::core::errors::Result<()> {
    let track = reader.default_track().unwrap();

    let decoder_options = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_options)
        .unwrap();

    let mut audio_sink = None;

    let result = loop {
        let packet = match reader.next_packet() {
            Ok(packet) => packet,
            Err(error) => break Err(error),
        };
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(error) => break Err(error),
        };
        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;
        if audio_sink.is_none() {
            audio_sink.replace(AudioSink::new(spec, duration, device));
        }
        audio_sink.as_mut().unwrap().write(decoded);
    };

    let finalize = decoder.finalize();

    if let Some(verify_ok) = finalize.verify_ok {
        if verify_ok {
            eprintln!("Verify OK!")
        } else {
            eprintln!("Verify not OK!")
        }
    }

    result
}

pub struct AudioSink {
    producer: Producer<f32>,
    stream: Stream,
    buffer: SampleBuffer<f32>,
    preprocess: Vec<f32>,
}

impl AudioSink {
    pub fn new(spec: SignalSpec, duration: u64, device: &Device) -> Self {
        let channels = spec.channels.count();
        let sample_rate = spec.rate;
        let audio_buffer_size = (250 * sample_rate as usize * channels) / 1000;
        let audio_buffer = SpscRb::new(audio_buffer_size);
        let (producer, consumer) = (audio_buffer.producer(), audio_buffer.consumer());

        let config = StreamConfig {
            channels: channels as u16,
            sample_rate: SampleRate(sample_rate),
            buffer_size: BufferSize::Default,
        };
        let stream = device
            .build_output_stream(
                &config,
                move |output: &mut [f32], _: &OutputCallbackInfo| {
                    let offset = consumer.read(output).unwrap_or(0);
                    output[offset..]
                        .iter_mut()
                        .for_each(|sample| *sample = f32::MID);
                },
                |_| {},
            )
            .unwrap();
        stream.play().unwrap();
        let buffer = SampleBuffer::<f32>::new(duration, spec);
        let preprocess: Vec<f32> = vec![0.0; buffer.capacity()];
        Self {
            producer,
            stream,
            buffer,
            preprocess,
        }
    }

    pub fn write(&mut self, buffer: AudioBufferRef) {
        self.buffer.copy_interleaved_ref(buffer);

        self.preprocess.copy_from_slice(self.buffer.samples());
        self.preprocess.iter_mut().for_each(|sample| {
            *sample *= 0.25;
        });

        let mut samples = self.preprocess.as_ref();
        while let Some(offset) = self.producer.write_blocking(samples) {
            samples = &samples[offset..];
        }
    }

    pub fn flush(&mut self) {
        let _ = self.stream.pause();
    }
}

fn main() {
    let file = File::open("assets/discover_universe.flac").unwrap();
    let mut reader = file_reader(file);

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();

    let _ = dbg!(play_track(&mut reader, &device));
}
