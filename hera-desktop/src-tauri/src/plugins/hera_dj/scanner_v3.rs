use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use hera_ipc::dj_types::{DiscoveredMusicRoot, SonicTwinMatch};
use hera_node_rs::genre_intelligence::BoothGenreFamily;
use hera_node_rs::scanner::audio_metrics::cosine_similarity_i8;
use hera_node_rs::scanner::omkf_capsule::{
    OmkfCratePruneCandidate, camelot_code_mask_bit, camelot_to_code, code_to_camelot,
    prune_crates_stage1, rank_slots_stage2, read_omkf_capsule,
};
use hera_node_rs::scanner::scheduler::{ScanPriority, ScanScheduler};
use hera_node_rs::synergy::compute_camelot_synergy;
use sqlx::SqliteConnection;
use tauri::State;

use crate::libs::db_scanner_queries::count_scanner_levels;
use crate::plugins::db::DBState;

static GLOBAL_SCANNER_SCHEDULER: OnceLock<Arc<ScanScheduler>> = OnceLock::new();

pub fn global_scheduler() -> &'static Arc<ScanScheduler> {
    GLOBAL_SCANNER_SCHEDULER.get_or_init(|| {
        let workers = std::thread::available_parallelism()
            .map(|n| (n.get() / 2).clamp(2, 6))
            .unwrap_or(4);
        Arc::new(ScanScheduler::new(workers))
    })
}

#[tauri::command]
pub fn scanner_boost_viewport(track_ids: Vec<String>) -> usize {
    let sched = global_scheduler();
    let mut boosted = 0_usize;
    for id in track_ids {
        sched.enqueue_or_promote(id, ScanPriority::P1Viewport);
        boosted += 1;
    }
    boosted
}

#[tauri::command]
pub fn scanner_prewarm_track(track_id: String) -> usize {
    global_scheduler().enqueue_or_promote(track_id, ScanPriority::P0Immediate);
    1
}

#[tauri::command]
pub fn scanner_notify_deck_track(track_id: String, camelot_neighbors: Vec<String>) -> usize {
    let sched = global_scheduler();
    sched.enqueue_or_promote(track_id, ScanPriority::P0Immediate);
    for neighbor in &camelot_neighbors {
        sched.enqueue_or_promote(neighbor.clone(), ScanPriority::P1Viewport);
    }
    1 + camelot_neighbors.len()
}

#[tauri::command]
pub fn scanner_set_deck_playing(playing: bool) -> usize {
    global_scheduler().governor.set_deck_playing(playing);
    global_scheduler().governor.effective_workers()
}

#[allow(dead_code)]
#[tauri::command]
pub async fn scanner_promote_crate_full(
    db_state: State<'_, DBState>,
    crate_id: Option<String>,
    track_ids: Option<Vec<String>>,
) -> Result<usize, String> {
    let mut ids = track_ids.unwrap_or_default();
    if ids.is_empty()
        && let Some(cid) = crate_id.filter(|s| !s.is_empty())
    {
        let mut conn = db_state
            .acquire_read_conn()
            .await
            .map_err(|e| e.to_string())?;
        let q = "SELECT id FROM tracks WHERE crate_id = ?1 AND scan_level >= 0 AND scan_level < 2";
        ids = sqlx::query_scalar(q)
            .bind(&cid)
            .fetch_all(&mut *conn)
            .await
            .unwrap_or_default();
    }
    let count = ids.len();
    global_scheduler().enqueue_batch(ids, ScanPriority::P0Immediate);
    Ok(count)
}

#[allow(dead_code)]
#[tauri::command]
pub async fn scanner_get_telemetry(
    db_state: State<'_, DBState>,
) -> Result<serde_json::Value, String> {
    let mut conn = db_state
        .acquire_read_conn()
        .await
        .map_err(|e| e.to_string())?;
    let (total, fast_done, full_done) = count_scanner_levels(&mut conn).await.unwrap_or((0, 0, 0));
    let sched = global_scheduler();
    let busy = (fast_done < total || full_done < total) && !super::workability::is_scan_cancelled();
    Ok(serde_json::json!({
        "phase": if busy { "analyzing" } else { "idle" },
        "fastDone": fast_done, "fullDone": full_done, "total": total,
        "tracksPerSec": 0.0, "etaSec": 0,
        "governorActive": sched.governor.is_throttled_booth_mode(),
        "externalBoothMode": sched.governor.is_throttled_booth_mode(),
        "pendingCount": sched.pending_count(),
    }))
}

#[allow(dead_code)]
#[tauri::command]
pub fn scanner_cancel_scan() -> bool {
    super::workability::cancel_scan_flag();
    global_scheduler().clear_background_queues();
    true
}

