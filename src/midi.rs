use std::sync::{Arc, Mutex};
use midir::{MidiInput, MidiInputConnection};
use crate::envelope::EnvelopeManager;

/// MIDIコールバックをセットアップする関数
pub fn setup_midi_callback(
    midi_in: MidiInput,
    port: &midir::MidiInputPort,
    current_freq: Arc<Mutex<f32>>,
    envelope_manager: Arc<Mutex<EnvelopeManager>>,
) -> Result<MidiInputConnection<()>, midir::ConnectError<MidiInput>> {
    // MIDIコールバック関数を定義
    let callback = move |_stamp: u64, message: &[u8], _: &mut ()| {
        // メッセージの種類を判定
        match message[0] & 0xF0 {
            0x90 => { // Note On
                let note = message[1];
                let velocity = message[2];
                if velocity > 0 {
                    // ノート番号から周波数を計算（A4 = 440Hz）
                    let freq = 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
                    if let Ok(mut freq_lock) = current_freq.lock() {
                        *freq_lock = freq;
                    }
                    // エンベロープを開始
                    if let Ok(mut env_manager) = envelope_manager.lock() {
                        env_manager.start_all(note as u32);
                    }
                }
            }
            0x80 => { // Note Off
                let _note = message[1];
                // エンベロープを終了
                if let Ok(mut env_manager) = envelope_manager.lock() {
                    env_manager.end_all();
                }
            }
            _ => {}
        }
    };

    // MIDIポートに接続
    midi_in.connect(port, "rust_synth", callback, ())
} 