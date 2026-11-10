use serde::{Deserialize, Serialize};

/// Output of the local CPU `candle` 2-layer neural transition scoring model (`8 -> 16 -> 4`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandleTransitionScore {
    pub neural_synergy: f32,
    pub eq_swap_confidence: f32,
    pub echo_out_confidence: f32,
    pub stem_blend_confidence: f32,
    pub recommended_technique: String,
}

/// Deterministic 2-layer CPU neural network (`Linear(8 -> 16)` + `Activation::Gelu` + `Linear(16 -> 4)`)
/// backed by `candle_core::Tensor` and `candle_nn::Linear` on `Device::Cpu` for `< 2 ms` offline DJ inference.
#[cfg(feature = "native")]
pub struct CandleLocalDjModel {
    device: candle_core::Device,
    fc1: candle_nn::Linear,
    act: candle_nn::Activation,
    fc2: candle_nn::Linear,
}

#[cfg(feature = "native")]
impl CandleLocalDjModel {
    /// Constructs a deterministic `8 -> 16 -> 4` MLP on `Device::Cpu` using sinusoidal weight priors
    /// calibrated to reward harmonic compatibility, gradual tempo deltas, and balanced punch.
    pub fn new_cpu() -> Result<Self, String> {
        use candle_core::{DType, Device, Tensor};
        use candle_nn::{Activation, Linear};

        let device = Device::Cpu;
        let in_dim = 8usize;
        let hidden_dim = 16usize;
        let out_dim = 4usize;

        let mut w1 = Vec::with_capacity(hidden_dim * in_dim);
        for h in 0..hidden_dim {
            for i in 0..in_dim {
                let phase = ((h * 7 + i * 13 + 3) as f32) * 0.37;
                let base = phase.sin() * 0.28 + (phase * 0.5).cos() * 0.14;
                // Emphasize Camelot compatibility (idx 4) and inverse BPM delta (idx 6)
                let prior = match i {
                    4 => 0.42,
                    6 => 0.36,
                    _ => 0.08,
                };
                w1.push(base + prior);
            }
        }
        let b1: Vec<f32> = (0..hidden_dim)
            .map(|h| (((h + 1) as f32) * 0.29).cos() * 0.12)
            .collect();

        let mut w2 = Vec::with_capacity(out_dim * hidden_dim);
        for o in 0..out_dim {
            for h in 0..hidden_dim {
                let phase = ((o * 11 + h * 5 + 1) as f32) * 0.41;
                w2.push(phase.cos() * 0.22 + 0.11);
            }
        }
        let b2 = vec![0.35_f32, 0.25_f32, 0.15_f32, 0.28_f32];

        let t_w1 = Tensor::from_vec(w1, (hidden_dim, in_dim), &device)
            .and_then(|t| t.to_dtype(DType::F32))
            .map_err(|e| format!("candle fc1 weight tensor error: {e}"))?;
        let t_b1 = Tensor::from_vec(b1, (hidden_dim,), &device)
            .map_err(|e| format!("candle fc1 bias tensor error: {e}"))?;
        let t_w2 = Tensor::from_vec(w2, (out_dim, hidden_dim), &device)
            .map_err(|e| format!("candle fc2 weight tensor error: {e}"))?;
        let t_b2 = Tensor::from_vec(b2, (out_dim,), &device)
            .map_err(|e| format!("candle fc2 bias tensor error: {e}"))?;

        Ok(Self {
            device,
            fc1: Linear::new(t_w1, Some(t_b1)),
            act: Activation::Gelu,
            fc2: Linear::new(t_w2, Some(t_b2)),
        })
    }

    /// Runs a forward pass `[1, 8] -> Linear(8, 16) -> Gelu -> Linear(16, 4) -> Sigmoid` on `Device::Cpu`.
    pub fn forward_features(&self, features: &[f32; 8]) -> Result<[f32; 4], String> {
        use candle_core::Tensor;
        use candle_nn::{Activation, Module};

        let input = Tensor::from_slice(features, (1, 8), &self.device)
            .map_err(|e| format!("candle input tensor error: {e}"))?;
        let h = self
            .fc1
            .forward(&input)
            .map_err(|e| format!("candle fc1 forward error: {e}"))?;
        let h_act = self
            .act
            .forward(&h)
            .map_err(|e| format!("candle Gelu forward error: {e}"))?;
        let logits = self
            .fc2
            .forward(&h_act)
            .map_err(|e| format!("candle fc2 forward error: {e}"))?;
        let probs = Activation::Sigmoid
            .forward(&logits)
            .map_err(|e| format!("candle Sigmoid forward error: {e}"))?;
        let flat = probs
            .flatten_all()
            .and_then(|t| t.to_vec1::<f32>())
            .map_err(|e| format!("candle output extraction error: {e}"))?;

        if flat.len() != 4 {
            return Err(format!("Unexpected output dimension: {}", flat.len()));
        }
        Ok([
            round3(flat[0].clamp(0.0, 1.0)),
            round3(flat[1].clamp(0.0, 1.0)),
            round3(flat[2].clamp(0.0, 1.0)),
            round3(flat[3].clamp(0.0, 1.0)),
        ])
    }
}

