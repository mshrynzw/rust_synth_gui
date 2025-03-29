/// エンベロープのパラメータを表す構造体
#[derive(Debug, Clone, Copy)]
pub struct EnvelopeParams {
    pub attack: f32,   // アタック時間（秒）
    pub decay: f32,    // ディケイ時間（秒）
    pub sustain: f32,  // サステインレベル（0.0-1.0）
    pub release: f32,  // リリース時間（秒）
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
        }
    }
}

/// エンベロープの状態を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeState {
    Idle,      // 待機状態
    Attack,    // アタックフェーズ
    Decay,     // ディケイフェーズ
    Sustain,   // サステインフェーズ
    Release,   // リリースフェーズ
}

/// エンベロープを表す構造体
#[derive(Debug)]
pub struct Envelope {
    params: EnvelopeParams,
    state: EnvelopeState,
    value: f32,
    time: f32,
    sample_rate: f32,
    last_value: f32,  // 前回の値を保持
    is_active: bool,  // エンベロープがアクティブかどうか
    sustain_value: f32,  // サステインフェーズの値を保持
    is_released: bool,  // リリースが開始されたかどうか
    is_triggered: bool,  // トリガーが開始されたかどうか
    is_sustaining: bool,  // サステインフェーズ中かどうか
    is_processing: bool,  // エンベロープが処理中かどうか
    attack_start_time: f32,  // アタック開始時の時間
    note_id: u32,  // ノートID
    release_start_value: f32,  // リリース開始時の値
    release_start_time: f32,  // リリース開始時の時間
    phase_time: f32,  // 現在のフェーズの経過時間
    release_phase_time: f32,  // リリースフェーズの経過時間
}

impl Envelope {
    pub fn new(params: EnvelopeParams, sample_rate: f32) -> Self {
        Self {
            params,
            state: EnvelopeState::Idle,
            value: 0.0,
            time: 0.0,
            sample_rate,
            last_value: 0.0,
            is_active: false,
            sustain_value: 0.0,
            is_released: false,
            is_triggered: false,
            is_sustaining: false,
            is_processing: false,
            attack_start_time: 0.0,
            note_id: 0,
            release_start_value: 0.0,
            release_start_time: 0.0,
            phase_time: 0.0,
            release_phase_time: 0.0,
        }
    }

    pub fn start(&mut self, note_id: u32) {
        // 同じノートIDの場合は再トリガーしない
        if self.note_id == note_id && self.is_processing {
            return;
        }

        // 異なるノートIDの場合は、現在のノートをリリースしてから新しいノートを開始
        if self.is_processing && self.note_id != note_id {
            self.end();
        }

        self.state = EnvelopeState::Attack;
        self.time = 0.0;
        self.phase_time = 0.0;
        self.attack_start_time = 0.0;
        self.last_value = self.value;
        self.is_active = true;
        self.is_released = false;
        self.is_triggered = true;
        self.is_sustaining = false;
        self.is_processing = true;
        self.sustain_value = self.params.sustain;
        self.note_id = note_id;
        self.release_start_value = 0.0;
        self.release_start_time = 0.0;
        self.release_phase_time = 0.0;
    }

