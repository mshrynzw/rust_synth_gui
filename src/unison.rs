use std::sync::{Arc, Mutex};

use crate::oscillator::{Waveform, generate_waveform, OscillatorSettings};

/// Unisonの設定を表す構造体
#[derive(Clone, Debug)]
pub struct UnisonSettings {
    /// Unisonの数（1-8）
    pub voices: usize,
    /// デチューン量（0から100セント）
    pub detune: f32,
    /// 波形タイプ
    pub waveform: Waveform,
}

impl Default for UnisonSettings {
    fn default() -> Self {
        Self {
            voices: 3,
            detune: 0.1,
            waveform: Waveform::default(),
        }
    }
}

/// Unison音声を生成する関数
pub fn generate_unison(
    settings: &UnisonSettings,
    base_freq: f32,
    t: f32,
    sample_rate: f32,
    osc_settings: &OscillatorSettings,
) -> f32 {
    if settings.voices == 1 {
        // ユニゾンなしの場合は単純に波形を生成
        return generate_waveform(settings.waveform, base_freq, t, sample_rate, osc_settings);
    }

    let mut sum = 0.0;
    let detune_step = settings.detune / (settings.voices - 1) as f32;

    // 各ボイスの波形を生成して合成
    for i in 0..settings.voices {
        let detune_amount = detune_step * i as f32 - settings.detune / 2.0;
        let freq = base_freq * (1.0 + detune_amount);
        let value = generate_waveform(settings.waveform, freq, t, sample_rate, osc_settings);
        sum += value;
    }

    // 平均を取って出力
    sum / settings.voices as f32
}

/// Unisonの設定を管理する構造体
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

    pub fn set_voices(&self, voices: usize) {
        if let Ok(mut settings) = self.settings.lock() {
            settings.voices = voices.clamp(1, 8);
        }
    }

    pub fn set_detune(&self, detune: f32) {
        if let Ok(mut settings) = self.settings.lock() {
            settings.detune = detune.clamp(0.0, 100.0);
        }
    }

    pub fn set_waveform(&self, waveform: Waveform) {
        if let Ok(mut settings) = self.settings.lock() {
            settings.waveform = waveform;
        }
    }
} 