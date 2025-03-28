mod app;
mod audio;
mod midi;

// 標準ライブラリから、円周率（PI）を使用
use std::f32::consts::PI;

// スレッド間で共有・同期するために、Arc（参照カウント付きポインタ）と Mutex（排他ロック）を使用
use std::sync::{Arc, Mutex};

// オーディオ出力のために、cpalクレートのトレイトをインポート
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// GUIアプリの構築のために、eframe（eguiベース）をインポート
use eframe::{egui, App};

// MIDI関連のインポート
use midir::{MidiInput, MidiInputConnection};

use eframe::NativeOptions;

/// アプリケーションのエントリーポイント（GUIの初期化）
fn main() -> Result<(), eframe::Error> {
    // ウィンドウ設定を定義（タイトルとウィンドウサイズ）
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 200.0])  // ウィンドウの初期サイズ
            .with_title("Rust Synth"),        // ウィンドウタイトル
        ..Default::default()
    };

    // アプリケーションを起動（`SynthApp` を中身として実行）
    eframe::run_native(
        "Rust Synth", // 内部的なアプリ名
        options,      // ウィンドウ設定
        Box::new(|_cc| Box::new(app::SynthApp::default())), // アプリケーションの初期化クロージャ
    )
}

/// アプリの状態を表す構造体
struct SynthApp {
    freq: f32, // 再生する周波数（Hz）
    playing: bool, // 音を再生中かどうかのフラグ
    stream_handle: Option<cpal::Stream>, // 再生中のストリーム（再生停止に使う）
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
                if let Ok(midi_in) = MidiInput::new("rust_synth") {
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
                if let Ok(mut midi_in) = MidiInput::new("rust_synth") {
                    midi_in.ignore(midir::Ignore::None);
                    let ports = midi_in.ports();
                    
                    // 選択されたポートに接続を試みる
                    if let Some(port) = ports.get(self.selected_port) {
                        let port_name = midi_in.port_name(port).unwrap_or_else(|_| "Unknown".to_string());
                        println!("Attempting to connect to MIDI port: {}", port_name);
                        
                        // MIDIコールバック用の周波数共有変数を作成
                        let current_freq = Arc::clone(&self.current_freq);
                        
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
                        match midi_in.connect(port, "rust_synth", callback, ()) {
                            Ok(conn) => {
                                println!("MIDI connection established successfully");
                                self.midi_connection = Some(conn);
                            }
                            Err(err) => {
                                println!("Failed to establish MIDI connection: {}", err);
                            }
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

/// サイン波を生成してスピーカーから再生する関数
fn play_sine_wave(_freq: f32, current_freq: Arc<Mutex<f32>>) -> cpal::Stream {
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
                    440.0 // デフォルト周波数
                };

                // 出力バッファにサンプルを書き込む
                for sample in data.iter_mut() {
                    // サイン波の式 sin(2πft)
                    let value = (2.0 * PI * freq * *t).sin() * 0.2; // 0.2 = 音量
                    *sample = value; // バッファに書き込む
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
