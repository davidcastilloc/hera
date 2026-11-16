use crate::forensics::audit_pcm_channels;
use crate::p2p::compute_hera_seal;
use crate::structure::analyze_structure_and_style;
use serde_json::{json, Value};

pub(super) fn analyze_batch_args(args: &Value) -> Result<Value, String> {
    let paths: Vec<String> = args
        .get("file_paths")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "`file_paths` array is required".to_string())?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let bpm_hint = args
        .get("bpm_hint")
        .and_then(|v| v.as_f64())
        .map(|f| f as f32);

    #[cfg(not(target_arch = "wasm32"))]
    let results: Vec<Value> = {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        };
        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(2)
            .min(paths.len().max(1));
        let next_idx = AtomicUsize::new(0);
        let out = Mutex::new(vec![Value::Null; paths.len()]);
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| loop {
                    let i = next_idx.fetch_add(1, Ordering::Relaxed);
                    if i >= paths.len() {
                        break;
                    }
                    let p_str = &paths[i];
                    let sub_arg = json!({ "file_path": p_str, "bpm_hint": bpm_hint });
                    let val = match analyze_single_track_arg(&sub_arg) {
                        Ok(r) => r,
                        Err(e) => json!({ "file_path": p_str, "error": e }),
                    };
                    if let Ok(mut guard) = out.lock() {
                        guard[i] = val;
                    }
                });
            }
        });
        out.into_inner().unwrap_or_default()
    };

    #[cfg(target_arch = "wasm32")]
    let results: Vec<Value> = paths
        .iter()
        .map(|p_str| {
            let sub_arg = json!({ "file_path": p_str, "bpm_hint": bpm_hint });
            analyze_single_track_arg(&sub_arg)
                .unwrap_or_else(|e| json!({ "file_path": p_str, "error": e }))
        })
        .collect();

    Ok(json!({
        "processed": results.len(),
        "results": results
    }))
}

pub(super) fn analyze_single_track_arg(args: &Value) -> Result<Value, String> {
    let bpm_hint = args
        .get("bpm_hint")
        .and_then(|v| v.as_f64())
        .map(|f| f as f32);

    if let Some(fp) = args.get("file_path").and_then(|v| v.as_str()) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = std::path::Path::new(fp);
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("wav");
            let dec =
                crate::bpm_engine::v2_core::audio_ingest::load_audio_mono_native_sr(path, None)
                    .map_err(|e| format!("Failed to decode `{fp}`: {e:?}"))?;
            let forensics = audit_pcm_channels(&dec.samples, None, dec.sample_rate, ext);
            let mut structure = analyze_structure_and_style(
                &dec.samples,
                dec.sample_rate,
                Some(dec.duration_sec as f32),
                bpm_hint,
            );
            let v2_key = crate::bpm_engine::v2_core::key_engine::detect_song_key_auto(
                &dec.samples,
                dec.sample_rate,
            );
            structure.camelot_key = v2_key.camelot;
            let seal = compute_hera_seal(
                fp,
                structure.bpm,
                &structure.camelot_key,
                forensics.quality_score,
                &forensics.forensic_verdict,
            );
            return Ok(json!({
                "file_path": fp,
                "forensics": forensics,
                "structure": structure,
                "hera_seal": seal
            }));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = fp;
            return Err("`file_path` is only supported on native VPS/local mode; pass `pcm_left` or POST binary to `/api/v1/audio/analyze-file` on Cloudflare Workers.".to_string());
        }
    }

    if let Some(left_arr) = args.get("pcm_left").and_then(|v| v.as_array()) {
        let left: Vec<f32> = left_arr
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect();
        if left.is_empty() {
            return Err("`pcm_left` cannot be empty".to_string());
        }
        let right: Option<Vec<f32>> = args.get("pcm_right").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_f64().map(|f| f as f32))
                .collect()
        });

        let sr = args
            .get("sample_rate")
            .and_then(|v| v.as_u64())
            .unwrap_or(44100) as u32;
        let ext = args
            .get("claimed_ext")
            .and_then(|v| v.as_str())
            .unwrap_or("flac");

        let forensics = audit_pcm_channels(&left, right.as_deref(), sr, ext);
        let mono: Vec<f32> = if let Some(ref r) = right {
            let n = left.len().min(r.len());
            left[..n]
                .iter()
                .zip(r[..n].iter())
                .map(|(&l, &rv)| 0.5 * (l + rv))
                .collect()
        } else {
            left
        };
        let dur = (mono.len() as f32) / (sr.max(1) as f32);
        let structure = analyze_structure_and_style(&mono, sr, Some(dur), bpm_hint);
        let seal = compute_hera_seal(
            "mcp-pcm",
            structure.bpm,
            &structure.camelot_key,
            forensics.quality_score,
            &forensics.forensic_verdict,
        );
        return Ok(json!({
            "forensics": forensics,
            "structure": structure,
            "hera_seal": seal
        }));
    }

    Err("Provide either `file_path` or `pcm_left` to `analyze_track`.".to_string())
}
