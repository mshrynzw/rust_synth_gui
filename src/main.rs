// 標準ライブラリから、円周率（PI）を使用
use std::f32::consts::PI;

// スレッド間で共有・同期するために、Arc（参照カウント付きポインタ）と Mutex（排他ロック）を使用
use std::sync::{Arc, Mutex};

// オーディオ出力のために、cpalクレートのトレイトをインポート
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// GUIアプリの構築のために、eframe（eguiベース）をインポート
use eframe::{egui, App};

/// アプリケーションのエントリーポイント（GUIの初期化）
fn main() -> Result<(), eframe::Error> {
    // ウィンドウ設定を定義（タイトルとウィンドウサイズ）
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 200.0])  // ウィンドウの初期サイズ
            .with_title("Rust Synth"),        // ウィンドウタイトル
        ..Default::default()
    };

    // アプリケーションを起動（`SynthApp` を中身として実行）
    eframe::run_native(
        "Rust Synth", // 内部的なアプリ名
        options,      // ウィンドウ設定
        Box::new(|_cc| Box::new(SynthApp::default())), // アプリケーションの初期化クロージャ
    )
}

/// アプリの状態を表す構造体
struct SynthApp {
    freq: f32, // 再生する周波数（Hz）
    playing: bool, // 音を再生中かどうかのフラグ
    stream_handle: Option<cpal::Stream>, // 再生中のストリーム（再生停止に使う）
}

/// アプリのデフォルト初期値を定義（440Hz・再生停止中）
impl Default for SynthApp {
    fn default() -> Self {
        Self {
            freq: 440.0,         // A4（ラ）の周波数
            playing: false,      // 初期状態は再生停止中
            stream_handle: None, // ストリームはまだ存在しない
        }
    }
}

/// eframe::App の実装（毎フレーム呼ばれる update 関数など）
impl App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 中央パネルにGUIを描画する
        egui::CentralPanel::default().show(ctx, |ui| {
            // タイトル見出し
            ui.heading("🎹 Rust Synth");

            // 周波数スライダー（100Hz〜1000Hz）を追加
            ui.add(
                egui::Slider::new(&mut self.freq, 100.0..=1000.0)
                    .text("Frequency (Hz)"),
            );

            // 再生・停止ボタンを横並びで表示
            ui.horizontal(|ui| {
                // ▶ 再生ボタンが押された & 現在停止中なら
                if ui.button("▶ Play").clicked() && !self.playing {
                    self.playing = true; // 再生状態に変更
                    let freq = self.freq; // 現在の周波数をコピー

                    // サイン波を再生してストリームを保持
                    let stream = play_sine_wave(freq);
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
fn play_sine_wave(freq: f32) -> cpal::Stream {
    // デフォルトのオーディオホストを取得（WindowsならWASAPIなど）
    let host = cpal::default_host();

    // 出力デバイス（例：スピーカー）を取得
    let device = host.default_output_device().expect("No output device found");

    // 出力デバイスの設定（例：44100Hz, f32型など）を取得
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate().0 as f32;

    // 音の時間位置を追跡するための変数を作成（スレッド安全）
    let t = Arc::new(Mutex::new(0.0_f32));
    let t_clone = Arc::clone(&t);

    // build_output_stream にクロージャを直接渡すことでライフタイムエラーを回避
    let stream = device
        .build_output_stream(
            &config.into(), // 出力設定（サンプルレートなど）
            move |data: &mut [f32], _info| {
                // t をロックして使う（他スレッドと競合しないように）
                let mut t = t_clone.lock().unwrap();

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
    stream // 呼び出し元にストリームを返す（停止用に必要）
}
