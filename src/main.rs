use std::{fs::File, time::Instant};

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

pub fn play_track(reader: &mut Box<dyn FormatReader>, device: &Device) {
    let track = reader.default_track().unwrap();

    let decoder_options = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_options)
        .unwrap();

    let mut audio_sink = None;

    let reference = Instant::now();
    loop {
        let packet = reader.next_packet().unwrap();
        let decoded = decoder.decode(&packet).unwrap();
        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;
        if audio_sink.is_none() {
            audio_sink.replace(AudioSink::new(spec, duration, device));
        }
        println!("{:?} has elapsed", reference.elapsed());
        audio_sink.as_mut().unwrap().write(decoded);
    }
}

pub struct AudioSink {
    producer: Producer<f32>,
    stream: Stream,
    buffer: SampleBuffer<f32>,
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
        Self {
            producer,
            stream,
            buffer,
        }
    }

    pub fn write(&mut self, buffer: AudioBufferRef) {
        self.buffer.copy_interleaved_ref(buffer);
        let mut samples = self.buffer.samples();
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

    play_track(&mut reader, &device);
}
