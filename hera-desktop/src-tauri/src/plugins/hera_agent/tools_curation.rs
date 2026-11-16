use super::*;
use hera_node_rs::cue_engine::{CueCandidate, CueOrigin, GenreCueTemplate};

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn parse_cue_candidates_arg(args: &Value) -> Vec<CueCandidate> {
    let Some(items) = args
        .get("cues")
        .or_else(|| args.get("manual_cues"))
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let num = item.get("num").and_then(|v| v.as_u64())? as u8;
            let seconds = item.get("seconds").and_then(|v| v.as_f64())?;
            if !(1..=8).contains(&num) || !seconds.is_finite() || seconds < 0.0 {
                return None;
            }
            Some(CueCandidate {
                num,
                name: arg_str(item, "name").unwrap_or_else(|| "Manual".to_string()),
                seconds,
                role: arg_str(item, "role").unwrap_or_else(|| "Phrase".to_string()),
                origin: CueOrigin::DjManual,
            })
        })
        .collect()
}

pub(crate) async fn learn_and_persist_genre_cue_template(
    db: &mut DB,
    genre_family: &str,
    duration_sec: f64,
    manual_cues: &[CueCandidate],
    now: i64,
) -> Result<(GenreCueTemplate, i64), String> {
    let learned = GenreCueTemplate::learn_from_manual_cues(genre_family, duration_sec, manual_cues);
    let existing: Option<(f64, f64, f64, f64, i64)> = sqlx::query_as(
        "SELECT pad5_ratio, pad6_ratio, pad7_ratio, pad8_ratio, sample_count FROM dj_learned_cue_rules WHERE genre_family = ?",
    )
    .bind(&learned.genre_family)
    .fetch_optional(&mut db.connection)
    .await
    .map_err(|e| e.to_string())?;

    let (merged, next_count) = match existing {
        Some((p5, p6, p7, p8, cnt)) if cnt > 0 => {
            let (w, n) = (cnt as f64, (cnt + 1) as f64);
            let m5 = ((p5 * w + learned.pad5_ratio) / n).clamp(0.50, 0.96);
            let m6 = ((p6 * w + learned.pad6_ratio) / n).clamp(m5 + 0.03, 0.96);
            let m7 = ((p7 * w + learned.pad7_ratio) / n).clamp(m6 + 0.03, 0.96);
            let m8 = ((p8 * w + learned.pad8_ratio) / n).clamp(m7 + 0.02, 0.96);
            (
                GenreCueTemplate {
                    genre_family: learned.genre_family,
                    pad5_ratio: m5,
                    pad6_ratio: m6,
                    pad7_ratio: m7,
                    pad8_ratio: m8,
                },
                cnt + 1,
            )
        }
        _ => (learned, 1),
    };

    sqlx::query(
        "INSERT INTO dj_learned_cue_rules (genre_family, pad5_ratio, pad6_ratio, pad7_ratio, pad8_ratio, sample_count, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(genre_family) DO UPDATE SET
           pad5_ratio = excluded.pad5_ratio, pad6_ratio = excluded.pad6_ratio, pad7_ratio = excluded.pad7_ratio,
           pad8_ratio = excluded.pad8_ratio, sample_count = excluded.sample_count, updated_at = excluded.updated_at",
    )
    .bind(&merged.genre_family)
    .bind(merged.pad5_ratio)
    .bind(merged.pad6_ratio)
    .bind(merged.pad7_ratio)
    .bind(merged.pad8_ratio)
    .bind(next_count)
    .bind(now)
    .execute(&mut db.connection)
    .await
    .map_err(|e| e.to_string())?;
    Ok((merged, next_count))
}