    pub fn end(&mut self) {
        if self.is_triggered && !self.is_released && self.is_processing {
            self.state = EnvelopeState::Release;
            self.release_phase_time = 0.0;
            self.last_value = self.value;
            self.release_start_value = self.value;
            self.release_start_time = self.time;
            self.is_released = true;
            self.is_sustaining = false;
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        if !self.is_triggered || !self.is_processing {
            self.value = 0.0;
            self.is_released = false;
            self.is_sustaining = false;
            self.is_processing = false;
            return;
        }

        self.time += delta_time;
        self.phase_time += delta_time;

        match self.state {
            EnvelopeState::Idle => {
                self.value = 0.0;
                self.is_released = false;
                self.is_sustaining = false;
                self.is_processing = false;
            }
            EnvelopeState::Attack => {
                if self.phase_time >= self.params.attack {
                    self.state = EnvelopeState::Decay;
                    self.phase_time = 0.0;
                    self.last_value = self.value;
                } else {
                    // より自然なアタックカーブ
                    let t = self.phase_time / self.params.attack;
                    let target = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
                    self.value = target;
                }
            }
            EnvelopeState::Decay => {
                if self.phase_time >= self.params.decay {
                    self.state = EnvelopeState::Sustain;
                    self.phase_time = 0.0;
                    self.last_value = self.value;
                    self.value = self.sustain_value;
                    self.is_sustaining = true;
                } else {
                    // より自然なディケイカーブ
                    let t = self.phase_time / self.params.decay;
                    let decay_curve = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
                    let target = 1.0 - (1.0 - self.sustain_value) * decay_curve;
                    self.value = target;
                }
            }
            EnvelopeState::Sustain => {
                // サステインフェーズでは値を更新しない
                if self.is_sustaining {
                    self.value = self.sustain_value;
                }
            }
            EnvelopeState::Release => {
                self.release_phase_time += delta_time;
                if self.release_phase_time >= self.params.release {
                    self.state = EnvelopeState::Idle;
                    self.phase_time = 0.0;
                    self.release_phase_time = 0.0;
                    self.last_value = self.value;
                    self.value = 0.0;
                    self.is_active = false;
                    self.is_released = false;
                    self.is_triggered = false;
                    self.is_sustaining = false;
                    self.is_processing = false;
                    self.note_id = 0;
                    self.release_start_value = 0.0;
                    self.release_start_time = 0.0;
                } else {
                    // より自然なリリースカーブ
                    let t = self.release_phase_time / self.params.release;
                    let release_curve = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
                    let target = self.release_start_value * (1.0 - release_curve);
                    self.value = target;
                }
            }
        }
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }

    pub fn set_params(&mut self, params: EnvelopeParams) {
        self.params = params;
        if self.is_sustaining {
            self.sustain_value = params.sustain;
            self.value = params.sustain;
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
}

/// 複数のエンベロープを管理する構造体
#[derive(Debug)]
pub struct EnvelopeManager {
    envelopes: Vec<Envelope>,
    params: EnvelopeParams,
    sample_rate: f32,
    is_triggered: bool,  // トリガーが開始されたかどうか
    is_processing: bool,  // エンベロープが処理中かどうか
}

impl EnvelopeManager {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            envelopes: vec![Envelope::new(EnvelopeParams::default(), sample_rate)],
            params: EnvelopeParams::default(),
            sample_rate,
            is_triggered: false,
            is_processing: false,
        }
    }

    pub fn start_all(&mut self, note_id: u32) {
        if !self.is_triggered && !self.is_processing {
            for envelope in &mut self.envelopes {
                envelope.start(note_id);
            }
            self.is_triggered = true;
            self.is_processing = true;
        }
    }

    pub fn end_all(&mut self) {
        if self.is_triggered && self.is_processing {
            for envelope in &mut self.envelopes {
                envelope.end();
            }
            self.is_triggered = false;
            self.is_processing = false;
        }
    }

    pub fn update_all(&mut self, delta_time: f32) {
        if self.is_processing {
            for envelope in &mut self.envelopes {
                envelope.update(delta_time);
            }
        }
    }

    pub fn get_value(&self, index: usize) -> f32 {
        self.envelopes.get(index).map_or(0.0, |e| e.get_value())
    }

    pub fn set_params(&mut self, params: EnvelopeParams) {
        self.params = params;
        for envelope in &mut self.envelopes {
            envelope.set_params(params);
        }
    }

    pub fn get_params(&self) -> EnvelopeParams {
        self.params
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for envelope in &mut self.envelopes {
            envelope.set_sample_rate(sample_rate);
        }
    }
} 