use super::CodeModeStep;
use serde_json::{json, Map, Value};

/// Lightweight parser that translates TypeScript/JS `hera.*` statements & `.filter(...)` chains into sandboxed Rust operations.
pub(super) fn parse_ts_code_to_steps(
    code: &str,
    scope: &mut Map<String, Value>,
    log: &mut Vec<String>,
) -> Result<Vec<CodeModeStep>, String> {
    let mut steps = Vec::new();
    let statements = split_ts_statements(code);

    for raw_stmt in statements {
        let stmt = raw_stmt.trim();
        if stmt.is_empty() || stmt.starts_with("//") || stmt.starts_with("/*") {
            continue;
        }

        if let Some(rest) = stmt.strip_prefix("return ") {
            let ret_str = rest.trim().trim_end_matches(';').trim();
            if let Some((call_name, call_args)) = extract_hera_call(ret_str) {
                steps.push(CodeModeStep {
                    bind: "__last__".to_string(),
                    call: call_name,
                    args: call_args,
                });
                scope.insert(
                    "__return_expr__".to_string(),
                    Value::String("$__last__".to_string()),
                );
            } else if ret_str.starts_with('{') && ret_str.ends_with('}') {
                let obj = parse_loose_js_object(ret_str);
                scope.insert("__return_expr__".to_string(), obj);
            } else {
                scope.insert(
                    "__return_expr__".to_string(),
                    Value::String(format!("${ret_str}")),
                );
            }
            continue;
        }

        let (var_name, rhs) = if let Some(after_const) = stmt
            .strip_prefix("const ")
            .or_else(|| stmt.strip_prefix("let "))
            .or_else(|| stmt.strip_prefix("var "))
        {
            if let Some(eq_idx) = after_const.find('=') {
                let name = after_const[..eq_idx]
                    .split(':')
                    .next()
                    .unwrap_or("res")
                    .trim()
                    .to_string();
                let expr = after_const[eq_idx + 1..]
                    .trim()
                    .trim_end_matches(';')
                    .trim();
                (name, expr)
            } else {
                continue;
            }
        } else {
            ("__anon__".to_string(), stmt.trim_end_matches(';').trim())
        };

        if let Some((call_name, call_args)) = extract_hera_call(rhs) {
            steps.push(CodeModeStep {
                bind: var_name,
                call: call_name,
                args: call_args,
            });
            continue;
        }

        if let Some(filter_idx) = rhs.find(".filter(") {
            let src_var = rhs[..filter_idx].trim();
            let pred_body = &rhs[filter_idx + 8..];
            let mut filter_args = Map::new();
            filter_args.insert("tracks".to_string(), Value::String(format!("${src_var}")));

            if pred_body.contains("mono_compatible_sub") {
                filter_args.insert("require_mono_compatible_sub".to_string(), Value::Bool(true));
            }
            if let Some(q_val) = extract_number_after(pred_body, "quality_score") {
                filter_args.insert("min_quality_score".to_string(), json!(q_val));
            }
            if let Some(bw_val) = extract_number_after(pred_body, "effective_bandwidth_hz") {
                filter_args.insert("min_bandwidth_hz".to_string(), json!(bw_val));
            }
            if let Some(bmin) = extract_number_after(pred_body, "bpm >=")
                .or_else(|| extract_number_after(pred_body, "bpm >"))
            {
                filter_args.insert("bpm_min".to_string(), json!(bmin));
            }
            if let Some(bmax) = extract_number_after(pred_body, "bpm <=")
                .or_else(|| extract_number_after(pred_body, "bpm <"))
            {
                filter_args.insert("bpm_max".to_string(), json!(bmax));
            }

            log.push(format!(
                "CodeMode AST: compiled `{src_var}.filter(...)` -> native Rust predicate filter"
            ));
            steps.push(CodeModeStep {
                bind: var_name,
                call: "hera.filterTracks".to_string(),
                args: Value::Object(filter_args),
            });
        }
    }

    Ok(steps)
}

/// Resolve `"$var"` or `"$var.field"` references against the Code Mode execution scope.
pub(super) fn resolve_variables(val: &Value, scope: &Map<String, Value>) -> Value {
    match val {
        Value::String(s) if s.starts_with('$') => {
            let path = s.trim_start_matches('$');
            if let Some((root, prop)) = path.split_once('.') {
                if let Some(root_val) = scope.get(root) {
                    if prop == "length" {
                        if let Some(arr) = root_val.as_array() {
                            return json!(arr.len());
                        }
                    }
                    if let Some(obj) = root_val.as_object() {
                        if let Some(field_val) = obj.get(prop) {
                            return field_val.clone();
                        }
                    }
                }
            }
            scope.get(path).cloned().unwrap_or(Value::Null)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| resolve_variables(v, scope)).collect())
        }
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), resolve_variables(v, scope));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn split_ts_statements(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;

    for ch in code.chars() {
        match ch {
            '{' | '(' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            '}' | ')' | ']' => {
                depth = (depth - 1).max(0);
                cur.push(ch);
            }
            ';' | '\n' if depth == 0 => {
                if !cur.trim().is_empty() {
                    out.push(cur.trim().to_string());
                }
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn extract_hera_call(expr: &str) -> Option<(String, Value)> {
    let clean = expr.trim().trim_start_matches("await ").trim();
    if !clean.starts_with("hera.") {
        return None;
    }
    let open_paren = clean.find('(')?;
    let close_paren = clean.rfind(')')?;
    let method = clean[..open_paren].trim().to_string();
    let inside = clean[open_paren + 1..close_paren].trim();
    let args = if inside.is_empty() {
        json!({})
    } else if let Ok(v) = serde_json::from_str::<Value>(inside) {
        v
    } else {
        parse_loose_js_object(inside)
    };
    Some((method, args))
}

fn parse_loose_js_object(raw: &str) -> Value {
    let trimmed = raw.trim();
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return v;
    }
    let inner = trimmed
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(trimmed);

    let mut map = Map::new();
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '{' | '[' | '(' => {
                depth += 1;
                cur.push(ch);
            }
            '}' | ']' | ')' => {
                depth = (depth - 1).max(0);
                cur.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }

    for part in parts {
        if let Some((k, v)) = part.split_once(':') {
            let key = k.trim().trim_matches('"').trim_matches('\'').to_string();
            let val_str = v.trim();
            let val = if (val_str.starts_with('"') && val_str.ends_with('"'))
                || (val_str.starts_with('\'') && val_str.ends_with('\''))
            {
                Value::String(val_str[1..val_str.len() - 1].to_string())
            } else if val_str == "true" {
                Value::Bool(true)
            } else if val_str == "false" {
                Value::Bool(false)
            } else if let Ok(n) = val_str.parse::<i64>() {
                json!(n)
            } else if let Ok(f) = val_str.parse::<f64>() {
                json!(f)
            } else {
                Value::String(format!("${val_str}"))
            };
            map.insert(key, val);
        }
    }
    Value::Object(map)
}

fn extract_number_after(text: &str, marker: &str) -> Option<f64> {
    let idx = text.find(marker)?;
    let rest = &text[idx + marker.len()..];
    let mut num_str = String::new();
    let mut started = false;
    for ch in rest.chars() {
        if ch.is_ascii_digit() || (ch == '.' && started) {
            started = true;
            num_str.push(ch);
        } else if started {
            break;
        }
    }
    num_str.parse::<f64>().ok()
}
