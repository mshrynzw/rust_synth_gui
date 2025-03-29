use std::sync::{Arc, Mutex};
use crate::oscillator::Waveform;

/// ユニゾン設定を表す構造体
#[derive(Debug, Clone, Copy)]
pub struct UnisonSettings {
    pub voices: u8,      // ボイス数（1-8）
    pub detune: f32,     // デチューン量（セント単位）
    pub waveform: Waveform, // 波形の種類
}

impl Default for UnisonSettings {
    fn default() -> Self {
        Self {
            voices: 1,
            detune: 20.0,
            waveform: Waveform::Sine,
        }
    }
}

/// ユニゾン設定を管理する構造体
pub struct UnisonManager {
    settings: Arc<Mutex<UnisonSettings>>,
}

impl UnisonManager {
    pub fn new() -> Self {
        Self {
            settings: Arc::new(Mutex::new(UnisonSettings::default())),
        }
    }

    pub fn get_settings(&self) -> Arc<Mutex<UnisonSettings>> {
        Arc::clone(&self.settings)
    }

    pub fn set_voices(&self, voices: u8) {
        if let Ok(mut settings) = self.settings.lock() {
            settings.voices = voices.clamp(1, 8);
        }
    }

    pub fn set_detune(&self, detune: f32) {
        if let Ok(mut settings) = self.settings.lock() {
            settings.detune = detune.clamp(-100.0, 100.0);
        }
    }

    pub fn set_waveform(&self, waveform: Waveform) {
        if let Ok(mut settings) = self.settings.lock() {
            settings.waveform = waveform;
        }
    }
} 