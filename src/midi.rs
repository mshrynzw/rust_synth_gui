use std::sync::{Arc, Mutex};
use midir::{MidiInput, MidiInputConnection, MidiInputPort};

/// MIDIコールバックをセットアップする関数
pub fn setup_midi_callback(
    midi_in: MidiInput,
    port: &MidiInputPort,
    current_freq: Arc<Mutex<f32>>,
) -> Result<MidiInputConnection<()>, midir::ConnectError<MidiInput>> {
    // MIDIコールバックを定義
    let callback = move |stamp_ms: u64, message: &[u8], _: &mut ()| {
        println!("Raw MIDI message at {}ms: {:02X?}", stamp_ms, message);
        
        if message.len() >= 3 {
            let status = message[0];
            let note = message[1];
            let velocity = message[2];
            
            println!("MIDI message: status={:02X}, note={}, velocity={}", status, note, velocity);
            
            match status & 0xF0 {
                0x90 => { // Note On
                    if velocity > 0 { // ベロシティ > 0
                        // 周波数を更新（MIDIノート番号から周波数に変換）
                        let freq = 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
                        println!("Note On: note={}, freq={:.2}Hz", note, freq);
                        
                        if let Ok(mut freq_lock) = current_freq.lock() {
                            *freq_lock = freq;
                            println!("Updated frequency to {:.2}Hz", freq);
                        } else {
                            println!("Failed to lock current_freq");
                        }
                    }
                }
                0x80 => { // Note Off
                    println!("Note Off: note={}", note);
                }
                _ => {
                    println!("Other MIDI message: status={:02X}", status);
                }
            }
        }
    };

    // MIDI接続を確立
    midi_in.connect(port, "rust_synth", callback, ())
} 