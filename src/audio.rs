use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::Arc;
use std::sync::Mutex;

use crate::unison::UnisonManager;
use crate::envelope::EnvelopeManager;
use crate::oscillator::generate_waveform;

/// サイン波を再生する関数
pub fn play_sine_wave(
    _initial_freq: f32,  // 現在は使用していないが、将来の拡張のために残す
    current_freq: Arc<Mutex<f32>>,
    unison_manager: Arc<UnisonManager>,
    envelope_manager: Arc<Mutex<EnvelopeManager>>,
) -> cpal::Stream {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("Failed to get default output device");
    let config = device.default_output_config().expect("Failed to get default output config");
    let sample_rate = config.sample_rate().0 as f32;

    // エンベロープマネージャーのサンプルレートを設定
    if let Ok(mut env_manager) = envelope_manager.lock() {
        env_manager.set_sample_rate(sample_rate);
    }

    let err_fn = |err| eprintln!("An error occurred on stream: {}", err);

    let stream = match config.sample_format() {
        SampleFormat::F32 => {
            let mut sample_clock = 0f32;
            device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let freq = *current_freq.lock().unwrap();
                    let mut env_manager = envelope_manager.lock().unwrap();
                    let settings = unison_manager.get_settings().lock().unwrap().clone();

                    for sample in data.iter_mut() {
                        // エンベロープの更新（サンプルレートに基づく時間経過）
                        env_manager.update_all(1.0 / sample_rate);

                        // エンベロープの値を取得
                        let envelope_value = env_manager.get_value(0);

                        // 現在の周波数で波形を生成
                        let mut output = 0.0;
                        for i in 0..settings.voices {
                            let detune_factor = if settings.voices > 1 {
                                let detune_cents = settings.detune * (i as f32 - (settings.voices - 1) as f32 / 2.0) / ((settings.voices - 1) as f32);
                                2.0f32.powf(detune_cents / 1200.0)
                            } else {
                                1.0
                            };
                            let detuned_freq = freq * detune_factor;
                            output += generate_waveform(settings.waveform, sample_clock, detuned_freq);
                        }
                        output /= settings.voices as f32;

                        // エンベロープを適用
                        *sample = output * envelope_value;

                        // サンプルクロックを更新
                        sample_clock += 1.0 / sample_rate;
                    }
                },
                err_fn,
                None,
            )
        },
        _ => panic!("Unsupported sample format"),
    }
    .expect("Failed to build output stream");

    stream.play().expect("Failed to start stream");
    stream
} 