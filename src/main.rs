use std::fs::File;

use cpal::traits::StreamTrait;
use crossterm::{
    event::{read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use photon::core::audio::SamplesInMemory;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let file = File::open("assets/mobius.mp3")?;
    let samples = SamplesInMemory::try_from_file(file)?;
    let stream = photon::core::playback::initialize(samples)?;

    let mut playing = false;

    enable_raw_mode()?;

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

        if event == Event::Key(KeyCode::Char('q').into()) {
            break;
        };
    }

    disable_raw_mode()?;

    Ok(())
}
