use super::dna::DjStyleDnaProfile;

pub fn build_openai_tools_catalog() -> Vec<serde_json::Value> {
    let raw = crate::mcp::list_mcp_tools();
    let Some(tools) = raw.get("tools").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    tools
        .iter()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?;
            if name == "chat_with_dj_copilot" || name == "ensure_hera_updated" {
                return None;
            }
            let description = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let parameters = t
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "type": "object", "properties": {} }));
            Some(serde_json::json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": description,
                    "parameters": parameters,
                }
            }))
        })
        .collect()
}

pub fn summarize_swarm_downloadable_catalog(state: &crate::p2p::P2PMeshState) -> String {
    let index = state.build_broker_swarm_index();
    if index.is_empty() {
        return "[CATÁLOGO DESCARGABLE DEL ENJAMBRE P2P: 0 pistas activas]".to_string();
    }
    let mut lines = vec![format!(
        "[CATÁLOGO EN VIVO DEL ENJAMBRE P2P DISPONIBLE PARA DESCARGA ({} Masters Verificados)]:",
        index.len()
    )];
    for rec in index.iter().take(12) {
        let boost_ready = rec
            .available_routes
            .iter()
            .any(|r| r.storage_provider == "r2" || r.storage_provider == "r2_cas_zero_copy_link");
        let seeders: Vec<String> = rec
            .available_routes
            .iter()
            .map(|r| format!("{}({})", r.owner_dj, r.storage_provider))
            .collect();
        lines.push(format!(
            "- {} — {} | {:.1} BPM | {} | {} | Replicas: {} | P2P Boost: {} | Blob: {} | Seeders: {}",
            rec.canonical_artist,
            rec.canonical_title,
            rec.bpm,
            rec.camelot_key,
            rec.top_subgenre,
            rec.swarm_replicas_count,
            if boost_ready { "SI (zk-ctr-sha256-v1)" } else { "P2P Directo" },
            rec.opaque_blob_key,
            seeders.join(", ")
        ));
    }
    lines.join("\n")
}

pub fn build_agent_harness_system_prompt(
    dj_handle: &str,
    style_dna: Option<&DjStyleDnaProfile>,
    state: &crate::p2p::P2PMeshState,
    commander_mode: Option<&str>,
) -> String {
    let mode = commander_mode.unwrap_or("executive_boss");
    let commander_instruction = match mode {
        "autonomous_operator" => {
            "[PROTOCOLO COMANDANTE DE CABINA: OPERADOR AUTÓNOMO]\n\
             El usuario es el JEFE y DIRECTOR MUSICAL. Tienes autorización para ejecutar toda la cadena de herramientas de forma autónoma. Al terminar entrega siempre un recibo de ejecución detallado con BPM, tonalidad Camelot, corte espectral FFT y 4 Hot Cues."
        }
        "advisor_only" => {
            "[PROTOCOLO COMANDANTE DE CABINA: AUDITOR Y CONSEJERO]\n\
             El usuario es el JEFE. Proporciona asesoría técnica, diagnóstico armónico y sugerencias de mezcla. No ejecutes descargas ni exportes crates a menos que se te ordene explícitamente."
        }
        _ => {
            "[PROTOCOLO COMANDANTE DE CABINA: COPILOTO EJECUTIVO CON APROBACIÓN (RECOMENDADO)]\n\
             El usuario es el JEFE SUPREMO de cabina. Tu deber es obedecer y preparar opciones impecables con atención quirúrgica a los detalles (BPM, Camelot, frecuencias y 4 Hot Cues). Presenta siempre propuestas estructuradas para que el Jefe elija y apruebe con 1 clic."
        }
    };

    let verified_count = state
        .search(&crate::p2p::P2PSearchQuery {
            q: None,
            camelot: None,
            bpm_min: None,
            bpm_max: None,
            subgenre: None,
            owner_dj: None,
            verified_only: Some(true),
        })
        .len();
    let dna_section = style_dna
        .map(|d| d.prompt_block.clone())
        .unwrap_or_else(|| {
            format!(
                "[MEMORIA ACTIVA DEL DJ BRAIN — PERFIL '{}']\n- Zona BPM: 122-128 BPM (8A/9A)\n- Software: VirtualDJ / Rekordbox",
                dj_handle
            )
        });
    let swarm_catalog_section = summarize_swarm_downloadable_catalog(state);
    format!(
        "You are HERA Agentic Copilot (100% Pure-Rust ReAct Harness) assisting DJ `{dj_handle}` on node `{}` ({verified_count} verified Studio Masters online).\n\
         {commander_instruction}\n\
         Operational Invariants:\n\
         1. Quarantine-First & Zero-Trust Spectral Forensics: only admit/recommend verified tracks passing FFT bandwidth, fake-lossless, synthetic-sine, and <120Hz sub-bass mono phase correlation checks.\n\
         2. 4-Layer Holistic Synergy + Global 2-Opt TSP: sequence crates using Camelot wheel harmony, gradual BPM / 3:4 polyrhythmic bridges, vocal clash avoidance, and 64D style embeddings (`build_dj_crate`).\n\
         3. Swarm-First Invariant: P2P sharing is ALWAYS enabled on community nodes. Every track admitted to library/ must be announced to the swarm.\n\
         4. Multi-Step Pipeline Efficiency: use `hera_code_mode` to chain multi-step filtering + crate building in a single turn when appropriate, and `export_dj_crate` for VirtualDJ, Rekordbox, Traktor, and M3U8 exports.\n\
         5. Always rely on live tool observations over assumptions.\n\n\
         {dna_section}\n\n\
         {swarm_catalog_section}",
        state.node_id
    )
}
