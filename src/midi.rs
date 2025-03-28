use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use midir::{MidiInput, MidiInputConnection, MidiInputPort};

/// MIDIコールバックをセットアップする関数
pub fn setup_midi_callback(
    midi_in: MidiInput,
    port: &MidiInputPort,
    current_freq: Arc<Mutex<f32>>,
) -> Result<MidiInputConnection<()>, midir::ConnectError<MidiInput>> {
    // 最後のノートオフ時刻を追跡するための変数
    let last_note_off = Arc::new(Mutex::new(None::<Instant>));
    let last_note_off_clone = Arc::clone(&last_note_off);
    let current_freq_clone = Arc::clone(&current_freq);

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
                        
                        if let Ok(mut freq_lock) = current_freq_clone.lock() {
                            *freq_lock = freq;
                            println!("Updated frequency to {:.2}Hz", freq);
                        }
                        // ノートオン時は最後のノートオフ時刻をNoneに設定
                        if let Ok(mut last_note_off) = last_note_off_clone.lock() {
                            *last_note_off = None;
                        }
                    } else { // Note On with velocity 0 (treated as Note Off)
                        println!("Note Off (velocity 0): note={}", note);
                        // 最後のノートオフ時刻を更新
                        if let Ok(mut last_note_off) = last_note_off_clone.lock() {
                            *last_note_off = Some(Instant::now());
                        }
                    }
                }
                0x80 => { // Note Off
                    println!("Note Off: note={}", note);
                    // 最後のノートオフ時刻を更新
                    if let Ok(mut last_note_off) = last_note_off_clone.lock() {
                        *last_note_off = Some(Instant::now());
                    }
                }
                _ => {
                    println!("Other MIDI message: status={:02X}", status);
                }
            }
        }
    };

    // 音声停止用のスレッドを開始
    let current_freq_clone = Arc::clone(&current_freq);
    let last_note_off_clone = Arc::clone(&last_note_off);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(50)); // チェック間隔を50msに短縮
            if let Ok(last_note_off) = last_note_off_clone.lock() {
                if let Some(time) = *last_note_off {
                    if time.elapsed() > Duration::from_millis(100) { // 遅延を100msに短縮
                        if let Ok(mut freq_lock) = current_freq_clone.lock() {
                            if *freq_lock != 0.0 {
                                *freq_lock = 0.0;
                                println!("Auto-stopped sound after 100ms of no notes");
                            }
                        }
                    }
                }
            }
        }
    });

    // MIDI接続を確立
    midi_in.connect(port, "rust_synth", callback, ())
} 