use anyhow::Context;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, StreamConfig,
};
use crossterm::{
    event::{read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use log::info;
use std::{
    fs::File,
    sync::{Arc, Mutex},
    time::Duration,
};
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
        if start >= self.samples.len() || end >= self.samples.len() {
            return;
        }
        output.copy_from_slice(&self.samples[start..end]);
    }

    pub fn length(&self) -> Duration {
        Duration::from_secs_f64(self.samples.len() as f64 / self.sample_rate as f64 / 2.0)
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let file = File::open("assets/mobius.mp3").unwrap();
    let audio = AudioInMemory::from_file(file)?;

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config = StreamConfig {
        channels: audio.channels as u16,
        sample_rate: SampleRate(audio.sample_rate as u32),
        buffer_size: BufferSize::Default,
    };

    let user_events = Arc::new(Mutex::new(Vec::new()));

    let stream = {
        let audio = audio.clone();
        let user_events = user_events.clone();
        let mut cursor = 0;

        let mut start_position = None;
        let mut repeat_delta = None;

        let repeat_8th =
            (60.0 / 230.0 / 2.0 * audio.sample_rate as f64 * audio.channels as f64) as usize;
        let repeat_16th =
            (60.0 / 230.0 / 4.0 * audio.sample_rate as f64 * audio.channels as f64) as usize;

        let mut retrigger_cursor = 0;

        device.build_output_stream(
            &config,
            move |output: &mut [f32], _| {
                while let Some(value) = user_events.lock().unwrap().pop() {
                    if value == 0 {
                        start_position.replace(cursor);
                        retrigger_cursor = cursor;
                        repeat_delta.replace(repeat_8th);
                    } else if value == 1 {
                        start_position.replace(cursor);
                        retrigger_cursor = cursor;
                        repeat_delta.replace(repeat_16th);
                    } else if value == 2 {
                        start_position = None;
                        repeat_delta = None;
                        retrigger_cursor = 0;
                    }
                }
                if start_position.is_some() {
                    audio.onto(retrigger_cursor, output);
                    retrigger_cursor += output.len();
                    if retrigger_cursor - start_position.unwrap() >= repeat_delta.unwrap() {
                        retrigger_cursor = start_position.unwrap() + retrigger_cursor
                            - start_position.unwrap()
                            - repeat_delta.unwrap();
                    }
                } else {
                    audio.onto(cursor, output);
                }
                cursor += output.len();
                output.iter_mut().for_each(|sample| *sample *= 0.25);
            },
            |_| {},
        )
    }?;

    stream.pause()?;

    println!("SPACEBAR - Play/Pause");
    println!("S - Retrigger Off");
    println!("D - Retrigger 8th");
    println!("F - Retrigger 16th");
    println!("Q - Quit");

    enable_raw_mode()?;

    let mut playing = false;

    loop {
        let event = read()?;

        if event == Event::Key(KeyCode::Char(' ').into()) {
            if !playing {
                stream.play()?;
            } else {
                stream.pause()?;
            }
            playing = !playing;
        };

        if event == Event::Key(KeyCode::Char('d').into()) {
            user_events.lock().unwrap().push(0);
        };

        if event == Event::Key(KeyCode::Char('f').into()) {
            user_events.lock().unwrap().push(1);
        };

        if event == Event::Key(KeyCode::Char('s').into()) {
            user_events.lock().unwrap().push(2);
        };

        if event == Event::Key(KeyCode::Char('q').into()) {
            break;
        };
    }

    disable_raw_mode()?;

    Ok(())
}
