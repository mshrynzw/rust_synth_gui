use std::sync::{Arc, Mutex};
use eframe::{egui, App};
use cpal::Stream;
use midir::MidiInputConnection;

use crate::audio::play_sine_wave;
use crate::midi::setup_midi_callback;
use crate::unison::UnisonManager;
use crate::oscillator::Waveform;
use crate::envelope::{Envelope, EnvelopeParams};

/// アプリの状態を表す構造体
pub struct SynthApp {
    freq: f32, // 再生する周波数（Hz）
    stream_handle: Option<Stream>, // 再生中のストリーム（再生停止に使う）
    midi_connection: Option<MidiInputConnection<()>>, // MIDI接続ハンドル
    last_note: Option<u8>, // 最後に押されたノート番号
    midi_freq: Arc<Mutex<f32>>, // MIDIから設定された周波数（スレッド間共有）
    current_freq: Arc<Mutex<f32>>, // 現在再生中の周波数（スレッド間共有）
    midi_ports: Vec<String>, // 利用可能なMIDIポートのリスト
    selected_port: usize, // 選択されたMIDIポートのインデックス
    unison_manager: Arc<UnisonManager>, // Unison設定の管理
    envelope: Arc<Mutex<Envelope>>, // ADSRエンベロープ
}

/// アプリのデフォルト初期値を定義（440Hz・再生停止中）
impl Default for SynthApp {
    fn default() -> Self {
        Self {
            freq: 0.0,          // 初期周波数は0（音なし）
            stream_handle: None, // ストリームはまだ存在しない
            midi_connection: None, // MIDI接続はまだ存在しない
            last_note: None,     // 最後に押されたノートはまだない
            midi_freq: Arc::new(Mutex::new(0.0)), // MIDI周波数の初期値（音なし）
            current_freq: Arc::new(Mutex::new(0.0)), // 現在の周波数の初期値（音なし）
            midi_ports: Vec::new(), // MIDIポートのリストは空
            selected_port: 0,    // デフォルトは最初のポート
            unison_manager: Arc::new(UnisonManager::new()), // Unison設定の初期化
            envelope: Arc::new(Mutex::new(Envelope::new(EnvelopeParams::default(), 44100.0))), // エンベロープの初期化
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
                        let envelope = Arc::clone(&self.envelope);
                        if let Ok(conn) = setup_midi_callback(midi_in, port, current_freq, envelope) {
                            println!("MIDI connection established successfully");
                            self.midi_connection = Some(conn);
                            
                            // オーディオストリームを開始（初期周波数は0で音なし）
                            let stream = play_sine_wave(
                                0.0,
                                Arc::clone(&self.current_freq),
                                Arc::clone(&self.unison_manager),
                                Arc::clone(&self.envelope),
                            );
                            self.stream_handle = Some(stream);
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
                // 音声ストリームを停止
                self.stream_handle = None;
                // MIDI接続を切断
                self.midi_connection = None;
                self.last_note = None;
                // 周波数を0に設定
                if let Ok(mut freq_lock) = self.current_freq.lock() {
                    *freq_lock = 0.0;
                }
                if let Ok(mut freq_lock) = self.midi_freq.lock() {
                    *freq_lock = 0.0;
                }
                self.freq = 0.0;
            }

            // 波形選択UI
            ui.separator();
            ui.heading("Oscillator Settings");
            
            // 波形選択コンボボックス
            let mut current_waveform = if let Ok(settings) = self.unison_manager.get_settings().lock() {
                settings.waveform
            } else {
                Waveform::Sine
            };
            
            egui::ComboBox::from_label("Waveform")
                .selected_text(format!("{:?}", current_waveform))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut current_waveform, Waveform::Sine, "Sine");
                    ui.selectable_value(&mut current_waveform, Waveform::Triangle, "Triangle");
                    ui.selectable_value(&mut current_waveform, Waveform::Square, "Square");
                    ui.selectable_value(&mut current_waveform, Waveform::Sawtooth, "Sawtooth");
                });
            
            self.unison_manager.set_waveform(current_waveform);

            // Unison設定UI
            ui.separator();
            ui.heading("Unison Settings");
            
            // Unisonボイス数のスライダー（1-8）
            let mut voices = if let Ok(settings) = self.unison_manager.get_settings().lock() {
                settings.voices
            } else {
                1
            };
            ui.add(egui::Slider::new(&mut voices, 1..=8).text("Unison Voices"));
            self.unison_manager.set_voices(voices);
            
            // デチューン量のスライダー（0から100セント）
            let mut detune = if let Ok(settings) = self.unison_manager.get_settings().lock() {
                settings.detune
            } else {
                0.0
            };
            ui.add(egui::Slider::new(&mut detune, 0.0..=100.0).text("Detune (cents)"));
            self.unison_manager.set_detune(detune);

            // ADSRエンベロープ設定UI
            ui.separator();
            ui.heading("ADSR Envelope");
            
            if let Ok(mut envelope) = self.envelope.lock() {
                let mut params = envelope.get_params();
                
                ui.add(egui::Slider::new(&mut params.attack, 0.001..=0.5)
                    .text("Attack (ms)")
                    .clamp_to_range(true));
                ui.add(egui::Slider::new(&mut params.decay, 0.001..=0.5)
                    .text("Decay (ms)")
                    .clamp_to_range(true));
                ui.add(egui::Slider::new(&mut params.sustain, 0.0..=1.0)
                    .text("Sustain")
                    .clamp_to_range(true));
                ui.add(egui::Slider::new(&mut params.release, 0.001..=0.5)
                    .text("Release (ms)")
                    .clamp_to_range(true));

                envelope.set_params(params);
            }

            // 周波数スライダー（100Hz〜1000Hz）を追加
            ui.separator();
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
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // アプリケーション終了時のクリーンアップ
        self.stream_handle = None;
        self.midi_connection = None;
        self.last_note = None;
        if let Ok(mut freq_lock) = self.current_freq.lock() {
            *freq_lock = 0.0;
        }
        if let Ok(mut freq_lock) = self.midi_freq.lock() {
            *freq_lock = 0.0;
        }
        self.freq = 0.0;
    }
} 