use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::unison::{UnisonManager, generate_unison};
use crate::envelope::Envelope;
use crate::oscillator::OscillatorSettings;

/// サイン波を生成してスピーカーから再生する関数
pub fn play_sine_wave(
    initial_freq: f32,
    current_freq: Arc<Mutex<f32>>,
    unison_manager: Arc<UnisonManager>,
    envelope: Arc<Mutex<Envelope>>,
    oscillator_settings: &OscillatorSettings,
) -> cpal::Stream {
    // デフォルトのホストを取得
    let host = cpal::default_host();
    // デフォルトの出力デバイスを取得
    let device = host.default_output_device().expect("No output device available");
    // デフォルトの出力フォーマットを取得
    let config = device.default_output_config().expect("Failed to get default output config");
    println!("Starting audio stream at {}Hz", config.sample_rate().0);

    // サンプルレートを取得
    let sample_rate = config.sample_rate().0 as f32;

    // エンベロープのサンプルレートを設定
    if let Ok(mut env) = envelope.lock() {
        env.set_sample_rate(sample_rate);
    }

    // 時間変数（サンプル数として保持）
    let mut t = 0u64;
    // 最後に有効だった周波数を保持
    let last_freq = Arc::new(Mutex::new(initial_freq));

    // オシレータ設定をクローン
    let oscillator_settings = oscillator_settings.clone();
    let oscillator_settings = Arc::new(oscillator_settings);
    let oscillator_settings_clone = Arc::clone(&oscillator_settings);

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

                // 周波数が有効な場合は保存
                if freq > 0.0 {
                    if let Ok(mut last_freq_lock) = last_freq.lock() {
                        *last_freq_lock = freq;
                    }
                }

                // Unison設定を取得
                let unison_settings = if let Ok(settings) = unison_manager.get_settings().try_lock() {
                    settings.clone()
                } else {
                    return;
                };

                // 現在の周波数または最後の有効な周波数を取得
                let current_freq = if freq <= 0.0 {
                    if let Ok(last_freq_lock) = last_freq.lock() {
                        *last_freq_lock
                    } else {
                        initial_freq
                    }
                } else {
                    freq
                };

                // バッファの開始時と終了時のエンベロープ値を取得
                let (start_value, end_value) = if let Ok(mut env) = envelope.lock() {
                    let start = env.get_value();
                    env.update((data.len() as f32) / sample_rate);
                    let end = env.get_value();
                    (start, end)
                } else {
                    (0.0, 0.0)
                };

                // バッファの長さを事前に取得
                let buffer_len = data.len() as f32;

                // 各サンプルを生成
                for (i, sample) in data.iter_mut().enumerate() {
                    // エンベロープ値を線形補間
                    let t_factor = i as f32 / buffer_len;
                    let envelope_value = start_value + (end_value - start_value) * t_factor;

                    if envelope_value > 0.0 {
                        // 時間を秒単位に変換（浮動小数点の精度を考慮）
                        let t_seconds = (t as f32) / sample_rate;
                        
                        // Unison音声を生成
                        let waveform_value = generate_unison(
                            &unison_settings,
                            current_freq,
                            t_seconds,
                            sample_rate,
                            &oscillator_settings_clone,
                        );

                        // 波形とエンベロープを掛け合わせる
                        *sample = waveform_value * envelope_value;
                    } else {
                        // エンベロープの値が0の場合は無音を出力
                        *sample = 0.0;
                    }
                    
                    // 時間を進める（サンプル数として）
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