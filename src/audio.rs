//! The audio layer: one track at a time on the default output device.
//!
//! The device is opened on first play, so machines without sound (and the tests) never touch it.
//! Every failure is reported as text for the operator; nothing here panics.

use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};

use crate::model::Id;

#[derive(Default)]
pub struct AudioEngine {
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
    pub current: Option<Id>,
    pub duration: Option<Duration>,
    pub error: Option<String>,
    volume: f32,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self { volume: 1.0, ..Default::default() }
    }

    fn device(&mut self) -> Result<&MixerDeviceSink, String> {
        if self.sink.is_none() {
            let mut sink = DeviceSinkBuilder::open_default_sink().map_err(|e| format!("No audio output: {e}"))?;
            sink.log_on_drop(false);
            self.sink = Some(sink);
        }
        self.sink.as_ref().ok_or_else(|| "No audio output".to_string())
    }

    /// Start a file from the beginning, replacing whatever was playing.
    pub fn play(&mut self, id: Id, path: &str) -> Result<(), String> {
        self.stop();
        let file = File::open(path).map_err(|e| format!("{path}: {e}"))?;
        let decoder = rodio::Decoder::new(BufReader::new(file)).map_err(|e| format!("{path}: {e}"))?;
        let duration = decoder.total_duration();
        let player = Player::connect_new(self.device()?.mixer());
        player.set_volume(self.volume);
        player.append(decoder);
        self.player = Some(player);
        self.current = Some(id);
        self.duration = duration;
        self.error = None;
        Ok(())
    }

    /// Play `id`, recording any failure in `error` for the UI.
    pub fn play_reporting(&mut self, id: Id, path: &str) {
        if let Err(e) = self.play(id, path) {
            self.error = Some(e);
        }
    }

    pub fn stop(&mut self) {
        if let Some(p) = self.player.take() {
            p.stop();
        }
        self.current = None;
        self.duration = None;
    }

    pub fn toggle_pause(&self) {
        if let Some(p) = &self.player {
            if p.is_paused() { p.play() } else { p.pause() }
        }
    }

    pub fn is_paused(&self) -> bool {
        self.player.as_ref().is_some_and(Player::is_paused)
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| !p.empty() && !p.is_paused())
    }

    pub fn position(&self) -> Duration {
        self.player.as_ref().map(Player::get_pos).unwrap_or_default()
    }

    pub fn seek(&self, to: Duration) {
        if let Some(p) = &self.player {
            let _ = p.try_seek(to);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.5);
        if let Some(p) = &self.player {
            p.set_volume(self.volume);
        }
    }

    /// Drop a track that has played to its end, so the audio clear button goes dark.
    pub fn tick(&mut self) {
        if self.player.as_ref().is_some_and(Player::empty) {
            self.stop();
        }
    }
}
