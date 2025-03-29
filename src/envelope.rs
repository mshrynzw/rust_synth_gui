use serde::{Deserialize, Serialize};

/// ADSRエンベロープのパラメータを表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeParams {
    pub attack: f32,   // アタック時間（秒）
    pub decay: f32,    // ディケイ時間（秒）
    pub sustain: f32,  // サステインレベル（0.0-1.0）
    pub release: f32,  // リリース時間（秒）
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        Self {
            attack: 0.001,  // 1ミリ秒
            decay: 0.05,    // 50ミリ秒
            sustain: 0.7,   // 70%
            release: 0.1,   // 100ミリ秒
        }
    }
}

/// エンベロープの状態を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvelopeState {
    Idle,     // 待機状態
    Attack,   // アタックフェーズ
    Decay,    // ディケイフェーズ
    Sustain,  // サステインフェーズ
    Release,  // リリースフェーズ
}

/// エンベロープを表す構造体
#[derive(Debug)]
pub struct Envelope {
    params: EnvelopeParams,
    state: EnvelopeState,
    value: f32,
    time: f32,
    sample_rate: f32,
    release_start_value: f32,  // リリース開始時の値を保存
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            params: EnvelopeParams::default(),
            state: EnvelopeState::Idle,
            value: 0.0,
            time: 0.0,
            sample_rate: 44100.0,
            release_start_value: 0.0,
        }
    }
}

impl Envelope {
    pub fn new(params: EnvelopeParams, sample_rate: f32) -> Self {
        Self {
            params,
            state: EnvelopeState::Idle,
            value: 0.0,
            time: 0.0,
            sample_rate,
            release_start_value: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.state = EnvelopeState::Attack;
        self.time = 0.0;
        self.value = 0.0;  // アタック開始時に値を0にリセット
    }

    pub fn end(&mut self) {
        println!("Release started with value: {}", self.value);  // デバッグ出力
        self.state = EnvelopeState::Release;
        self.time = 0.0;
        self.release_start_value = self.value;  // リリース開始時の値を保存
    }

    pub fn update(&mut self, delta_time: f32) {
        // 時間を更新（秒単位）
        self.time += delta_time;

        match self.state {
            EnvelopeState::Idle => {
                self.value = 0.0;
            }
            EnvelopeState::Attack => {
                if self.time >= self.params.attack {
                    self.state = EnvelopeState::Decay;
                    self.time = 0.0;
                    self.value = 1.0;  // アタック完了時に最大値に
                } else {
                    let t = self.time / self.params.attack;
                    // 線形なアタック
                    self.value = t;
                }
            }
            EnvelopeState::Decay => {
                if self.time >= self.params.decay {
                    self.state = EnvelopeState::Sustain;
                    self.time = 0.0;
                    self.value = self.params.sustain;
                } else {
                    let t = self.time / self.params.decay;
                    // 線形なディケイ
                    self.value = 1.0 - (1.0 - self.params.sustain) * t;
                }
            }
            EnvelopeState::Sustain => {
                self.value = self.params.sustain;
            }
            EnvelopeState::Release => {
                if self.time >= self.params.release {
                    println!("Release completed at time: {}", self.time);  // デバッグ出力
                    self.state = EnvelopeState::Idle;
                    self.time = 0.0;
                    self.value = 0.0;
                } else {
                    let t = self.time / self.params.release;
                    // 線形なリリース
                    self.value = self.release_start_value * (1.0 - t);
                    println!("Release value: {} at time: {} (target: {})", 
                        self.value, self.time, self.params.release);  // デバッグ出力
                }
            }
        }
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }

    pub fn get_params(&self) -> EnvelopeParams {
        self.params.clone()
    }

    pub fn set_params(&mut self, params: EnvelopeParams) {
        self.params = params;
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
}