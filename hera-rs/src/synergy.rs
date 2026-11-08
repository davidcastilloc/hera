use crate::structure::HotCuePoint;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedCrateResult {
    pub brief: String,
    pub energy_arc: String,
    pub total_tracks: usize,
    pub total_duration_seconds: f32,
    pub mean_synergy_score: f32,
    pub harmonic_match_pct: f32,
    pub polyrhythmic_bridges: usize,
    pub vocal_clash_warnings: usize,
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

/// Global 2-Opt Traveling Salesperson (TSP) Crate Optimizer with Energy Arcs.
pub fn optimize_crate_2opt(
    brief: &str,
    energy_arc: &str,
    candidates: Vec<CrateTrackCandidate>,
    max_tracks: Option<usize>,
) -> OptimizedCrateResult {
    let limit = max_tracks.unwrap_or(candidates.len()).min(candidates.len());
    if limit == 0 {
        return OptimizedCrateResult {
            brief: brief.to_string(),
            energy_arc: energy_arc.to_string(),
            total_tracks: 0,
            total_duration_seconds: 0.0,
            mean_synergy_score: 0.0,
            harmonic_match_pct: 0.0,
            polyrhythmic_bridges: 0,
            vocal_clash_warnings: 0,
            sequence: vec![],
            transitions: vec![],
            m3u8_export: String::new(),
            rekordbox_xml_export: String::new(),
            set_guide_markdown: String::new(),
        };
    }

    let n_all = candidates.len();
    // Precompute pairwise transition synergy matrix
    let mut matrix = vec![vec![0.0_f32; n_all]; n_all];
    for i in 0..n_all {
        for j in 0..n_all {
            if i != j {
                matrix[i][j] = evaluate_transition(&candidates[i], &candidates[j]).total_synergy;
            }
        }
    }

    // Target normalized energy curve across slots 0..limit
    let target_energy: Vec<f32> = (0..limit)
        .map(|slot| {
            let x = if limit > 1 {
                (slot as f32) / ((limit - 1) as f32)
            } else {
                0.5
            };
            match energy_arc {
                "rollercoaster" => (0.55 + 0.35 * (x * std::f32::consts::PI * 2.0).sin()).clamp(0.3, 0.98),
                "peak_hour" => (0.80 + 0.16 * (x * std::f32::consts::PI).sin()).clamp(0.65, 0.99),
                _ => (0.48 + 0.46 * x).clamp(0.35, 0.96), // steady_climb
            }
        })
        .collect();

    // 1. Greedy seed path starting from track closest to initial target energy
    let mut used = vec![false; n_all];
    let mut order = Vec::with_capacity(limit);

    let first_idx = (0..n_all)
        .min_by(|&a, &b| {
            let da = (candidates[a].energy_score - target_energy[0]).abs() + candidates[a].bpm * 0.001;
            let db = (candidates[b].energy_score - target_energy[0]).abs() + candidates[b].bpm * 0.001;
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0);

    used[first_idx] = true;
    order.push(first_idx);

    for slot in 1..limit {
        let prev = *order.last().unwrap();
        let mut best_next = 0usize;
        let mut best_val = -1e9_f32;
        for cand in 0..n_all {
            if used[cand] {
                continue;
            }
            let syn = matrix[prev][cand];
            let e_fit = 1.0 - (candidates[cand].energy_score - target_energy[slot]).abs();
            let val = syn * 0.80 + e_fit * 0.20;
            if val > best_val {
                best_val = val;
                best_next = cand;
            }
        }
        used[best_next] = true;
        order.push(best_next);
    }

    // 2. Global 2-Opt Local Search Refinement
    let route_score = |seq: &[usize]| -> f32 {
        if seq.len() <= 1 {
            return 1.0;
        }
        let mut s = 0.0_f32;
        for k in 0..(seq.len() - 1) {
            let syn = matrix[seq[k]][seq[k + 1]];
            let e_fit = 1.0 - (candidates[seq[k]].energy_score - target_energy[k]).abs();
            s += syn * 0.82 + e_fit * 0.18;
        }
        s / ((seq.len() - 1) as f32)
    };

    if order.len() >= 4 {
        let mut current_best = route_score(&order);
        for _pass in 0..30 {
            let mut improved = false;
            for i in 0..(order.len() - 2) {
                for k in (i + 2)..order.len() {
                    let mut trial = order.clone();
                    trial[i..=k].reverse();
                    let sc = route_score(&trial);
                    if sc > current_best + 1e-4 {
                        order = trial;
                        current_best = sc;
                        improved = true;
                    }
                }
            }
            if !improved {
                break;
            }
        }
    }

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
        harmonic_pct,
        &sequence,
        &transitions,
    );

    OptimizedCrateResult {
        brief: brief.to_string(),
        energy_arc: energy_arc.to_string(),
        total_tracks: sequence.len(),
        total_duration_seconds: round2(total_dur),
        mean_synergy_score: round3(mean_synergy),
        harmonic_match_pct: round2(harmonic_pct),
        polyrhythmic_bridges: poly_count,
        vocal_clash_warnings: vocal_warnings,
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
    harmonic_pct: f32,
    seq: &[CrateTrackCandidate],
    transitions: &[TransitionTelemetry],
) -> String {
    let mut md = format!(
        "# HERA RUST 2-OPT TSP SET GUIDE — {}\n- **Energy Arc**: `{}`\n- **Global Synergy**: `{:.1}%`\n- **Harmonic Lock**: `{:.1}%`\n\n",
        brief,
        energy_arc,
        mean_synergy * 100.0,
        harmonic_pct
    );
    for (i, t) in seq.iter().enumerate() {
        md.push_str(&format!(
            "{:02}. **{} - {}** (`{:.1} BPM` | `{}` | Energy `{:.0}%`)\n",
            i + 1,
            t.artist,
            t.title,
            t.bpm,
            t.camelot_key,
            t.energy_score * 100.0
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
