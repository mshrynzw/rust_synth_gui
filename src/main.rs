mod app;
mod audio;
mod midi;
mod unison;
mod oscillator;
mod envelope;

use crate::oscillator::{Waveform, OscillatorSettings};
use crate::unison::UnisonManager;
use crate::envelope::Envelope;

// スレッド間で共有・同期するために、Arc（参照カウント付きポインタ）と Mutex（排他ロック）を使用
use std::sync::{Arc, Mutex};

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
            .with_inner_size([500.0, 500.0])  // ウィンドウの初期サイズ
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
    oscillator_settings: OscillatorSettings,
    unison_manager: Arc<UnisonManager>, // Unison設定を管理
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
            oscillator_settings: OscillatorSettings::default(),
            unison_manager: Arc::new(UnisonManager::new()), // Unison設定の初期化
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

            // 現在の周波数をラベルとして表示
            ui.label(format!("Current frequency: {:.1} Hz", self.freq));

            // 音質調整セクション
            ui.collapsing("Sound Quality Settings", |ui| {
                // フィルターアルファ（ローパスフィルターの強度）
                ui.horizontal(|ui| {
                    ui.label("Filter Alpha:");
                    ui.add(
                        egui::Slider::new(&mut self.oscillator_settings.filter_alpha, 0.0..=1.0)
                            .step_by(0.01)
                            .show_value(true),
                    );
                });
                ui.label("Controls the strength of the low-pass filter. Higher values = more filtering.");

                // スムージング強度
                ui.horizontal(|ui| {
                    ui.label("Smoothing Strength:");
                    ui.add(
                        egui::Slider::new(&mut self.oscillator_settings.smoothing_strength, 0.0..=0.5)
                            .step_by(0.01)
                            .show_value(true),
                    );
                });
                ui.label("Controls the amount of waveform smoothing. Higher values = smoother sound.");

                // オーバーサンプリング比率
                ui.horizontal(|ui| {
                    ui.label("Oversample Ratio:");
                    ui.add(
                        egui::Slider::new(&mut self.oscillator_settings.oversample_ratio, 1..=16)
                            .step_by(1.0)
                            .show_value(true),
                    );
                });
                ui.label("Controls the quality of waveform generation. Higher values = less aliasing.");

                if ui.button("Reset to Default").clicked() {
                    self.oscillator_settings = OscillatorSettings::default();
                }
            });

            // Unison設定のセクション
            ui.collapsing("Unison Settings", |ui| {
                // ボイス数のスライダー
                ui.horizontal(|ui| {
                    ui.label("Voices:");
                    ui.add(
                        egui::Slider::new(
                            &mut self.unison_manager.get_settings().lock().unwrap().voices,
                            1..=8,
                        )
                        .step_by(1.0)
                        .show_value(true),
                    );
                });

                // デチューン量のスライダー
                ui.horizontal(|ui| {
                    ui.label("Detune (cents):");
                    ui.add(
                        egui::Slider::new(
                            &mut self.unison_manager.get_settings().lock().unwrap().detune,
                            0.0..=100.0,
                        )
                        .step_by(0.1)
                        .show_value(true),
                    );
                });

                // 波形選択
                ui.horizontal(|ui| {
                    ui.label("Waveform:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.unison_manager.get_settings().lock().unwrap().waveform))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.unison_manager.get_settings().lock().unwrap().waveform,
                                Waveform::Sine,
                                "Sine",
                            );
                            ui.selectable_value(
                                &mut self.unison_manager.get_settings().lock().unwrap().waveform,
                                Waveform::Triangle,
                                "Triangle",
                            );
                            ui.selectable_value(
                                &mut self.unison_manager.get_settings().lock().unwrap().waveform,
                                Waveform::Square,
                                "Square",
                            );
                            ui.selectable_value(
                                &mut self.unison_manager.get_settings().lock().unwrap().waveform,
                                Waveform::Sawtooth,
                                "Sawtooth",
                            );
                        });
                });
            });

            // 再生・停止ボタンを横並びで表示
            ui.horizontal(|ui| {
                // ▶ 再生ボタンが押された & 現在停止中なら
                if ui.button("▶ Play").clicked() && !self.playing {
                    self.playing = true; // 再生状態に変更
                    let freq = self.freq; // 現在の周波数をコピー

                    // サイン波を再生してストリームを保持
                    let stream = audio::play_sine_wave(
                        freq,
                        Arc::clone(&self.current_freq),
                        Arc::clone(&self.unison_manager),
                        Arc::new(Mutex::new(Envelope::default())),
                        &self.oscillator_settings,
                    );
                    self.stream_handle = Some(stream);
                }

                // ⏹ 停止ボタンが押された & 再生中なら
                if ui.button("⏹ Stop").clicked() && self.playing {
                    self.playing = false;      // 停止状態に変更
                    self.stream_handle = None; // ストリームを破棄（再生停止）
                }
            });
        });
    }
}
