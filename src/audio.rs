use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::unison::{UnisonManager, generate_unison};

/// サイン波を生成してスピーカーから再生する関数
pub fn play_sine_wave(_freq: f32, current_freq: Arc<Mutex<f32>>, unison_manager: Arc<UnisonManager>) -> cpal::Stream {
    // デフォルトのオーディオホストを取得（WindowsならWASAPIなど）
    let host = cpal::default_host();

    // 出力デバイス（例：スピーカー）を取得
    let device = host.default_output_device().expect("No output device found");

    // 出力デバイスの設定（例：44100Hz, f32型など）を取得
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate().0 as f32;

    println!("Audio stream started with sample rate: {}Hz", sample_rate);

    // 音の時間位置を追跡するための変数を作成（スレッド安全）
    let t = Arc::new(Mutex::new(0.0_f32));
    let t_clone = Arc::clone(&t);

    // current_freqのクローンを作成
    let current_freq_clone = Arc::clone(&current_freq);

    // Unison設定のクローンを作成
    let unison_settings = unison_manager.get_settings();

    // build_output_stream にクロージャを直接渡すことでライフタイムエラーを回避
    let stream = device
        .build_output_stream(
            &config.into(), // 出力設定（サンプルレートなど）
            move |data: &mut [f32], _info| {
                // t をロックして使う（他スレッドと競合しないように）
                let mut t = t_clone.lock().unwrap();
                // 現在の周波数を取得
                let freq = if let Ok(freq_lock) = current_freq_clone.lock() {
                    *freq_lock
                } else {
                    0.0 // デフォルト周波数（音なし）
                };

                // Unison設定を取得
                let settings = if let Ok(settings) = unison_settings.lock() {
                    *settings
                } else {
                    Default::default()
                };

                // 出力バッファにサンプルを書き込む
                for sample in data.iter_mut() {
                    if freq > 0.0 {
                        // Unison音声を生成
                        let value = generate_unison(freq, settings, *t, sample_rate);
                        *sample = value * 0.2; // 0.2 = 音量
                    } else {
                        *sample = 0.0; // 音なし
                    }
                    *t += 1.0 / sample_rate; // 時間を進める
                }
            },
            move |err| {
                // エラーハンドラ：ストリームエラーを出力
                eprintln!("Stream error: {}", err);
            },
            None, // 出力レイアウトの指定（Noneでデフォルト）
        )
        .unwrap(); // エラーハンドリング（失敗したら panic）

    stream.play().unwrap(); // ストリームの再生開始
    stream
} 