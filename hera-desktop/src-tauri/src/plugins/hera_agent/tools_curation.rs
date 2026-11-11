use super::*;

pub(super) async fn execute_curation_feedback_tool<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    tool_name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    match tool_name {
        "record_golden_pair" => Some(
            async {
                let from_track_id = args
                    .get("from_track_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'from_track_id'".to_string())?
                    .to_string();
                let to_track_id = args
                    .get("to_track_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'to_track_id'".to_string())?
                    .to_string();
                let custom_bonus = args
                    .get("custom_bonus")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.38);
                let note = args
                    .get("note")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let id = format!("gp-{}", uuid::Uuid::new_v4());
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                // Fetch track metadata if available for readable history
                let (from_artist, from_title, to_artist, to_title) = ctx
                    .with_db(|db| {
                        let f_id = from_track_id.clone();
                        let t_id = to_track_id.clone();
                        Box::pin(async move {
                            let f = db.get_track(&f_id).await.ok().flatten();
                            let t = db.get_track(&t_id).await.ok().flatten();
                            let f_art = f.as_ref().map(|x| x.artists.join(", ")).unwrap_or_default();
                            let f_tit = f.as_ref().map(|x| x.title.clone()).unwrap_or_default();
                            let t_art = t.as_ref().map(|x| x.artists.join(", ")).unwrap_or_default();
                            let t_tit = t.as_ref().map(|x| x.title.clone()).unwrap_or_default();
                            Ok((f_art, f_tit, t_art, t_tit))
                        })
                    })
                    .await
                    .unwrap_or_default();

                ctx.with_db(|db| {
                    let id_c = id.clone();
                    let f_id_c = from_track_id.clone();
                    let t_id_c = to_track_id.clone();
                    let f_art_c = from_artist.clone();
                    let f_tit_c = from_title.clone();
                    let t_art_c = to_artist.clone();
                    let t_tit_c = to_title.clone();
                    let note_c = note.clone();
                    Box::pin(async move {
                        sqlx::query(
                            r#"
                            INSERT INTO dj_golden_pairs (
                                id, from_track_id, to_track_id, from_artist, from_title,
                                to_artist, to_title, custom_bonus, note, created_at
                            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                            "#,
                        )
                        .bind(id_c)
                        .bind(f_id_c)
                        .bind(t_id_c)
                        .bind(f_art_c)
                        .bind(f_tit_c)
                        .bind(t_art_c)
                        .bind(t_tit_c)
                        .bind(custom_bonus)
                        .bind(note_c)
                        .bind(now)
                        .execute(&mut db.connection)
                        .await
                        .map_err(|e| e.to_string())
                    })
                })
                .await?;

                Ok(json!({
                    "success": true,
                    "golden_pair_id": id,
                    "from_track": format!("{from_artist} - {from_title} ({from_track_id})"),
                    "to_track": format!("{to_artist} - {to_title} ({to_track_id})"),
                    "bonus": custom_bonus,
                    "note": note
                }))
            }
            .await,
        ),
        "exclude_track_curation" => Some(
            async {
                let track_id = args.get("track_id").and_then(|v| v.as_str()).map(String::from);
                let title_pattern = args.get("title_pattern").and_then(|v| v.as_str()).map(String::from);
                let reason = args.get("reason").and_then(|v| v.as_str()).map(String::from);

                if track_id.is_none() && title_pattern.is_none() {
                    return Err("Debes proporcionar 'track_id' o 'title_pattern'".to_string());
                }

                let id = format!("ex-{}", uuid::Uuid::new_v4());
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                ctx.with_db(|db| {
                    let id_c = id.clone();
                    let t_id_c = track_id.clone();
                    let p_c = title_pattern.clone();
                    let r_c = reason.clone();
                    Box::pin(async move {
                        sqlx::query(
                            r#"
                            INSERT INTO dj_excluded_tracks (
                                id, track_id, title_pattern, reason, created_at
                            ) VALUES (?1, ?2, ?3, ?4, ?5)
                            "#,
                        )
                        .bind(id_c)
                        .bind(t_id_c)
                        .bind(p_c)
                        .bind(r_c)
                        .bind(now)
                        .execute(&mut db.connection)
                        .await
                        .map_err(|e| e.to_string())
                    })
                })
                .await?;

                Ok(json!({
                    "success": true,
                    "exclusion_id": id,
                    "track_id": track_id,
                    "title_pattern": title_pattern,
                    "reason": reason
                }))
            }
            .await,
        ),
        "record_dj_feedback" => Some(
            async {
                let category = args
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general")
                    .to_string();
                let rule_text = args
                    .get("rule_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'rule_text'".to_string())?
                    .to_string();

                let id = format!("fb-{}", uuid::Uuid::new_v4());
                let sid = session_id.map(String::from);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                ctx.with_db(|db| {
                    let id_c = id.clone();
                    let sid_c = sid.clone();
                    let cat_c = category.clone();
                    let rule_c = rule_text.clone();
                    Box::pin(async move {
                        sqlx::query(
                            r#"
                            INSERT INTO dj_feedback_memory (
                                id, session_id, category, rule_text, created_at
                            ) VALUES (?1, ?2, ?3, ?4, ?5)
                            "#,
                        )
                        .bind(id_c)
                        .bind(sid_c)
                        .bind(cat_c)
                        .bind(rule_c)
                        .bind(now)
                        .execute(&mut db.connection)
                        .await
                        .map_err(|e| e.to_string())
                    })
                })
                .await?;

                Ok(json!({
                    "success": true,
                    "feedback_id": id,
                    "category": category,
                    "rule_text": rule_text
                }))
            }
            .await,
        ),
        _ => None,
    }
}
