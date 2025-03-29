use std::f32::consts::PI;

/// オシレータの波形タイプを表す列挙型
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Waveform {
    Sine,    // サイン波
    Triangle, // 三角波
    Square,   // 矩形波
    Sawtooth, // ノコギリ波
}

impl Default for Waveform {
    fn default() -> Self {
        Self::Sine
    }
}

/// オシレータの設定を表す構造体
pub struct OscillatorSettings {
    pub oversample_ratio: u32,
    pub filter_alpha: f32,
    pub smoothing_strength: f32,
}

/// 指定された波形を生成する関数（オーバーサンプリング、フィルター、スムージング付き）
pub fn generate_waveform(
    waveform: Waveform,
    frequency: f32,
    t: f32,
    sample_rate: f32,
    settings: &OscillatorSettings,
) -> f32 {
    // オーバーサンプリング用の時間刻み
    let dt = 1.0 / (sample_rate * settings.oversample_ratio as f32);
    let mut sum = 0.0;
    let mut prev_sample = 0.0;

    // オーバーサンプリングによる波形生成
    for i in 0..settings.oversample_ratio {
        let t_oversampled = t + (i as f32 * dt);
        let phase = (t_oversampled * frequency).fract();

        let raw_sample = match waveform {
            Waveform::Sine => {
                // サイン波の計算
                (2.0 * PI * phase).sin()
            }
            Waveform::Triangle => {
                // 三角波の計算（より滑らかな実装）
                let x = phase * 2.0 - 1.0;
                let smoothed = (x.abs() * 2.0 - 1.0).signum();
                smoothed * 0.8 // 振幅を少し抑える
            }
            Waveform::Square => {
                // 矩形波の計算（より滑らかな実装）
                let smoothed = phase.sin().signum();
                smoothed * 0.8 // 振幅を少し抑える
            }
            Waveform::Sawtooth => {
                // ノコギリ波の計算（より滑らかな実装）
                let x = phase * 2.0 - 1.0;
                let smoothed = x - (x.abs() * 2.0 - 1.0).signum() * 0.5;
                smoothed * 0.8 // 振幅を少し抑える
            }
        };

        // フィルターとスムージングを適用
        let filtered = apply_lowpass_filter(raw_sample, prev_sample, settings.filter_alpha);
        let smoothed = apply_smoothing(filtered, settings.smoothing_strength);
        
        sum += smoothed;
        prev_sample = filtered;
    }

    // 平均を取って最終的なサンプルを生成
    sum / settings.oversample_ratio as f32
}

/// 簡単なローパスフィルター
fn apply_lowpass_filter(input: f32, prev_output: f32, filter_alpha: f32) -> f32 {
    // フィルターの効果を強化
    let alpha = filter_alpha * 2.0; // フィルターの強度を2倍に
    prev_output + alpha * (input - prev_output)
}

/// スムージング処理を適用
fn apply_smoothing(input: f32, smoothing_strength: f32) -> f32 {
    // スムージングの効果を強化
    let strength = smoothing_strength * 2.0; // スムージングの強度を2倍に
    let x = input.max(-1.0).min(1.0);
    x * (1.0 - x.abs() * strength)
}