async fn handle_record_golden_pair<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    args: &Value,
) -> Result<Value, String> {
    let from_id = arg_str(args, "from_track_id").ok_or("Falta 'from_track_id'")?;
    let to_id = arg_str(args, "to_track_id").ok_or("Falta 'to_track_id'")?;
    let bonus = args
        .get("custom_bonus")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.38);
    let note = arg_str(args, "note");
    let (id, now) = (
        format!("gp-{}", uuid::Uuid::new_v4()),
        now_epoch_ms() / 1000,
    );

    ctx.with_db(|db| {
        Box::pin(async move {
            let f = db.get_track(&from_id).await.ok().flatten();
            let t = db.get_track(&to_id).await.ok().flatten();
            let f_art = f.as_ref().map(|x| x.artists.join(", ")).unwrap_or_default();
            let f_tit = f.as_ref().map(|x| x.title.clone()).unwrap_or_default();
            let t_art = t.as_ref().map(|x| x.artists.join(", ")).unwrap_or_default();
            let t_tit = t.as_ref().map(|x| x.title.clone()).unwrap_or_default();
            sqlx::query(
                "INSERT INTO dj_golden_pairs (id, from_track_id, to_track_id, from_artist, from_title, to_artist, to_title, custom_bonus, note, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .bind(&id).bind(&from_id).bind(&to_id).bind(&f_art).bind(&f_tit)
            .bind(&t_art).bind(&t_tit).bind(bonus).bind(&note).bind(now)
            .execute(&mut db.connection).await.map_err(|e| e.to_string())?;
            Ok(json!({
                "success": true, "golden_pair_id": id,
                "from_track": format!("{f_art} - {f_tit} ({from_id})"),
                "to_track": format!("{t_art} - {t_tit} ({to_id})"),
                "bonus": bonus, "note": note
            }))
        })
    })
    .await
}

async fn handle_learn_genre_cues<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    args: &Value,
) -> Result<Value, String> {
    let track_id = arg_str(args, "track_id");
    let genre_arg = arg_str(args, "genre_family");
    let dur_arg = args.get("duration_sec").and_then(|v| v.as_f64());
    let explicit_cues = parse_cue_candidates_arg(args);
    let now = now_epoch_ms() / 1000;
    let (tpl, samples) = ctx
        .with_db(|db| {
            Box::pin(async move {
                let trk = match track_id {
                    Some(ref tid) => db.get_track(tid).await.map_err(|e| e.to_string())?,
                    None => None,
                };
                let genre = genre_arg
                    .or_else(|| trk.as_ref().and_then(|t| t.inferred_genre_family.clone()))
                    .or_else(|| trk.as_ref().and_then(|t| t.genres.first().cloned()))
                    .unwrap_or_else(|| "General".to_string());
                let dur = dur_arg
                    .or_else(|| trk.as_ref().map(|t| f64::from(t.duration)))
                    .unwrap_or(0.0);
                let cues = if !explicit_cues.is_empty() {
                    explicit_cues
                } else {
                    trk.and_then(|t| t.hot_cues)
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|p| p.origin.as_deref() == Some("dj_manual") || p.num >= 5)
                        .map(|p| CueCandidate {
                            num: p.num,
                            name: p.name,
                            seconds: p.seconds,
                            role: p.role,
                            origin: CueOrigin::DjManual,
                        })
                        .collect()
                };
                if dur <= 1.0 || cues.is_empty() {
                    return Err("Se requiere duración > 1s y un Hot Cue manual".to_string());
                }
                learn_and_persist_genre_cue_template(db, &genre, dur, &cues, now).await
            })
        })
        .await?;
    Ok(json!({
        "success": true, "genre_family": tpl.genre_family, "sample_count": samples,
        "pad5_ratio": tpl.pad5_ratio, "pad6_ratio": tpl.pad6_ratio,
        "pad7_ratio": tpl.pad7_ratio, "pad8_ratio": tpl.pad8_ratio,
    }))
}

