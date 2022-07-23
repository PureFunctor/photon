use std::{fs::File, sync::Arc};

use anyhow::Context;
use log::info;
use symphonia::core::{
    audio::SampleBuffer,
    codecs::DecoderOptions,
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::Hint,
    sample::Sample,
};

/// An audio file loaded in memory.
#[derive(Debug, Clone)]
pub struct SamplesInMemory {
    /// A read-only view of samples.
    pub samples: Arc<Vec<f32>>,
    /// The number of audio channels.
    pub channels: usize,
    /// The sample rate of the audio.
    pub sample_rate: usize,
}

impl SamplesInMemory {
    /// Try to decode a file onto memory.
    pub fn try_from_file(file: File) -> anyhow::Result<Self> {
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

        let _sample_count = track.codec_params.n_frames.unwrap() * 2;

        let mut samples = vec![];

        let (channels, sample_rate, mut sample_buffer) = {
            let packet = reader
                .next_packet()
                .context("while reading the next packet")?;
            let decoded = decoder
                .decode(&packet)
                .context("while decoding the next packet")?;
            let duration = decoded.capacity() as u64;
            let spec = *decoded.spec();
            let mut sample_buffer = SampleBuffer::<f32>::new(duration, spec);
            sample_buffer.copy_interleaved_ref(decoded);
            samples.extend_from_slice(sample_buffer.samples());
            let channels = spec.channels.count();
            let sample_rate = spec.rate as usize;
            (channels, sample_rate, sample_buffer)
        };

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
        };

        let samples = Arc::new(samples);

        Ok(Self {
            samples,
            channels,
            sample_rate,
        })
    }

    /// Copy samples from a start offset onto a buffer.
    ///
    /// # Panics
    ///
    /// Panics if the start offset is greater than the length of the
    /// samples. This usually means that the track has already ended,
    /// and as such, must be checked by the caller.
    pub fn copy_from_onto(&self, start_offset: usize, buffer: &mut [f32]) {
        if start_offset >= self.samples.len() {
            panic!("start_offset is greater than the sample length!");
        }
        let end_offset = start_offset + buffer.len();
        if end_offset > self.samples.len() {
            let overflow = end_offset - self.samples.len();
            let end_offset = end_offset - overflow;
            let total_len = end_offset - start_offset;
            buffer[..total_len].copy_from_slice(&self.samples[start_offset..end_offset]);
            for sample in buffer.iter_mut().skip(total_len) {
                *sample = f32::MID;
            }
        } else {
            buffer.copy_from_slice(&self.samples[start_offset..end_offset]);
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::SamplesInMemory;

    #[test]
    pub fn copy_test_equal() {
        let samples = Arc::new(vec![1.0; 8]);
        let channels = 2;
        let sample_rate = 44100;
        let in_memory = SamplesInMemory {
            samples,
            channels,
            sample_rate,
        };
        let mut buffer = vec![0.0; 8];
        let expected = vec![1.0; 8];
        in_memory.copy_from_onto(0, &mut buffer);
        assert_eq!(buffer, expected);
    }

    #[test]
    pub fn copy_test_over() {
        let samples = Arc::new(vec![1.0; 8]);
        let channels = 2;
        let sample_rate = 44100;
        let in_memory = SamplesInMemory {
            samples,
            channels,
            sample_rate,
        };
        let mut buffer = vec![0.0; 8];
        let expected = vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        in_memory.copy_from_onto(4, &mut buffer);
        assert_eq!(buffer, expected);
    }
}
