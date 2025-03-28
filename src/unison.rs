use std::sync::{Arc, Mutex};

use crate::oscillator::{Waveform, generate_waveform};

/// Unisonの設定を表す構造体
#[derive(Clone, Copy)]
pub struct UnisonSettings {
    /// Unisonの数（1-8）
    pub voices: u8,
    /// デチューン量（0から100セント）
    pub detune: f32,
    /// 波形タイプ
    pub waveform: Waveform,
}

impl Default for UnisonSettings {
    fn default() -> Self {
        Self {
            voices: 1,
            detune: 0.0,
            waveform: Waveform::Sine,
        }
    }
}

/// Unison音声を生成する関数
pub fn generate_unison(
    base_freq: f32,
    settings: UnisonSettings,
    t: f32,
    _sample_rate: f32,
) -> f32 {
    if settings.voices == 0 || settings.voices > 8 {
        return 0.0;
    }

    let mut sum = 0.0;
    let voice_count = settings.voices as f32;
    
    // ボイス数が1の場合は通常の波形を生成
    if settings.voices == 1 {
        return generate_waveform(settings.waveform, base_freq, t);
    }
    
    // 各ボイスを生成
    for i in 0..settings.voices {
        // デチューン量を計算（-detuneから+detuneの範囲で均等に分散）
        let detune_step = (settings.detune * 2.0) / (voice_count - 1.0);
        let detune_amount = -settings.detune + (detune_step * i as f32);
        
        // セントから周波数比に変換
        let detune_ratio = 2.0f32.powf(detune_amount / 1200.0);
        
        // このボイスの周波数を計算
        let freq = base_freq * detune_ratio;
        
        // 波形を生成
        let value = generate_waveform(settings.waveform, freq, t);
        
        // 音量を調整（ボイス数で割って音量を一定に保つ）
        sum += value / voice_count;
    }
    
    sum
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

    pub fn set_voices(&self, voices: u8) {
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