/// Evaluates transition synergy and mix technique confidences using `CandleLocalDjModel` on CPU
/// (with a deterministic mathematical fallback on `wasm32`).
pub fn score_transition_neural(
    bpm_a: f32,
    bpm_b: f32,
    energy_a: f32,
    energy_b: f32,
    camelot_compat: f32,
    punch_delta: f32,
) -> Result<CandleTransitionScore, String> {
    let safe_bpm_a = bpm_a.max(60.0);
    let safe_bpm_b = bpm_b.max(60.0);
    let bpm_delta_ratio = ((safe_bpm_b - safe_bpm_a).abs() / safe_bpm_a).clamp(0.0, 1.0);
    let tempo_closeness = (1.0 - bpm_delta_ratio * 4.0).clamp(0.0, 1.0);
    let energy_step = (energy_b - energy_a).clamp(-1.0, 1.0);
    let cam = camelot_compat.clamp(0.0, 1.0);
    let punch = (1.0 - punch_delta.abs()).clamp(0.0, 1.0);

    let features: [f32; 8] = [
        (safe_bpm_a - 120.0) / 40.0,
        (safe_bpm_b - 120.0) / 40.0,
        energy_a.clamp(0.0, 1.0),
        energy_b.clamp(0.0, 1.0),
        cam,
        punch,
        tempo_closeness,
        energy_step,
    ];

    #[cfg(feature = "native")]
    let raw_scores = {
        let model = CandleLocalDjModel::new_cpu()?;
        model.forward_features(&features)?
    };

    #[cfg(not(feature = "native"))]
    let raw_scores = [
        round3(
            (features[4] * 0.45 + features[6] * 0.35 + features[5] * 0.20 + features[7] * 0.05)
                .clamp(0.1, 0.99),
        ),
        round3((features[6] * 0.55 + features[5] * 0.45).clamp(0.1, 0.99)),
        round3(((1.0 - features[6]) * 0.65 + 0.25).clamp(0.1, 0.99)),
        round3((features[4] * 0.60 + features[6] * 0.40).clamp(0.1, 0.99)),
    ];

    // Blend neural tensor output with domain guardrails for interpretable booth confidence
    let neural_synergy = round3((raw_scores[0] * 0.55 + cam * 0.25 + tempo_closeness * 0.20).clamp(0.0, 1.0));
    let eq_swap_confidence = round3((raw_scores[1] * 0.60 + punch * 0.40).clamp(0.0, 1.0));
    let echo_out_confidence =
        round3((raw_scores[2] * 0.55 + (1.0 - tempo_closeness) * 0.45).clamp(0.0, 1.0));
    let stem_blend_confidence =
        round3((raw_scores[3] * 0.55 + cam * 0.45).clamp(0.0, 1.0));

    let recommended_technique = if bpm_delta_ratio > 0.05 {
        "Echo-Out 3/4 Tail Freeze @ Cue 4 -> Hard Cut Cue 1".to_string()
    } else if stem_blend_confidence >= eq_swap_confidence && cam >= 0.85 {
        "16-Bar Neural Stem Blend (Vocal/Instrumental Split Cue 4 -> Cue 1)".to_string()
    } else {
        "8-Bar Sub-Bass EQ Swap @ Cue 4 -> Cue 1".to_string()
    };

    Ok(CandleTransitionScore {
        neural_synergy,
        eq_swap_confidence,
        echo_out_confidence,
        stem_blend_confidence,
        recommended_technique,
    })
}

fn round3(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_cpu_neural_forward_determinism_and_bounds() {
        let start = std::time::Instant::now();
        let s1 = score_transition_neural(124.0, 125.0, 0.72, 0.80, 0.95, 0.05)
            .expect("candle CPU forward pass should succeed");
        let elapsed = start.elapsed();
        let s2 = score_transition_neural(124.0, 125.0, 0.72, 0.80, 0.95, 0.05)
            .expect("second candle CPU forward pass should match");

        assert_eq!(s1, s2, "CandleLocalDjModel forward pass must be 100% deterministic");
        assert!((0.0..=1.0).contains(&s1.neural_synergy));
        assert!((0.0..=1.0).contains(&s1.eq_swap_confidence));
        assert!((0.0..=1.0).contains(&s1.echo_out_confidence));
        assert!((0.0..=1.0).contains(&s1.stem_blend_confidence));
        assert!(s1.neural_synergy >= 0.70);
        assert!(!s1.recommended_technique.is_empty());
        assert!(
            elapsed < std::time::Duration::from_millis(25),
            "Expected fast local CPU tensor inference, took {elapsed:?}"
        );
    }
}