#[allow(dead_code)]
#[tauri::command]
pub async fn scanner_discover_workable_roots() -> Result<Vec<DiscoveredMusicRoot>, String> {
    tauri::async_runtime::spawn_blocking(
        super::workability::discover_workable_roots_from_existing_dbs,
    )
    .await
    .map_err(|e| e.to_string())
}

#[allow(dead_code)]
#[tauri::command]
pub fn scanner_set_external_booth_mode(enabled: bool) -> bool {
    global_scheduler().governor.set_external_booth_mode(enabled);
    enabled
}

fn compat_camelot_mask(cam: &str) -> u32 {
    let c = camelot_to_code(cam);
    if !(1..=24).contains(&c) {
        return 0;
    }
    let (num, off) = ((c - 1) % 12, if c > 12 { 12 } else { 0 });
    let keys = [
        c,
        ((num + 11) % 12) + 1 + off,
        ((num + 1) % 12) + 1 + off,
        (num + 1) + if off == 0 { 12 } else { 0 },
    ];
    keys.into_iter()
        .fold(0, |m, k| m | camelot_code_mask_bit(k))
}

fn decode_i8_fixed<const N: usize>(b64_opt: Option<&str>) -> Option<[i8; N]> {
    let bytes = B64.decode(b64_opt?).ok()?;
    (bytes.len() >= N).then(|| std::array::from_fn(|i| bytes[i] as i8))
}

