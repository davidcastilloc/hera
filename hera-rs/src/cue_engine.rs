//! 8-Hot-Cue DJ merger (`CueOrigin::DjManual` sacred + anti-empty-zone second-half Pads 5..8).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CueOrigin {
    DjManual,
    HeraAuto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueCandidate {
    pub num: u8,
    pub name: String,
    pub seconds: f64,
    pub role: String,
    pub origin: CueOrigin,
}

impl CueCandidate {
    pub fn auto(num: u8, name: impl Into<String>, seconds: f64, role: impl Into<String>) -> Self {
        Self {
            num,
            name: name.into(),
            seconds,
            role: role.into(),
            origin: CueOrigin::HeraAuto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenreCueTemplate {
    pub genre_family: String,
    pub pad5_ratio: f64,
    pub pad6_ratio: f64,
    pub pad7_ratio: f64,
    pub pad8_ratio: f64,
}

impl GenreCueTemplate {
    pub fn default_for(genre_family: impl Into<String>) -> Self {
        Self {
            genre_family: genre_family.into(),
            pad5_ratio: 0.55,
            pad6_ratio: 0.66,
            pad7_ratio: 0.78,
            pad8_ratio: 0.88,
        }
    }

    pub fn learn_from_manual_cues(
        genre_family: impl Into<String>,
        duration_sec: f64,
        manual_cues: &[CueCandidate],
    ) -> Self {
        let mut tpl = Self::default_for(genre_family);
        if duration_sec <= 1.0 {
            return tpl;
        }
        let avg_pad = |pad: u8, fallback: f64| -> f64 {
            let matching: Vec<f64> = manual_cues
                .iter()
                .filter(|c| c.num == pad && c.seconds >= 0.0)
                .map(|c| (c.seconds / duration_sec).clamp(0.50, 0.96))
                .collect();
            if matching.is_empty() {
                fallback
            } else {
                (matching.iter().sum::<f64>() / matching.len() as f64).clamp(0.50, 0.96)
            }
        };
        tpl.pad5_ratio = avg_pad(5, tpl.pad5_ratio);
        tpl.pad6_ratio = avg_pad(6, tpl.pad6_ratio).clamp(tpl.pad5_ratio + 0.03, 0.96);
        tpl.pad7_ratio = avg_pad(7, tpl.pad7_ratio).clamp(tpl.pad6_ratio + 0.03, 0.96);
        tpl.pad8_ratio = avg_pad(8, tpl.pad8_ratio).clamp(tpl.pad7_ratio + 0.02, 0.96);
        tpl
    }
}

fn window_rms_at(samples: &[f32], sr: usize, center_sec: f64, half_win_sec: f64) -> f32 {
    if samples.is_empty() || sr == 0 {
        return 0.0;
    }
    let center = ((center_sec.max(0.0) * sr as f64) as usize).min(samples.len() - 1);
    let half = ((half_win_sec * sr as f64) as usize).max(1);
    let (s0, s1) = (
        center.saturating_sub(half),
        (center + half).min(samples.len()),
    );
    (samples[s0..s1].iter().map(|&x| x * x).sum::<f32>() / ((s1 - s0).max(1) as f32)).sqrt()
}

fn track_median_rms(samples: &[f32], sr: usize) -> f32 {
    if samples.is_empty() || sr == 0 {
        return 0.0;
    }
    let win = (sr / 2).max(64);
    let mut vals: Vec<f32> = samples
        .chunks(win)
        .map(|c| (c.iter().map(|&x| x * x).sum::<f32>() / (c.len() as f32)).sqrt())
        .filter(|&r| r > 1e-4)
        .collect();
    if vals.is_empty() {
        return 0.0;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    vals[vals.len() / 2]
}

fn nudge_out_of_trough(
    samples: &[f32],
    sr: usize,
    target_sec: f64,
    min_sec: f64,
    max_sec: f64,
    step_sec: f64,
    rms_floor: f32,
) -> f64 {
    let clamped = target_sec.clamp(min_sec, max_sec);
    if rms_floor <= 0.0 || window_rms_at(samples, sr, clamped, 0.25) >= rms_floor {
        return clamped;
    }
    let step = step_sec.max(0.25);
    let mut best_sec = clamped;
    let mut best_rms = window_rms_at(samples, sr, clamped, 0.25);
    for k in 1..=16 {
        for dir in [1.0_f64, -1.0_f64] {
            let cand = (clamped + dir * (k as f64) * step).clamp(min_sec, max_sec);
            let r = window_rms_at(samples, sr, cand, 0.25);
            if r >= rms_floor {
                return cand;
            }
            if r > best_rms {
                best_rms = r;
                best_sec = cand;
            }
        }
    }
    best_sec
}

#[allow(clippy::too_many_arguments)]
pub fn merge_and_place_8_hot_cues(
    samples: &[f32],
    sample_rate: u32,
    bpm: f64,
    first_beat_sec: f64,
    first_audible_sec: f64,
    base_4_cues: &[CueCandidate],
    manual_cues: &[CueCandidate],
    template: Option<&GenreCueTemplate>,
) -> Vec<CueCandidate> {
    let sr = sample_rate.max(8_000) as usize;
    let dur_sec = if samples.is_empty() {
        base_4_cues
            .iter()
            .chain(manual_cues.iter())
            .map(|c| c.seconds)
            .fold(0.0_f64, f64::max)
    } else {
        samples.len() as f64 / sr as f64
    };
    let beat_sec = 60.0 / bpm.clamp(60.0, 200.0);
    let bar_sec = beat_sec * 4.0;
    let anchor = first_beat_sec.max(0.0);
    let med_rms = track_median_rms(samples, sr);
    let minus_38_dbfs = 0.012_589_25_f32; // 10^(-38 / 20)

    // Rule 1: Manual DJ cues in 1..=8 are sacred and preserved verbatim
    let mut pads: BTreeMap<u8, CueCandidate> = BTreeMap::new();
    for m in manual_cues {
        if (1..=8).contains(&m.num) && m.seconds.is_finite() && m.seconds >= 0.0 {
            pads.entry(m.num).or_insert_with(|| CueCandidate {
                num: m.num,
                name: m.name.clone(),
                seconds: m.seconds,
                role: m.role.clone(),
                origin: CueOrigin::DjManual,
            });
        }
    }

    // Rule 2: Cue 1 >= first_audible_sec snapped forward by beat_sec from first_beat_sec
    pads.entry(1).or_insert_with(|| {
        let mut c1 = anchor;
        if c1 < first_audible_sec.max(0.0) {
            c1 += ((first_audible_sec.max(0.0) - c1) / beat_sec).ceil() * beat_sec;
        }
        if dur_sec > 0.0 {
            c1 = c1.min(dur_sec);
        }
        CueCandidate::auto(1, "1:Intro", c1, "Intro")
    });

    // Rule 3: Fill missing Pads 2..=4 from base_4_cues (monotonic + nudged out of <-38 dBFS troughs)
    let mut prev_sec = pads.get(&1).map(|c| c.seconds).unwrap_or(0.0);
    let floor_2_4 = if med_rms > minus_38_dbfs * 1.5 {
        minus_38_dbfs
    } else {
        0.0
    };
    for pad in 2..=4_u8 {
        if let Some(existing) = pads.get(&pad) {
            prev_sec = prev_sec.max(existing.seconds);
            continue;
        }
        let (raw_sec, name, role) = base_4_cues
            .iter()
            .find(|c| c.num == pad)
            .map(|c| (c.seconds, c.name.clone(), c.role.clone()))
            .unwrap_or_else(|| {
                let (ratio, label, r) = match pad {
                    2 => (0.22, "2:Breakdown", "Breakdown"),
                    3 => (0.36, "3:Drop", "Drop"),
                    _ => (0.46, "4:Phrase", "Outro"),
                };
                (dur_sec * ratio, label.into(), r.into())
            });
        let min_s = (prev_sec + beat_sec).min(dur_sec.max(0.0));
        let max_s = (dur_sec - beat_sec).max(min_s);
        let placed = nudge_out_of_trough(
            samples,
            sr,
            raw_sec.max(min_s),
            min_s,
            max_s,
            beat_sec,
            floor_2_4,
        );
        prev_sec = placed;
        pads.insert(pad, CueCandidate::auto(pad, name, placed, role));
    }

    // Rule 4: Pads 5..=8 in second half (>= 50% duration) when duration_sec >= 30.0
    if dur_sec >= 30.0 {
        let default_tpl = GenreCueTemplate::default_for("General");
        let tpl = template.unwrap_or(&default_tpl);
        let ratios = [
            (5_u8, tpl.pad5_ratio, "5:Phrase B", "Phrase"),
            (6_u8, tpl.pad6_ratio, "6:Bridge", "Breakdown"),
            (7_u8, tpl.pad7_ratio, "7:Climax", "Drop"),
            (8_u8, tpl.pad8_ratio, "8:Mix Out", "Outro"),
        ];
        let half_sec = 0.50 * dur_sec;
        let max_sec = (dur_sec - 0.5).max(half_sec);
        let trough_floor_5_8 = med_rms * 0.15;
        let mut last_auto = half_sec;

        for (pad, ratio, label, role) in ratios {
            if let Some(existing) = pads.get(&pad) {
                if existing.seconds >= half_sec {
                    last_auto = last_auto.max(existing.seconds);
                }
                continue;
            }
            let min_sec = half_sec.max((last_auto + beat_sec).min(max_sec));
            let raw_target = (ratio.clamp(0.50, 0.96) * dur_sec).max(min_sec);
            let bars = ((raw_target - anchor) / bar_sec).round();
            let mut snapped = anchor + bars * bar_sec;
            while snapped < min_sec {
                snapped += bar_sec;
            }
            snapped = snapped.clamp(min_sec, max_sec);
            let placed = nudge_out_of_trough(
                samples,
                sr,
                snapped,
                min_sec,
                max_sec,
                bar_sec * 0.5,
                trough_floor_5_8,
            );
            last_auto = placed;
            pads.insert(pad, CueCandidate::auto(pad, label, placed, role));
        }
    }

    pads.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn cand(num: u8, name: &str, seconds: f64, role: &str, origin: CueOrigin) -> CueCandidate {
        CueCandidate {
            num,
            name: name.into(),
            seconds,
            role: role.into(),
            origin,
        }
    }

    #[test]
    fn test_merge_and_place_8_hot_cues_preserves_manual_and_places_second_half() {
        let (sr, dur_sec) = (8_000_u32, 60.0_f64);
        let mut samples = vec![0.0_f32; (dur_sec * sr as f64) as usize];
        for (i, s) in samples.iter_mut().enumerate() {
            let t = i as f64 / sr as f64;
            if t >= 2.2 && !(32.4..=34.6).contains(&t) {
                *s = 0.45 * (TAU * 120.0 * (t as f32)).sin();
            }
        }
        let base_4 = vec![
            cand(1, "1:Intro", 0.0, "Intro", CueOrigin::HeraAuto),
            cand(2, "2:Breakdown", 12.0, "Breakdown", CueOrigin::HeraAuto),
            cand(3, "3:Drop", 20.0, "Drop", CueOrigin::HeraAuto),
            cand(4, "4:Outro", 28.0, "Outro", CueOrigin::HeraAuto),
        ];
        let manual = vec![
            cand(5, "Mambo", 35.12, "Custom", CueOrigin::DjManual),
            cand(8, "Cierre", 54.40, "Outro", CueOrigin::DjManual),
        ];
        let cues =
            merge_and_place_8_hot_cues(&samples, sr, 120.0, 0.25, 2.2, &base_4, &manual, None);
        assert_eq!(cues.len(), 8);
        assert!(cues[0].seconds >= 2.0);
        let c5 = cues.iter().find(|c| c.num == 5).unwrap();
        let c8 = cues.iter().find(|c| c.num == 8).unwrap();
        assert_eq!(
            (c5.origin, (c5.seconds * 100.0).round() as i64),
            (CueOrigin::DjManual, 3512)
        );
        assert_eq!(
            (c8.origin, (c8.seconds * 100.0).round() as i64),
            (CueOrigin::DjManual, 5440)
        );
        for pad in 5..=8 {
            let c = cues.iter().find(|x| x.num == pad).unwrap();
            assert!(c.seconds >= 0.50 * dur_sec && !(32.7..=34.3).contains(&c.seconds));
        }
        let learned = GenreCueTemplate::learn_from_manual_cues("Salsa", dur_sec, &manual);
        assert!((learned.pad5_ratio - 35.12 / 60.0).abs() < 1e-3);
        assert!((learned.pad8_ratio - 54.40 / 60.0).abs() < 1e-3);
    }
}