pub(super) async fn execute_curation_feedback_tool<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    tool_name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    match tool_name {
        "record_golden_pair" => Some(handle_record_golden_pair(ctx, args).await),
        "exclude_track_curation" => Some(
            async {
                let (tid, pat, reason) = (
                    arg_str(args, "track_id"),
                    arg_str(args, "title_pattern"),
                    arg_str(args, "reason"),
                );
                if tid.is_none() && pat.is_none() {
                    return Err("Debes proporcionar 'track_id' o 'title_pattern'".to_string());
                }
                let (id, now) = (
                    format!("ex-{}", uuid::Uuid::new_v4()),
                    now_epoch_ms() / 1000,
                );
                ctx.with_db(|db| {
                    Box::pin(async move {
                        sqlx::query("INSERT INTO dj_excluded_tracks (id, track_id, title_pattern, reason, created_at) VALUES (?1, ?2, ?3, ?4, ?5)")
                            .bind(&id).bind(&tid).bind(&pat).bind(&reason).bind(now)
                            .execute(&mut db.connection).await.map_err(|e| e.to_string())?;
                        Ok(json!({ "success": true, "exclusion_id": id, "track_id": tid, "title_pattern": pat, "reason": reason }))
                    })
                }).await
            }.await,
        ),
        "record_dj_feedback" => Some(
            async {
                let category = arg_str(args, "category").unwrap_or_else(|| "general".to_string());
                let rule_text = arg_str(args, "rule_text").ok_or("Falta 'rule_text'")?;
                let (id, sid, now) = (format!("fb-{}", uuid::Uuid::new_v4()), session_id.map(String::from), now_epoch_ms() / 1000);
                let res = ctx.with_db(|db| {
                    Box::pin(async move {
                        sqlx::query("INSERT INTO dj_feedback_memory (id, session_id, category, rule_text, created_at) VALUES (?1, ?2, ?3, ?4, ?5)")
                            .bind(&id).bind(&sid).bind(&category).bind(&rule_text).bind(now)
                            .execute(&mut db.connection).await.map_err(|e| e.to_string())?;
                        Ok(json!({ "success": true, "feedback_id": id, "category": category, "rule_text": rule_text }))
                    })
                }).await?;
                if (args.get("cues").is_some() || args.get("manual_cues").is_some())
                    && let Ok(learned) = handle_learn_genre_cues(ctx, args).await
                {
                    return Ok(json!({ "success": true, "feedback": res, "cue_template": learned }));
                }
                Ok(res)
            }.await,
        ),
        "learn_genre_cue_template" => Some(handle_learn_genre_cues(ctx, args).await),
        "get_genre_cue_templates" => Some(
            async {
                let rows: Vec<(String, f64, f64, f64, f64, i64)> = ctx
                    .with_read_conn(|conn| {
                        Box::pin(async move {
                            sqlx::query_as("SELECT genre_family, pad5_ratio, pad6_ratio, pad7_ratio, pad8_ratio, sample_count FROM dj_learned_cue_rules ORDER BY sample_count DESC")
                                .fetch_all(conn).await.map_err(|e| e.to_string())
                        })
                    })
                    .await?;
                let templates: Vec<Value> = rows.into_iter().map(|(g, p5, p6, p7, p8, cnt)| {
                    json!({ "genre_family": g, "pad5_ratio": p5, "pad6_ratio": p6, "pad7_ratio": p7, "pad8_ratio": p8, "sample_count": cnt })
                }).collect();
                Ok(json!({ "templates": templates }))
            }.await,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::{Connection, SqliteConnection};

    #[tokio::test]
    async fn test_learn_genre_cue_template_persists_and_averages() {
        let opts = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);
        let mut conn = SqliteConnection::connect_with(&opts).await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut conn)
            .await
            .unwrap();
        let mut db = DB { connection: conn };
        let c1 = vec![
            CueCandidate {
                num: 5,
                name: "Mambo".into(),
                seconds: 120.0,
                role: "Phrase".into(),
                origin: CueOrigin::DjManual,
            },
            CueCandidate {
                num: 8,
                name: "Cierre".into(),
                seconds: 180.0,
                role: "Outro".into(),
                origin: CueOrigin::DjManual,
            },
        ];
        let (t1, n1) = learn_and_persist_genre_cue_template(&mut db, "SalsaBaul", 200.0, &c1, 1000)
            .await
            .unwrap();
        assert_eq!(
            (
                n1,
                (t1.pad5_ratio - 0.60).abs() < 1e-6,
                (t1.pad8_ratio - 0.90).abs() < 1e-6
            ),
            (1, true, true)
        );
        let c2 = vec![
            CueCandidate {
                num: 5,
                name: "Coro".into(),
                seconds: 112.0,
                role: "Phrase".into(),
                origin: CueOrigin::DjManual,
            },
            CueCandidate {
                num: 8,
                name: "Salida".into(),
                seconds: 172.0,
                role: "Outro".into(),
                origin: CueOrigin::DjManual,
            },
        ];
        let (t2, n2) = learn_and_persist_genre_cue_template(&mut db, "SalsaBaul", 200.0, &c2, 2000)
            .await
            .unwrap();
        assert_eq!(
            (
                n2,
                (t2.pad5_ratio - 0.58).abs() < 1e-6,
                (t2.pad8_ratio - 0.88).abs() < 1e-6
            ),
            (2, true, true)
        );
    }
}