#[allow(dead_code)]
pub(crate) async fn compute_sonic_twins(
    conn: &mut SqliteConnection,
    track_id: &str,
    limit: usize,
) -> Result<Vec<SonicTwinMatch>, String> {
    let src: Option<(Option<f64>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT bpm, camelot, embedding_64d_b64 FROM tracks WHERE id = ? LIMIT 1")
            .bind(track_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    let Some((bpm_opt, cam_opt, emb_opt)) = src else {
        return Ok(Vec::new());
    };
    let (src_bpm, src_cam) = (bpm_opt.unwrap_or(124.0), cam_opt.unwrap_or_default());
    let q_emb = decode_i8_fixed::<64>(emb_opt.as_deref());
    let mut allowed = HashMap::<String, String>::new();
    if let Some(ref q) = q_emb {
        #[allow(clippy::type_complexity)]
        let fc_rows: Vec<(String, String, Option<f64>, Option<f64>, i64, Option<String>)> =
            sqlx::query_as("SELECT crate_id, folder_path, bpm_min, bpm_max, camelot_mask, centroid_64d_b64 FROM folder_crates")
                .fetch_all(&mut *conn).await.unwrap_or_default();
        let fp_map: HashMap<String, String> =
            fc_rows.iter().map(|r| (r.0.clone(), r.1.clone())).collect();
        let cands: Vec<_> = fc_rows
            .into_iter()
            .map(|(cid, _, b0, b1, mk, c64)| OmkfCratePruneCandidate {
                crate_id: cid,
                bpm_min: b0.map(|x| x as f32),
                bpm_max: b1.map(|x| x as f32),
                camelot_mask: mk.max(0) as u32,
                centroid_top24_i8: decode_i8_fixed::<24>(c64.as_deref()).unwrap_or([0; 24]),
            })
            .collect();
        if !cands.is_empty() {
            let pruned = prune_crates_stage1(
                &cands,
                q,
                Some(((src_bpm * 0.90) as f32, (src_bpm * 1.12) as f32)),
                compat_camelot_mask(&src_cam),
                30,
            );
            for cid in pruned {
                if let Some(fp) = fp_map.get(&cid) {
                    allowed.insert(cid, fp.clone());
                }
            }
        }
    }
    let mut omkf_hits = Vec::<SonicTwinMatch>::new();
    if let Some(ref q) = q_emb {
        let mut seen_paths = HashSet::new();
        for fp in allowed.values() {
            let Ok(cap) = read_omkf_capsule(&Path::new(fp).join(".hera/tracks.omkf")) else {
                continue;
            };
            let slots: Vec<_> = cap.tracks.iter().map(|t| t.slot.clone()).collect();
            for (idx, cos) in rank_slots_stage2(&slots, q, limit) {
                let tr = &cap.tracks[idx];
                let full_p = Path::new(fp)
                    .join(&tr.filename)
                    .to_string_lossy()
                    .to_string();
                let tid: String =
                    sqlx::query_scalar("SELECT id FROM tracks WHERE path = ?1 LIMIT 1")
                        .bind(&full_p)
                        .fetch_optional(&mut *conn)
                        .await
                        .unwrap_or(None)
                        .unwrap_or_else(|| full_p.clone());
                if tid == track_id || !seen_paths.insert(tid.clone()) {
                    continue;
                }
                let fam = BoothGenreFamily::from_code(tr.slot.genre_family_code);
                let g = (fam != BoothGenreFamily::Unknown).then(|| fam.as_str().to_string());
                let pct = (((cos + 1.0) * 50.0).clamp(0.0, 100.0) * 10.0).round() / 10.0;
                omkf_hits.push(SonicTwinMatch {
                    track_id: tid,
                    title: tr.title.clone(),
                    artist: tr.artist.clone(),
                    bpm: f64::from(tr.slot.bpm_x100) / 100.0,
                    camelot: code_to_camelot(tr.slot.camelot_code).unwrap_or_else(|| "8A".into()),
                    similarity_pct: pct,
                    genre_family: g,
                });
            }
        }
        if omkf_hits.len() >= limit.min(5) {
            omkf_hits.sort_by(|a, b| {
                b.similarity_pct
                    .partial_cmp(&a.similarity_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            omkf_hits.truncate(limit.clamp(1, 50));
            return Ok(omkf_hits);
        }
    }
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, String, Option<f64>, Option<String>, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT id, title, album_artist, bpm, camelot, embedding_64d_b64, inferred_genre_family, crate_id FROM tracks WHERE scan_level >= 0 AND id != ?1 AND bpm BETWEEN ?2 AND ?3 LIMIT 800")
            .bind(track_id).bind(src_bpm * 0.90).bind(src_bpm * 1.12).fetch_all(&mut *conn).await.unwrap_or_default();
    let mut matches: Vec<SonicTwinMatch> = rows
        .into_iter()
        .filter_map(|(id, title, artist, b, c, e_b64, gen_fam, cid_opt)| {
            if !allowed.is_empty()
                && !cid_opt
                    .as_deref()
                    .is_some_and(|cid| allowed.contains_key(cid))
            {
                return None;
            }
            let (cand_bpm, cand_cam) = (b.unwrap_or(src_bpm), c.unwrap_or_default());
            let sim = match (q_emb.as_ref(), decode_i8_fixed::<64>(e_b64.as_deref())) {
                (Some(qa), Some(cb)) => {
                    ((cosine_similarity_i8(qa, &cb) + 1.0) * 50.0).clamp(0.0, 100.0)
                }
                _ => {
                    let cs = if !src_cam.is_empty() && !cand_cam.is_empty() {
                        compute_camelot_synergy(&src_cam, &cand_cam).0 * 25.0
                    } else {
                        15.0
                    };
                    (85.0 - (cand_bpm - src_bpm).abs() * 2.0 + cs as f64).clamp(0.0, 99.0) as f32
                }
            };
            Some(SonicTwinMatch {
                track_id: id,
                title,
                artist,
                bpm: cand_bpm,
                camelot: cand_cam,
                similarity_pct: (sim * 10.0).round() / 10.0,
                genre_family: gen_fam,
            })
        })
        .collect();
    matches.sort_by(|a, b| {
        b.similarity_pct
            .partial_cmp(&a.similarity_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches.truncate(limit.clamp(1, 50));
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Connection, SqliteConnection};

    #[tokio::test]
    async fn test_sonic_twins_two_stage_64d_pruning() {
        let mut c = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut c)
            .await
            .unwrap();
        let e1 = B64.encode([90_i8 as u8; 64]);
        let cent = B64.encode([90_i8 as u8; 24]);
        let mk = camelot_code_mask_bit(camelot_to_code("8A")) as i64;
        sqlx::query("INSERT INTO folder_crates (crate_id,folder_path,display_name,tree_seal_64,dir_mtime_ns,bpm_min,bpm_max,camelot_mask,centroid_64d_b64,updated_at_unix_ms) VALUES ('c1','/m','M','0',0,120.0,132.0,?1,?2,0)")
            .bind(mk).bind(&cent).execute(&mut c).await.unwrap();
        sqlx::query("INSERT INTO tracks (id,path,title,album,album_artist,artists,genres,duration,bpm,camelot,hot_cues,scan_level,crate_id,embedding_64d_b64) VALUES ('t1','/m/1.wav','One','EP','DJ','[]','[]',200,126.0,'8A','[]',2,'c1',?1)")
            .bind(&e1).execute(&mut c).await.unwrap();
        sqlx::query("INSERT INTO tracks (id,path,title,album,album_artist,artists,genres,duration,bpm,camelot,hot_cues,scan_level,crate_id,embedding_64d_b64) VALUES ('t2','/m/2.wav','Two','EP','DJ2','[]','[]',200,127.0,'8A','[]',2,'c1',?1)")
            .bind(&e1).execute(&mut c).await.unwrap();
        let twins = compute_sonic_twins(&mut c, "t1", 5).await.unwrap();
        assert_eq!(twins.len(), 1);
        assert!(twins[0].track_id == "t2" && twins[0].similarity_pct > 95.0);
    }
}
