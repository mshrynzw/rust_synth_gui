use std::sync::{Arc, Mutex};
use eframe::{egui, App};
use cpal::Stream;
use midir::MidiInputConnection;

use crate::audio::play_sine_wave;
use crate::midi::setup_midi_callback;

/// アプリの状態を表す構造体
pub struct SynthApp {
    freq: f32, // 再生する周波数（Hz）
    playing: bool, // 音を再生中かどうかのフラグ
    stream_handle: Option<Stream>, // 再生中のストリーム（再生停止に使う）
    midi_connection: Option<MidiInputConnection<()>>, // MIDI接続ハンドル
    last_note: Option<u8>, // 最後に押されたノート番号
    midi_freq: Arc<Mutex<f32>>, // MIDIから設定された周波数（スレッド間共有）
    current_freq: Arc<Mutex<f32>>, // 現在再生中の周波数（スレッド間共有）
    midi_ports: Vec<String>, // 利用可能なMIDIポートのリスト
    selected_port: usize, // 選択されたMIDIポートのインデックス
}

/// アプリのデフォルト初期値を定義（440Hz・再生停止中）
impl Default for SynthApp {
    fn default() -> Self {
        Self {
            freq: 440.0,         // A4（ラ）の周波数
            playing: false,      // 初期状態は再生停止中
            stream_handle: None, // ストリームはまだ存在しない
            midi_connection: None, // MIDI接続はまだ存在しない
            last_note: None,     // 最後に押されたノートはまだない
            midi_freq: Arc::new(Mutex::new(440.0)), // MIDI周波数の初期値
            current_freq: Arc::new(Mutex::new(440.0)), // 現在の周波数の初期値
            midi_ports: Vec::new(), // MIDIポートのリストは空
            selected_port: 0,    // デフォルトは最初のポート
        }
    }
}

/// eframe::App の実装（毎フレーム呼ばれる update 関数など）
impl App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // MIDIから設定された周波数を取得
        if let Ok(midi_freq) = self.midi_freq.try_lock() {
            self.freq = *midi_freq;
            // 再生中の周波数も更新
            if let Ok(mut current_freq) = self.current_freq.try_lock() {
                *current_freq = self.freq;
            }
        }

        // 中央パネルにGUIを描画する
        egui::CentralPanel::default().show(ctx, |ui| {
            // タイトル見出し
            ui.heading("🎹 Rust Synth");

            // MIDIポートの更新と選択UI
            if ui.button("🔄 Refresh MIDI Ports").clicked() {
                // MIDIポートのリストを更新
                if let Ok(midi_in) = midir::MidiInput::new("rust_synth") {
                    let ports = midi_in.ports();
                    self.midi_ports.clear();
                    for port in ports.iter() {
                        if let Ok(port_name) = midi_in.port_name(port) {
                            self.midi_ports.push(port_name);
                        }
                    }
                    println!("Available MIDI ports:");
                    for (i, name) in self.midi_ports.iter().enumerate() {
                        println!("[{}] {}", i, name);
                    }
                }
            }

            // MIDIポート選択コンボボックス
            if !self.midi_ports.is_empty() {
                egui::ComboBox::from_label("MIDI Port")
                    .selected_text(&self.midi_ports[self.selected_port])
                    .show_ui(ui, |ui| {
                        for (i, port_name) in self.midi_ports.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_port, i, port_name);
                        }
                    });
            }

            // MIDI接続ボタン
            if ui.button("🔌 Connect MIDI").clicked() && self.midi_connection.is_none() {
                if let Ok(mut midi_in) = midir::MidiInput::new("rust_synth") {
                    midi_in.ignore(midir::Ignore::None);
                    let ports = midi_in.ports();
                    
                    // 選択されたポートに接続を試みる
                    if let Some(port) = ports.get(self.selected_port) {
                        let port_name = midi_in.port_name(port).unwrap_or_else(|_| "Unknown".to_string());
                        println!("Attempting to connect to MIDI port: {}", port_name);
                        
                        // MIDIコールバックをセットアップ
                        let current_freq = Arc::clone(&self.current_freq);
                        if let Ok(conn) = setup_midi_callback(midi_in, port, current_freq) {
                            println!("MIDI connection established successfully");
                            self.midi_connection = Some(conn);
                        } else {
                            println!("Failed to establish MIDI connection");
                        }
                    } else {
                        println!("Selected MIDI port not available");
                    }
                } else {
                    println!("Failed to create MIDI input");
                }
            }

            // MIDI切断ボタン
            if ui.button("🔌 Disconnect MIDI").clicked() && self.midi_connection.is_some() {
                self.midi_connection = None;
                self.last_note = None;
            }

            // 周波数スライダー（100Hz〜1000Hz）を追加
            ui.add(
                egui::Slider::new(&mut self.freq, 100.0..=1000.0)
                    .text("Frequency (Hz)"),
            );
            // スライダーの値を現在の周波数に反映
            if let Ok(mut current_freq) = self.current_freq.try_lock() {
                *current_freq = self.freq;
            }

            // 再生・停止ボタンを横並びで表示
            ui.horizontal(|ui| {
                // ▶ 再生ボタンが押された & 現在停止中なら
                if ui.button("▶ Play").clicked() && !self.playing {
                    self.playing = true; // 再生状態に変更
                    let freq = self.freq; // 現在の周波数をコピー

                    // サイン波を再生してストリームを保持（current_freqを渡す）
                    let stream = play_sine_wave(freq, Arc::clone(&self.current_freq));
                    self.stream_handle = Some(stream);
                }

                // ⏹ 停止ボタンが押された & 再生中なら
                if ui.button("⏹ Stop").clicked() && self.playing {
                    self.playing = false;      // 停止状態に変更
                    self.stream_handle = None; // ストリームを破棄（再生停止）
                }
            });

            // 現在の周波数をラベルとして表示
            ui.label(format!("Current frequency: {:.1} Hz", self.freq));
        });
    }
} 