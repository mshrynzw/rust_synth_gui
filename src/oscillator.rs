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
pub fn generate_waveform(waveform: Waveform, frequency: f32, t: f32, _sample_rate: f32) -> f32 {
    // 位相を計算（0.0から1.0の範囲）
    // 時間を1秒で割った余りを使用して、位相の蓄積を防ぐ
    let phase = (t % 1.0) * frequency;

    match waveform {
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
    }
}