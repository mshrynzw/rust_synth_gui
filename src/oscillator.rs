/// 波形の種類を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq)]
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
pub fn generate_waveform(waveform: Waveform, t: f32, freq: f32) -> f32 {
    let phase = (t * freq * 2.0 * std::f32::consts::PI) % (2.0 * std::f32::consts::PI);

    match waveform {
        Waveform::Sine => phase.sin(),
        Waveform::Triangle => {
            let normalized_phase = phase / (2.0 * std::f32::consts::PI);
            if normalized_phase < 0.5 {
                4.0 * normalized_phase - 1.0
            } else {
                3.0 - 4.0 * normalized_phase
            }
        }
        Waveform::Square => {
            if phase < std::f32::consts::PI {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Sawtooth => {
            let normalized_phase = phase / (2.0 * std::f32::consts::PI);
            2.0 * normalized_phase - 1.0
        }
    }
}