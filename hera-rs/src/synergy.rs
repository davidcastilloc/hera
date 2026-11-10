use crate::structure::HotCuePoint;
pub use crate::synergy_hnsw::{
    prefilter_candidates_hnsw, synthesize_or_normalize_64d_embedding, HNSW_DEFAULT_STAGE1_POOL,
    HNSW_PREFILTER_THRESHOLD,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateTrackCandidate {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub file_path: String,
    pub duration_seconds: f32,
    pub bpm: f32,
    pub camelot_key: String,
    #[serde(default = "default_energy")]
    pub energy_score: f32,
    #[serde(default = "default_punch")]
    pub punch_index: f32,
    #[serde(default = "default_groove")]
    pub groove_type: String,
    #[serde(default = "default_subgenre")]
    pub top_subgenre: String,
    #[serde(default)]
    pub has_intro_vocals: bool,
    #[serde(default)]
    pub has_outro_vocals: bool,
    #[serde(default)]
    pub embedding_64d: Vec<f32>,
    #[serde(default)]
    pub hot_cues: Vec<HotCuePoint>,
}

fn default_energy() -> f32 {
    0.70
}
fn default_punch() -> f32 {
    0.65
}
fn default_groove() -> String {
    "four_on_floor".to_string()
}
fn default_subgenre() -> String {
    "Electronic / Club".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionTelemetry {
    pub from_track_id: String,
    pub to_track_id: String,
    pub camelot_score: f32,
    pub camelot_relation: String,
    pub bpm_score: f32,
    pub bpm_delta_pct: f32,
    pub is_polyrhythmic_bridge: bool,
    pub transient_score: f32,
    pub style_score: f32,
    pub vocal_clash_warning: bool,
    pub total_synergy: f32,
    pub mix_recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpineAnchor {
    pub track_id: String,
    /// Named role (`"opener"`, `"peak"`, `"bridge"`, `"closer"`) used when `slot`/`position` are omitted.
    #[serde(default)]
    pub role: Option<String>,
    /// Exact 0-based slot index in the crate (`0..limit`).
    #[serde(default)]
    pub slot: Option<usize>,
    /// Normalized position along the set (`0.0..=1.0`).
    #[serde(default)]
    pub position: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetSpineConfig {
    #[serde(default)]
    pub anchors: Vec<SpineAnchor>,
    /// Custom energy control points (`0.0..=1.0`) linearly interpolated across all slots.
    #[serde(default)]
    pub custom_energy_curve: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedCrateResult {
    pub brief: String,
    pub energy_arc: String,
    pub total_tracks: usize,
    pub total_duration_seconds: f32,
    pub mean_synergy_score: f32,
    #[serde(default)]
    pub energy_curve_adherence: f32,
    pub harmonic_match_pct: f32,
    pub polyrhythmic_bridges: usize,
    pub vocal_clash_warnings: usize,
    #[serde(default)]
    pub target_energy_curve: Vec<f32>,
    #[serde(default)]
    pub pinned_slots: Vec<usize>,
    pub sequence: Vec<CrateTrackCandidate>,
    pub transitions: Vec<TransitionTelemetry>,
    pub m3u8_export: String,
    pub rekordbox_xml_export: String,
    pub set_guide_markdown: String,
}

/// Parse a Camelot string (`"8A"`, `"11B"`) into `(number: 1..=12, letter: 'A' | 'B')`.
pub fn parse_camelot(code: &str) -> Option<(i32, char)> {
    let clean = code.trim().to_ascii_uppercase();
    if clean.len() < 2 {
        return None;
    }
    let letter = clean.chars().last()?;
    if letter != 'A' && letter != 'B' {
        return None;
    }
    let num: i32 = clean[..clean.len() - 1].parse().ok()?;
    if (1..=12).contains(&num) {
        Some((num, letter))
    } else {
        None
    }
}

/// Compute Layer 1 Camelot Wheel circular harmonic compatibility score and relationship label.
pub fn compute_camelot_synergy(key_a: &str, key_b: &str) -> (f32, String) {
    let (Some((n1, l1)), Some((n2, l2))) = (parse_camelot(key_a), parse_camelot(key_b)) else {
        return (0.60, "Unknown Key (EQ Blend)".to_string());
    };

    let raw_diff = (n2 - n1).rem_euclid(12);
    let circ_dist = raw_diff.min(12 - raw_diff);

    if n1 == n2 && l1 == l2 {
        (1.00, format!("Unison Lock ({key_a} -> {key_b})"))
    } else if n1 == n2 && l1 != l2 {
        (0.95, format!("Relative Major/Minor ({key_a} -> {key_b})"))
    } else if circ_dist == 1 && l1 == l2 {
        if raw_diff == 1 {
            (0.92, format!("Fifth Climb +1 ({key_a} -> {key_b})"))
        } else {
            (0.90, format!("Fifth Smooth -1 ({key_a} -> {key_b})"))
        }
    } else if raw_diff == 2 && l1 == l2 {
        (0.82, format!("Energy Boost +2 ({key_a} -> {key_b})"))
    } else if raw_diff == 7 && l1 == l2 {
        (0.80, format!("Semitone Energy Surge +7 ({key_a} -> {key_b})"))
    } else if circ_dist == 1 && l1 != l2 {
        (0.76, format!("Diagonal Modulation ({key_a} -> {key_b})"))
    } else if circ_dist == 2 {
        (0.62, format!("Two-Step Bridge ({key_a} -> {key_b})"))
    } else {
        (0.28, format!("Harmonic Clash ({key_a} -> {key_b}, Drum-Only Swap)"))
    }
}

/// Compute Layer 2 BPM curve compatibility including 3:4 / 4:3 polyrhythmic bridges and 2x/0.5x octave bridges.
pub fn compute_bpm_synergy(bpm_a: f32, bpm_b: f32) -> (f32, f32, bool, String) {
    if bpm_a <= 0.0 || bpm_b <= 0.0 {
        return (0.70, 0.0, false, "Standard Tempo Match".to_string());
    }

    let direct_pct = ((bpm_b - bpm_a).abs() / bpm_a) * 100.0;

    // Check 3:4 / 4:3 Dotted-Eighth Polyrhythmic Bridge (e.g. 96 BPM <-> 128 BPM)
    let ratio_4_3 = bpm_a * (4.0 / 3.0);
    let ratio_3_4 = bpm_a * (3.0 / 4.0);
    let poly_4_3_pct = ((bpm_b - ratio_4_3).abs() / ratio_4_3) * 100.0;
    let poly_3_4_pct = ((bpm_b - ratio_3_4).abs() / ratio_3_4) * 100.0;

    if direct_pct > 8.0 && (poly_4_3_pct <= 2.5 || poly_3_4_pct <= 2.5) {
        let best_poly = poly_4_3_pct.min(poly_3_4_pct);
        let score = (0.91 - best_poly * 0.04).clamp(0.75, 0.92);
        let dir = if poly_4_3_pct <= poly_3_4_pct {
            "3:4 Polyrhythmic Upshift"
        } else {
            "4:3 Polyrhythmic Downshift"
        };
        return (
            score,
            round2(best_poly),
            true,
            format!("{dir} ({bpm_a:.1} -> {bpm_b:.1} BPM)"),
        );
    }

    // Check 2:1 / 1:2 Half/Double Time Bridge
    let dbl_pct = ((bpm_b - bpm_a * 2.0).abs() / (bpm_a * 2.0)) * 100.0;
    let half_pct = ((bpm_b - bpm_a * 0.5).abs() / (bpm_a * 0.5)) * 100.0;
    if direct_pct > 15.0 && (dbl_pct <= 3.0 || half_pct <= 3.0) {
        let best_oct = dbl_pct.min(half_pct);
        return (
            0.86,
            round2(best_oct),
            true,
            format!("Half/Double Time Bridge ({bpm_a:.1} -> {bpm_b:.1} BPM)"),
        );
    }

    let score = if direct_pct <= 1.5 {
        1.00 - direct_pct * 0.02
    } else if direct_pct <= 3.5 {
        0.96 - (direct_pct - 1.5) * 0.06
    } else if direct_pct <= 6.0 {
        0.84 - (direct_pct - 3.5) * 0.09
    } else {
        (0.60 - (direct_pct - 6.0) * 0.05).max(0.15)
    };

    (
        round3(score),
        round2(direct_pct),
        false,
        format!("Pitch Shift {:+.1}%", ((bpm_b - bpm_a) / bpm_a) * 100.0),
    )
}

/// Evaluate complete 4-Layer Synergy between Track A and Track B.
pub fn evaluate_transition(a: &CrateTrackCandidate, b: &CrateTrackCandidate) -> TransitionTelemetry {
    let (camelot_score, camelot_relation) = compute_camelot_synergy(&a.camelot_key, &b.camelot_key);
    let (bpm_score, bpm_delta_pct, is_poly, tempo_note) = compute_bpm_synergy(a.bpm, b.bpm);

    // Layer 3: Transient Punch & Groove + Vocal Clash Penalty
    let punch_diff = (a.punch_index - b.punch_index).abs();
    let groove_bonus = if a.groove_type == b.groove_type {
        0.10
    } else if is_poly {
        0.06
    } else {
        -0.04
    };
    let vocal_clash = a.has_outro_vocals && b.has_intro_vocals;
    let vocal_penalty = if vocal_clash { 0.22 } else { 0.0 };

    let transient_score = (1.0 - punch_diff * 0.65 + groove_bonus - vocal_penalty).clamp(0.15, 1.0);

    // Layer 4: 64D Style Embedding Cosine Similarity
    let style_score = cosine_similarity(&a.embedding_64d, &b.embedding_64d);

    let total_synergy = (camelot_score * 0.34
        + bpm_score * 0.28
        + transient_score * 0.20
        + style_score * 0.18)
        .clamp(0.0, 1.0);

    let mix_recommendation = if is_poly {
        format!(
            "{tempo_note} | Cut sub-bass on Track A at Cue 4 and lock Track B Cue 1 on triplet grid"
        )
    } else if vocal_clash {
        format!(
            "VOCAL CLASH ALERT: Both Outro A & Intro B have active vocals — use 4-bar stem/EQ mid cut at Cue 4 ({camelot_relation})"
        )
    } else if camelot_score >= 0.90 && bpm_delta_pct <= 2.5 {
        format!(
            "Long 16-bar harmonic blend from Cue 4 -> Cue 1 ({camelot_relation}, {tempo_note})"
        )
    } else {
        format!("8-bar rhythmic EQ swap at Cue 4 -> Cue 1 ({camelot_relation}, {tempo_note})")
    };

    TransitionTelemetry {
        from_track_id: a.track_id.clone(),
        to_track_id: b.track_id.clone(),
        camelot_score: round3(camelot_score),
        camelot_relation,
        bpm_score: round3(bpm_score),
        bpm_delta_pct,
        is_polyrhythmic_bridge: is_poly,
        transient_score: round3(transient_score),
        style_score: round3(style_score),
        vocal_clash_warning: vocal_clash,
        total_synergy: round3(total_synergy),
        mix_recommendation,
    }
}

/// Build target normalized energy curve across `0..limit` from custom control points or named presets.
pub fn compute_target_energy_curve(
    limit: usize,
    energy_arc: &str,
    custom_curve: &[f32],
) -> Vec<f32> {
    if limit == 0 {
        return vec![];
    }
    if !custom_curve.is_empty() {
        if custom_curve.len() == 1 || limit == 1 {
            return vec![round3(custom_curve[0].clamp(0.10, 1.0)); limit];
        }
        let max_idx = (custom_curve.len() - 1) as f32;
        return (0..limit)
            .map(|slot| {
                let pos = ((slot as f32) / ((limit - 1) as f32)) * max_idx;
                let lo = (pos.floor() as usize).min(custom_curve.len() - 1);
                let hi = (lo + 1).min(custom_curve.len() - 1);
                let frac = pos - (lo as f32);
                round3((custom_curve[lo] * (1.0 - frac) + custom_curve[hi] * frac).clamp(0.10, 1.0))
            })
            .collect();
    }

    (0..limit)
        .map(|slot| {
            let x = if limit > 1 {
                (slot as f32) / ((limit - 1) as f32)
            } else {
                0.5
            };
            let val = match energy_arc {
                "rollercoaster" => {
                    (0.55 + 0.35 * (x * std::f32::consts::PI * 2.0).sin()).clamp(0.30, 0.98)
                }
                "peak_hour" => (0.80 + 0.16 * (x * std::f32::consts::PI).sin()).clamp(0.65, 0.99),
                "club_journey" | "warmup_peak_cooldown" => {
                    if x <= 0.70 {
                        0.45 + (x / 0.70) * 0.51
                    } else {
                        0.96 - ((x - 0.70) / 0.30) * 0.34
                    }
                }
                "double_peak" | "festival_wave" => {
                    (0.66 + 0.28 * (x * std::f32::consts::PI * 3.0).sin()).clamp(0.40, 0.97)
                }
                _ => (0.48 + 0.46 * x).clamp(0.35, 0.96), // steady_climb
            };
            round3(val)
        })
        .collect()
}

/// Global 2-Opt Traveling Salesperson (TSP) Crate Optimizer with Energy Arcs
/// and Stage-1 `usearch` HNSW candidate retrieval for large libraries (`> 64` tracks).
pub fn optimize_crate_2opt(
    brief: &str,
    energy_arc: &str,
    candidates: Vec<CrateTrackCandidate>,
    max_tracks: Option<usize>,
) -> OptimizedCrateResult {
    let stage1_candidates = if candidates.len() > HNSW_PREFILTER_THRESHOLD {
        let target_pool = max_tracks
            .unwrap_or(HNSW_DEFAULT_STAGE1_POOL)
            .saturating_mul(4)
            .max(HNSW_DEFAULT_STAGE1_POOL);
        prefilter_candidates_hnsw(&candidates, target_pool)
    } else {
        candidates
    };
    optimize_crate_with_spine(
        brief,
        energy_arc,
        stage1_candidates,
        max_tracks,
        &SetSpineConfig::default(),
    )
}

/// Constrained 2-Opt TSP ("Set Spine Builder") with locked anchor tracks and custom energy curves.
pub fn optimize_crate_with_spine(
    brief: &str,
    energy_arc: &str,
    candidates: Vec<CrateTrackCandidate>,
    max_tracks: Option<usize>,
    spine: &SetSpineConfig,
) -> OptimizedCrateResult {
    let limit = max_tracks.unwrap_or(candidates.len()).min(candidates.len());
    if limit == 0 {
        return OptimizedCrateResult {
            brief: brief.to_string(),
            energy_arc: energy_arc.to_string(),
            total_tracks: 0,
            total_duration_seconds: 0.0,
            mean_synergy_score: 0.0,
            energy_curve_adherence: 0.0,
            harmonic_match_pct: 0.0,
            polyrhythmic_bridges: 0,
            vocal_clash_warnings: 0,
            target_energy_curve: vec![],
            pinned_slots: vec![],
            sequence: vec![],
            transitions: vec![],
            m3u8_export: String::new(),
            rekordbox_xml_export: String::new(),
            set_guide_markdown: String::new(),
        };
    }

    let n_all = candidates.len();
    let mut matrix = vec![0.0_f32; n_all * n_all];
    for i in 0..n_all {
        let row_offset = i * n_all;
        for j in 0..n_all {
            if i != j {
                matrix[row_offset + j] =
                    evaluate_transition(&candidates[i], &candidates[j]).total_synergy;
            }
        }
    }

    let target_energy = compute_target_energy_curve(limit, energy_arc, &spine.custom_energy_curve);
    let max_energy_slot = (0..limit)
        .max_by(|&a, &b| {
            target_energy[a]
                .partial_cmp(&target_energy[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(limit / 2);

    // Resolve Spine Anchors into locked slot indices
    let mut pinned: Vec<Option<usize>> = vec![None; limit];
    let mut used = vec![false; n_all];
    for anchor in &spine.anchors {
        let Some(cand_idx) = candidates
            .iter()
            .position(|c| c.track_id == anchor.track_id)
        else {
            continue;
        };
        if used[cand_idx] {
            continue;
        }
        let desired_slot = if let Some(s) = anchor.slot {
            s.min(limit - 1)
        } else if let Some(p) = anchor.position {
            ((p.clamp(0.0, 1.0) * (limit.saturating_sub(1) as f32)).round() as usize)
                .min(limit - 1)
        } else {
            match anchor
                .role
                .as_deref()
                .unwrap_or("opener")
                .to_ascii_lowercase()
                .as_str()
            {
                "closer" | "outro" => limit - 1,
                "peak" | "climax" => max_energy_slot,
                "bridge" | "pivot" | "mid" | "valley" => limit / 2,
                _ => 0,
            }
        };
        let resolved_slot = if pinned[desired_slot].is_none() {
            Some(desired_slot)
        } else {
            (0..limit)
                .filter(|&s| pinned[s].is_none())
                .min_by_key(|&s| s.abs_diff(desired_slot))
        };
        if let Some(slot) = resolved_slot {
            pinned[slot] = Some(cand_idx);
            used[cand_idx] = true;
        }
    }

    let (syn_w, ene_w) = if !spine.custom_energy_curve.is_empty() {
        (0.62_f32, 0.38_f32)
    } else {
        (0.76_f32, 0.24_f32)
    };
    let slot_energy_fit = |cand: usize, slot: usize| -> f32 {
        let diff = (candidates[cand].energy_score - target_energy[slot]).abs();
        (1.0 - diff - (diff * diff * 0.65)).clamp(0.0, 1.0)
    };

    // 1. Constrained Greedy Seed with Lookahead to Next Anchor
    let mut order = vec![0usize; limit];
    for slot in 0..limit {
        if let Some(c_idx) = pinned[slot] {
            order[slot] = c_idx;
            continue;
        }
        let next_anchor = ((slot + 1)..limit).find_map(|s| pinned[s].map(|c| (s, c)));
        let mut best_next = 0usize;
        let mut best_val = -1e9_f32;
        for cand in 0..n_all {
            if used[cand] {
                continue;
            }
            let e_fit = slot_energy_fit(cand, slot);
            let mut val = if slot == 0 {
                e_fit - candidates[cand].bpm * 0.0005
            } else {
                let prev = order[slot - 1];
                matrix[prev * n_all + cand] * syn_w + e_fit * ene_w
            };
            if let Some((anchor_slot, anchor_cand)) = next_anchor {
                let dist = (anchor_slot - slot) as f32;
                let pull_weight = if dist <= 1.0 { syn_w } else { 0.40 / dist };
                val += matrix[cand * n_all + anchor_cand] * pull_weight;
            }
            if val > best_val {
                best_val = val;
                best_next = cand;
            }
        }
        used[best_next] = true;
        order[slot] = best_next;
    }

    // 2. Constrained 2-Opt + Cross-Anchor Unpinned Swap + Pool Bridge Replacement (Zero-Allocation In-Place)
    let route_score = |seq: &[usize]| -> f32 {
        if seq.is_empty() {
            return 0.0;
        }
        let e_sum: f32 = (0..seq.len()).map(|k| slot_energy_fit(seq[k], k)).sum();
        let mean_e = e_sum / (seq.len() as f32);
        if seq.len() == 1 {
            return mean_e;
        }
        let syn_sum: f32 = (0..(seq.len() - 1))
            .map(|k| matrix[seq[k] * n_all + seq[k + 1]])
            .sum();
        let mean_s = syn_sum / ((seq.len() - 1) as f32);
        mean_s * syn_w + mean_e * ene_w
    };

    if order.len() >= 2 {
        let mut current_best = route_score(&order);
        for _pass in 0..35 {
            let mut improved = false;

            // Move A: Constrained 2-Opt segment reversal in-place (zero Vec clones)
            if order.len() >= 4 {
                for i in 0..(order.len() - 2) {
                    for k in (i + 2)..order.len() {
                        let valid_rev =
                            (i..=k).all(|s| pinned[s].is_none() || s == i + k - s);
                        if !valid_rev {
                            continue;
                        }
                        order[i..=k].reverse();
                        let sc = route_score(&order);
                        if sc > current_best + 1e-4 {
                            current_best = sc;
                            improved = true;
                        } else {
                            order[i..=k].reverse();
                        }
                    }
                }
            }

            // Move B: Pairwise swap of unpinned slots in-place (zero Vec clones)
            for i in 0..order.len() {
                if pinned[i].is_some() {
                    continue;
                }
                for k in (i + 1)..order.len() {
                    if pinned[k].is_some() {
                        continue;
                    }
                    order.swap(i, k);
                    let sc = route_score(&order);
                    if sc > current_best + 1e-4 {
                        current_best = sc;
                        improved = true;
                    } else {
                        order.swap(i, k);
                    }
                }
            }

            // Move C: Candidate pool bridge replacement in-place when max_tracks < candidates.len()
            if limit < n_all {
                for slot in 0..order.len() {
                    if pinned[slot].is_some() {
                        continue;
                    }
                    for cand in 0..n_all {
                        if order.contains(&cand) {
                            continue;
                        }
                        let old_cand = order[slot];
                        order[slot] = cand;
                        let sc = route_score(&order);
                        if sc > current_best + 1e-4 {
                            current_best = sc;
                            improved = true;
                        } else {
                            order[slot] = old_cand;
                        }
                    }
                }
            }

            if !improved {
                break;
            }
        }
    }

    let pinned_slots: Vec<usize> = (0..limit).filter(|&s| pinned[s].is_some()).collect();
    let sequence: Vec<CrateTrackCandidate> =
        order.iter().map(|&idx| candidates[idx].clone()).collect();

    let mut transitions = Vec::with_capacity(sequence.len().saturating_sub(1));
    let mut sum_syn = 0.0_f32;
    let mut harmonic_hits = 0usize;
    let mut poly_count = 0usize;
    let mut vocal_warnings = 0usize;

    for w in sequence.windows(2) {
        let t = evaluate_transition(&w[0], &w[1]);
        sum_syn += t.total_synergy;
        if t.camelot_score >= 0.75 {
            harmonic_hits += 1;
        }
        if t.is_polyrhythmic_bridge {
            poly_count += 1;
        }
        if t.vocal_clash_warning {
            vocal_warnings += 1;
        }
        transitions.push(t);
    }

    let n_trans = transitions.len().max(1) as f32;
    let mean_synergy = if transitions.is_empty() {
        1.0
    } else {
        sum_syn / n_trans
    };
    let energy_adherence = (0..sequence.len())
        .map(|i| 1.0 - (sequence[i].energy_score - target_energy[i]).abs())
        .sum::<f32>()
        / (sequence.len().max(1) as f32);
    let harmonic_pct = if transitions.is_empty() {
        100.0
    } else {
        (harmonic_hits as f32 / n_trans) * 100.0
    };
    let total_dur: f32 = sequence.iter().map(|t| t.duration_seconds).sum();

    let m3u8_export = build_m3u8(&sequence);
    let rekordbox_xml_export = build_rekordbox_xml(brief, &sequence);
    let set_guide_markdown = build_set_guide(
        brief,
        energy_arc,
        mean_synergy,
        energy_adherence,
        harmonic_pct,
        &target_energy,
        &pinned_slots,
        &sequence,
        &transitions,
    );

    OptimizedCrateResult {
        brief: brief.to_string(),
        energy_arc: energy_arc.to_string(),
        total_tracks: sequence.len(),
        total_duration_seconds: round2(total_dur),
        mean_synergy_score: round3(mean_synergy),
        energy_curve_adherence: round3(energy_adherence),
        harmonic_match_pct: round2(harmonic_pct),
        polyrhythmic_bridges: poly_count,
        vocal_clash_warnings: vocal_warnings,
        target_energy_curve: target_energy,
        pinned_slots,
        sequence,
        transitions,
        m3u8_export,
        rekordbox_xml_export,
        set_guide_markdown,
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.75;
    }
    let n = a.len().min(b.len());
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for i in 0..n {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = (na.sqrt() * nb.sqrt()).max(1e-9);
    ((dot / denom) + 1.0) * 0.5
}

fn build_m3u8(seq: &[CrateTrackCandidate]) -> String {
    let mut out = String::from("#EXTM3U\n#EXTENC:UTF-8\n");
    for t in seq {
        out.push_str(&format!(
            "#EXTINF:{},{} - {} [{} | {:.1} BPM]\n{}\n",
            t.duration_seconds.round() as i32,
            t.artist,
            t.title,
            t.camelot_key,
            t.bpm,
            t.file_path
        ));
    }
    out
}

fn build_rekordbox_xml(brief: &str, seq: &[CrateTrackCandidate]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<DJ_PLAYLISTS Version=\"1.0.0\">\n  <PRODUCT Name=\"Hera Rust DSP Engine\" Version=\"0.2.0\"/>\n  <COLLECTION Entries=\"",
    );
    xml.push_str(&seq.len().to_string());
    xml.push_str("\">\n");
    for (idx, t) in seq.iter().enumerate() {
        xml.push_str(&format!(
            "    <TRACK TrackID=\"{}\" Name=\"{}\" Artist=\"{}\" TotalTime=\"{}\" AverageBpm=\"{:.2}\" Tonality=\"{}\" Location=\"file://localhost/{}\">\n",
            idx + 1,
            escape_xml(&t.title),
            escape_xml(&t.artist),
            t.duration_seconds.round() as i32,
            t.bpm,
            escape_xml(&t.camelot_key),
            escape_xml(&t.file_path.replace('\\', "/"))
        ));
        for cue in &t.hot_cues {
            xml.push_str(&format!(
                "      <POSITION_MARK Name=\"{}\" Type=\"0\" Start=\"{:.3}\" Num=\"{}\"/>\n",
                escape_xml(&cue.name),
                cue.seconds,
                cue.num.saturating_sub(1)
            ));
        }
        xml.push_str("    </TRACK>\n");
    }
    xml.push_str("  </COLLECTION>\n  <PLAYLISTS>\n    <NODE Type=\"0\" Name=\"ROOT\">\n");
    xml.push_str(&format!(
        "      <NODE Name=\"{}\" Type=\"1\" KeyType=\"0\" Entries=\"{}\">\n",
        escape_xml(brief),
        seq.len()
    ));
    for idx in 0..seq.len() {
        xml.push_str(&format!("        <TRACK Key=\"{}\"/>\n", idx + 1));
    }
    xml.push_str("      </NODE>\n    </NODE>\n  </PLAYLISTS>\n</DJ_PLAYLISTS>\n");
    xml
}

fn build_set_guide(
    brief: &str,
    energy_arc: &str,
    mean_synergy: f32,
    energy_adherence: f32,
    harmonic_pct: f32,
    target_energy: &[f32],
    pinned_slots: &[usize],
    seq: &[CrateTrackCandidate],
    transitions: &[TransitionTelemetry],
) -> String {
    let mut md = format!(
        "# HERA RUST 2-OPT TSP SET GUIDE — {}\n- **Energy Arc**: `{}`\n- **Global Synergy**: `{:.1}%`\n- **Spine Adherence**: `{:.1}%`\n- **Harmonic Lock**: `{:.1}%`\n\n",
        brief,
        energy_arc,
        mean_synergy * 100.0,
        energy_adherence * 100.0,
        harmonic_pct
    );
    for (i, t) in seq.iter().enumerate() {
        let pin_tag = if pinned_slots.contains(&i) {
            " 📌 `[ANCHOR]`"
        } else {
            ""
        };
        let tgt_e = target_energy.get(i).copied().unwrap_or(t.energy_score);
        md.push_str(&format!(
            "{:02}. **{} - {}**{} (`{:.1} BPM` | `{}` | Energy `{:.0}%` vs Target `{:.0}%`)\n",
            i + 1,
            t.artist,
            t.title,
            pin_tag,
            t.bpm,
            t.camelot_key,
            t.energy_score * 100.0,
            tgt_e * 100.0
        ));
        if !t.hot_cues.is_empty() {
            let cues_str: Vec<String> = t
                .hot_cues
                .iter()
                .map(|c| format!("Cue {}: {} @ {:.2}s", c.num, c.name, c.seconds))
                .collect();
            md.push_str(&format!("    - *Hot Cues*: {}\n", cues_str.join(" | ")));
        }
        if i < transitions.len() {
            let tr = &transitions[i];
            md.push_str(&format!(
                "    - *Transition -> #{:02}* (`{:.0}%` synergy): {}\n",
                i + 2,
                tr.total_synergy * 100.0,
                tr.mix_recommendation
            ));
        }
    }
    md
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn round2(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

fn round3(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_track(
        id: &str,
        title: &str,
        bpm: f32,
        key: &str,
        energy: f32,
        outro_vox: bool,
        intro_vox: bool,
    ) -> CrateTrackCandidate {
        CrateTrackCandidate {
            track_id: id.to_string(),
            title: title.to_string(),
            artist: "SimDJ".to_string(),
            file_path: format!("/library/{id}.flac"),
            duration_seconds: 210.0,
            bpm,
            camelot_key: key.to_string(),
            energy_score: energy,
            punch_index: (energy * 0.9).clamp(0.3, 0.95),
            groove_type: "four_on_floor".to_string(),
            top_subgenre: "Club".to_string(),
            has_intro_vocals: intro_vox,
            has_outro_vocals: outro_vox,
            embedding_64d: vec![0.125; 64],
            hot_cues: vec![],
        }
    }

    #[test]
    fn test_set_spine_builder_multi_scenario_simulation() {
        let pool = vec![
            mock_track("t_opener", "Deep Sunset Opener", 122.0, "8A", 0.45, false, false),
            mock_track("t_warmup_2", "Organic Terrace", 123.0, "8A", 0.55, false, false),
            mock_track("t_bridge_9a", "Rolling Groove 9A", 125.0, "9A", 0.68, false, false),
            mock_track("t_build_9a", "Acid Riser 9A", 127.0, "9A", 0.80, false, false),
            mock_track("t_build_10a", "Hypnotic Tunnel 10A", 128.0, "10A", 0.88, false, false),
            mock_track("t_peak_anthem", "Mainstage Peak Anthem", 130.0, "10A", 0.96, false, false),
            mock_track("t_after_peak", "Melodic Release 10B", 126.0, "10B", 0.76, false, false),
            mock_track("t_sunrise_closer", "Sunrise Emotional Closer", 124.0, "9B", 0.62, false, false),
            // Distractors / wrong-key / vocal clash traps / polyrhythmic bridge tracks
            mock_track("t_clash_3b", "Dissonant Industrial 3B", 138.0, "3B", 0.92, true, true),
            mock_track("t_96_bass_closer", "Halftime 96 BPM Bass Outro", 96.0, "10A", 0.60, false, false),
            mock_track("t_valley_ambient", "Deep Breakdown Interlude", 124.0, "9A", 0.36, false, false),
        ];

        // SCENARIO 1: Club Journey (8 tracks selected from 11) with Opener + Peak + Closer Anchors
        let spine_s1 = SetSpineConfig {
            anchors: vec![
                SpineAnchor {
                    track_id: "t_opener".into(),
                    role: Some("opener".into()),
                    slot: None,
                    position: None,
                },
                SpineAnchor {
                    track_id: "t_peak_anthem".into(),
                    role: Some("peak".into()),
                    slot: None,
                    position: None,
                },
                SpineAnchor {
                    track_id: "t_sunrise_closer".into(),
                    role: Some("closer".into()),
                    slot: None,
                    position: None,
                },
            ],
            custom_energy_curve: vec![],
        };
        let res_s1 = optimize_crate_with_spine(
            "Scenario 1: Club Journey Spine",
            "club_journey",
            pool.clone(),
            Some(8),
            &spine_s1,
        );
        println!("\n=== SCENARIO 1: CLUB JOURNEY (OPENER + PEAK + CLOSER) ===\n{}", res_s1.set_guide_markdown);
        assert_eq!(res_s1.sequence.len(), 8);
        assert_eq!(res_s1.sequence[0].track_id, "t_opener");
        assert_eq!(res_s1.sequence[5].track_id, "t_peak_anthem"); // 70% of 0..=7 is slot 5
        assert_eq!(res_s1.sequence[7].track_id, "t_sunrise_closer");
        assert_eq!(res_s1.pinned_slots, vec![0, 5, 7]);
        assert!(res_s1.mean_synergy_score >= 0.85, "Synergy was {}", res_s1.mean_synergy_score);
        assert!(!res_s1.sequence.iter().any(|t| t.track_id == "t_clash_3b"));

        // SCENARIO 2: Custom Drawn "V-Valley Breakdown" Curve ([0.88, 0.36, 0.96])
        let spine_s2 = SetSpineConfig {
            anchors: vec![],
            custom_energy_curve: vec![0.88, 0.36, 0.96],
        };
        let res_s2 = optimize_crate_with_spine(
            "Scenario 2: V-Valley Custom Curve",
            "custom",
            pool.clone(),
            Some(5),
            &spine_s2,
        );
        println!("\n=== SCENARIO 2: CUSTOM V-VALLEY CURVE ===\n{}", res_s2.set_guide_markdown);
        assert_eq!(res_s2.target_energy_curve, vec![0.88, 0.62, 0.36, 0.66, 0.96]);
        // Slot 2 (the 0.36 valley) or adjacent mid slot should dip significantly below start and end
        let mid_min_energy = res_s2.sequence[1..=3]
            .iter()
            .map(|t| t.energy_score)
            .fold(1.0_f32, f32::min);
        assert!(mid_min_energy <= 0.56, "Expected valley dip in middle slots, got {mid_min_energy}");
        assert!(res_s2.sequence[4].energy_score >= 0.88);

        // SCENARIO 3: 3:4 Polyrhythmic Bridge into Locked 96 BPM Closer Anchor
        let spine_s3 = SetSpineConfig {
            anchors: vec![
                SpineAnchor {
                    track_id: "t_opener".into(),
                    role: None,
                    slot: Some(0),
                    position: None,
                },
                SpineAnchor {
                    track_id: "t_96_bass_closer".into(),
                    role: Some("closer".into()),
                    slot: None,
                    position: None,
                },
            ],
            custom_energy_curve: vec![0.45, 0.70, 0.85, 0.88, 0.60],
        };
        let res_s3 = optimize_crate_with_spine(
            "Scenario 3: 128->96 BPM Polyrhythmic Anchor Bridge",
            "custom",
            pool.clone(),
            Some(5),
            &spine_s3,
        );
        println!("\n=== SCENARIO 3: POLYRHYTHMIC BRIDGE TO 96 BPM CLOSER ===\n{}", res_s3.set_guide_markdown);
        assert_eq!(res_s3.sequence[4].track_id, "t_96_bass_closer");
        // The track placed at slot 3 right before 96 BPM (10A) should be t_build_10a (128 BPM, 10A -> exact 4:3 bridge!)
        assert_eq!(res_s3.sequence[3].track_id, "t_build_10a");
        assert!(res_s3.transitions[3].is_polyrhythmic_bridge);

        // SCENARIO 4: Anchor Slot Collision Resolution (two tracks pinned to slot 0)
        let spine_s4 = SetSpineConfig {
            anchors: vec![
                SpineAnchor {
                    track_id: "t_opener".into(),
                    role: None,
                    slot: Some(0),
                    position: None,
                },
                SpineAnchor {
                    track_id: "t_warmup_2".into(),
                    role: None,
                    slot: Some(0),
                    position: None,
                },
            ],
            custom_energy_curve: vec![],
        };
        let res_s4 = optimize_crate_with_spine(
            "Scenario 4: Anchor Collision Resolution",
            "steady_climb",
            pool,
            Some(4),
            &spine_s4,
        );
        assert_eq!(res_s4.sequence[0].track_id, "t_opener");
        assert_eq!(res_s4.sequence[1].track_id, "t_warmup_2");
        assert_eq!(res_s4.pinned_slots, vec![0, 1]);
    }
}
