use crate::forensics::audit_pcm_channels;
#[cfg(not(target_arch = "wasm32"))]
use crate::forensics::decode_audio_bytes;
use crate::p2p::compute_hera_seal;
use crate::structure::analyze_structure_and_style;
use serde_json::{json, Value};

pub(super) fn analyze_batch_args(args: &Value) -> Result<Value, String> {
    let paths = args
        .get("file_paths")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "`file_paths` array is required".to_string())?;
    let bpm_hint = args
        .get("bpm_hint")
        .and_then(|v| v.as_f64())
        .map(|f| f as f32);
    let mut results = Vec::with_capacity(paths.len());
    for p_val in paths {
        if let Some(p_str) = p_val.as_str() {
            let sub_arg = json!({
                "file_path": p_str,
                "bpm_hint": bpm_hint
            });
            match analyze_single_track_arg(&sub_arg) {
                Ok(r) => results.push(r),
                Err(e) => results.push(json!({ "file_path": p_str, "error": e })),
            }
        }
    }
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
            let bytes =
                std::fs::read(fp).map_err(|e| format!("Failed to read audio file `{fp}`: {e}"))?;
            let ext = std::path::Path::new(fp)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("wav");
            let decoded = decode_audio_bytes(&bytes, Some(ext), 90.0)?;
            let forensics = audit_pcm_channels(
                &decoded.left,
                decoded.right.as_deref(),
                decoded.sample_rate,
                ext,
            );
            let structure = analyze_structure_and_style(
                &decoded.mono,
                decoded.sample_rate,
                Some(decoded.duration_seconds),
                bpm_hint,
            );
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
