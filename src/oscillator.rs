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

/// 指定された波形を生成する関数
pub fn generate_waveform(waveform: Waveform, frequency: f32, t: f32, sample_rate: f32) -> f32 {
    // 時間を周期で割った余りを使用（浮動小数点の精度を考慮）
    let period = sample_rate / frequency;
    let t = t * sample_rate;
    let t = t % period;

    match waveform {
        Waveform::Sine => {
            // サイン波の計算
            (2.0 * PI * t / period).sin()
        }
        Waveform::Triangle => {
            // 三角波の計算
            let half_period = period * 0.5;
            if t < half_period {
                (t / half_period) * 2.0 - 1.0
            } else {
                -((t - half_period) / half_period) * 2.0 + 1.0
            }
        }
        Waveform::Square => {
            // 矩形波の計算
            if t < period * 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Sawtooth => {
            // ノコギリ波の計算
            (t / period) * 2.0 - 1.0
        }
    }
}