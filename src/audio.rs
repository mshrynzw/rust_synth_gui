use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::unison::{UnisonManager, generate_unison};

/// サイン波を生成してスピーカーから再生する関数
pub fn play_sine_wave(
    initial_freq: f32,
    current_freq: Arc<Mutex<f32>>,
    unison_manager: Arc<UnisonManager>,
) -> cpal::Stream {
    // デフォルトのホストを取得
    let host = cpal::default_host();
    // デフォルトの出力デバイスを取得
    let device = host.default_output_device().expect("No output device available");
    // デフォルトの出力フォーマットを取得
    let config = device.default_output_config().expect("Failed to get default output config");
    println!("Starting audio stream at {}Hz", config.sample_rate().0);

    // 時間変数（サンプル数として保持）
    let mut t = 0u64;

    // オーディオストリームを構築
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // 現在の周波数を取得
                let freq = if let Ok(freq_lock) = current_freq.try_lock() {
                    *freq_lock
                } else {
                    initial_freq
                };

                // Unison設定を取得
                let unison_settings = if let Ok(settings) = unison_manager.get_settings().try_lock() {
                    *settings
                } else {
                    return;
                };

                // 各サンプルを生成
                for sample in data.iter_mut() {
                    // 時間を秒単位に変換（オーバーフロー対策）
                    let t_seconds = (t as f32) / config.sample_rate().0 as f32;
                    
                    // Unison音声を生成
                    *sample = generate_unison(
                        freq,
                        unison_settings,
                        t_seconds,
                        config.sample_rate().0 as f32,
                    );
                    
                    t = t.wrapping_add(1);
                }
            },
            move |err| {
                eprintln!("Error in output stream: {}", err);
            },
            None,
        ),
        _ => panic!("Unsupported sample format"),
    }
    .expect("Failed to build output stream");

    // ストリームを開始
    stream.play().expect("Failed to start output stream");

    stream
} 