use std::sync::{Arc, Mutex};
use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use crate::envelope::Envelope;

/// MIDIコールバックをセットアップする関数
pub fn setup_midi_callback(
    midi_in: MidiInput,
    port: &MidiInputPort,
    current_freq: Arc<Mutex<f32>>,
    envelope: Arc<Mutex<Envelope>>,
) -> Result<MidiInputConnection<()>, midir::ConnectError<MidiInput>> {
    // MIDIメッセージを処理するコールバック関数
    let callback = move |_stamp_ms: u64, message: &[u8], _: &mut ()| {
        // MIDIメッセージの長さが3バイト以上あることを確認
        if message.len() >= 3 {
            let status = message[0];
            let note = message[1];
            let velocity = message[2];

            // Note On メッセージ（0x90）の場合
            if status == 0x90 && velocity > 0 {
                // MIDIノート番号から周波数を計算（A4 = 440Hz）
                let freq = 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
                println!("MIDI message: status={}, note={}, velocity={}", status, note, velocity);
                println!("Updated frequency to {:.2}Hz", freq);

                // 周波数を更新
                if let Ok(mut freq_lock) = current_freq.lock() {
                    *freq_lock = freq;
                }

                // エンベロープを開始
                if let Ok(mut env) = envelope.lock() {
                    env.start();
                }
            }
            // Note Off メッセージ（0x80）または Note On with velocity 0 の場合
            else if status == 0x80 || (status == 0x90 && velocity == 0) {
                println!("Note off: note={}", note);
                // 周波数を0に設定（音を停止）
                if let Ok(mut freq_lock) = current_freq.lock() {
                    *freq_lock = 0.0;
                }

                // エンベロープを終了
                if let Ok(mut env) = envelope.lock() {
                    env.end();
                }
            }
        }
    };

    // MIDIポートに接続
    let connection = midi_in.connect(port, "rust_synth", callback, ())?;

    Ok(connection)
} 