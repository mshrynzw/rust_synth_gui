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
pub fn generate_waveform(waveform: Waveform, frequency: f32, t: f32) -> f32 {
    match waveform {
        Waveform::Sine => (2.0 * PI * frequency * t).sin(),
        Waveform::Triangle => {
            let period = 1.0 / frequency;
            let t = t % period;
            let half_period = period / 2.0;
            if t < half_period {
                (t / half_period) * 2.0 - 1.0
            } else {
                -((t - half_period) / half_period) * 2.0 + 1.0
            }
        }
        Waveform::Square => {
            let period = 1.0 / frequency;
            let t = t % period;
            if t < period / 2.0 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Sawtooth => {
            let period = 1.0 / frequency;
            let t = t % period;
            (t / period) * 2.0 - 1.0
        }
    }
} 