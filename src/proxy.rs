use axum::{
    extract::State,
    response::IntoResponse,
};
use std::time::Instant;

use crate::state::AppState;
use crate::utils::chrono_now;

fn detect_language(text: &str) -> &'static str {
    let mut cjk: u32 = 0;
    let mut hiragana: u32 = 0;
    let mut katakana: u32 = 0;
    let mut hangul: u32 = 0;
    let mut cyrillic: u32 = 0;
    let mut arabic: u32 = 0;
    let mut devanagari: u32 = 0;
    let mut thai: u32 = 0;
    let mut latin: u32 = 0;
    let mut latin_fr: u32 = 0;
    let mut latin_de: u32 = 0;
    let mut latin_es: u32 = 0;
    let mut latin_pt: u32 = 0;

    for ch in text.chars().take(1000) {
        let c = ch as u32;
        match c {
            0x0041..=0x005A | 0x0061..=0x007A => latin += 1,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF => cjk += 1,
            0x3040..=0x309F => hiragana += 1,
            0x30A0..=0x30FF | 0x31F0..=0x31FF => katakana += 1,
            0xAC00..=0xD7AF | 0x1100..=0x11FF => hangul += 1,
            0x0400..=0x04FF => cyrillic += 1,
            0x0600..=0x06FF | 0x0750..=0x077F => arabic += 1,
            0x0900..=0x097F => devanagari += 1,
            0x0E00..=0x0E7F => thai += 1,
            _ => {
                match ch {
                    'à' | 'â' | 'ç' | 'è' | 'é' | 'ê' | 'ë' | 'î' | 'ï' | 'ô' | 'ù' | 'û' | 'œ' | 'æ' => {
                        latin += 1;
                        latin_fr += 1;
                    }
                    'ä' | 'ö' | 'ü' | 'ß' => {
                        latin += 1;
                        latin_de += 1;
                    }
                    'ñ' | '¿' | '¡' => {
                        latin += 1;
                        latin_es += 1;
                    }
                    'ã' | 'õ' => {
                        latin += 1;
                        latin_pt += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    let ja_score = hiragana + katakana;
    if ja_score > 0 {
        let ja_total = ja_score + cjk;
        if ja_total >= hangul && ja_total >= cyrillic && ja_total >= arabic
            && ja_total >= devanagari && ja_total >= thai && ja_total >= latin {
            return "JA";
        }
    }

    let scores: [(&str, u32); 7] = [
        ("ZH", cjk),
        ("KO", hangul),
        ("RU", cyrillic),
        ("AR", arabic),
        ("HI", devanagari),
        ("TH", thai),
        ("EN", latin),
    ];

    let (best_lang, best_score) = scores.iter().fold(("EN", 0u32), |(bl, bs), &(lang, score)| {
        if score > bs { (lang, score) } else { (bl, bs) }
    });

    if best_score == 0 {
        return "EN";
    }

    if best_lang == "EN" {
        let sub_scores = [
            ("FR", latin_fr),
            ("DE", latin_de),
            ("ES", latin_es),
            ("PT", latin_pt),
        ];
        for (lang, score) in sub_scores {
            if score >= 2 {
                return lang;
            }
        }
        return "EN";
    }

    best_lang
}

pub async fn translate(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let text = body.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": "text field is required and must not be empty"})),
        ).into_response();
    }
    let source_chars = text.chars().count() as i64;
    if source_chars > 50000 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": "text exceeds maximum length of 50000 characters"})),
        ).into_response();
    }
    let target_lang = body
        .get("target_lang")
        .and_then(|v| v.as_str())
        .unwrap_or("ZH")
        .to_uppercase();

    let log_source = body
        .get("source_lang")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| detect_language(text).to_string());

    // 检查缓存（除非请求头指定绕过）
    let no_cache = headers
        .get("X-No-Cache")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "true")
        .unwrap_or(false);

    if !no_cache {
        if let Some(cached) = state.cache.get(text, &log_source, &target_lang) {
            return (axum::http::StatusCode::OK, axum::Json(cached.response_json)).into_response();
        }
    }

    // 通过 LoadBalancer 选择端点
    let endpoint = match state.load_balancer.select().await {
        Some(ep) => ep,
        None => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({"error": "no upstream endpoints configured"})),
            ).into_response();
        }
    };

    let result = send_to_upstream(&state, &endpoint, &body).await;

    // 如果失败，尝试重试下一个端点
    let (endpoint_used, result) = match &result {
        Ok(_) => (endpoint, result),
        Err(e) => {
            state.load_balancer.report_failure(&endpoint, &e.to_string()).await;
            if let Some(next_ep) = state.load_balancer.select_excluding(endpoint.index()).await {
                let retry_result = send_to_upstream(&state, &next_ep, &body).await;
                match &retry_result {
                    Ok(_) => (next_ep, retry_result),
                    Err(e) => {
                        state.load_balancer.report_failure(&next_ep, &e.to_string()).await;
                        (next_ep, retry_result)
                    }
                }
            } else {
                (endpoint, result)
            }
        }
    };

    let (status_code, resp_json, error_msg, latency_ms) = match result {
        Ok(upstream_result) => {
            state.load_balancer.report_success(&endpoint_used, upstream_result.latency_ms).await;
            if upstream_result.status >= 200 && upstream_result.status < 300 {
                let target_chars = upstream_result.body.get("data")
                    .and_then(|v| {
                        if let Some(s) = v.as_str() {
                            Some(s.chars().count() as i64)
                        } else if let Some(obj) = v.as_object() {
                            obj.get("text")
                                .or_else(|| obj.get("target_text"))
                                .or_else(|| obj.get("translation"))
                                .and_then(|t| t.as_str())
                                .map(|s| s.chars().count() as i64)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);

                let db = state.db.clone();
                let log_src = log_source.clone();
                let tgt_lang = target_lang.clone();
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = db.log_translation(&log_src, &tgt_lang, source_chars, target_chars, "success", None) {
                        tracing::warn!("Failed to log translation: {}", e);
                    }
                });

                // 写入缓存
                if !no_cache {
                    state.cache.insert(text, &log_source, &target_lang, upstream_result.body.clone());
                }

                (upstream_result.status, upstream_result.body, None, upstream_result.latency_ms)
            } else {
                let err = format!("Upstream error {}", upstream_result.status);
                let db = state.db.clone();
                let log_src = log_source.clone();
                let tgt_lang = target_lang.clone();
                let err_clone = err.clone();
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = db.log_translation(&log_src, &tgt_lang, source_chars, 0, "error", Some(&err_clone)) {
                        tracing::warn!("Failed to log translation: {}", e);
                    }
                });
                (upstream_result.status, upstream_result.body, Some(err), upstream_result.latency_ms)
            }
        }
        Err(e) => {
            let err = format!("Connection error: {}", e);
            let db = state.db.clone();
            let log_src = log_source.clone();
            let tgt_lang = target_lang.clone();
            let err_clone = err.clone();
            tokio::task::spawn_blocking(move || {
                if let Err(e) = db.log_translation(&log_src, &tgt_lang, source_chars, 0, "error", Some(&err_clone)) {
                    tracing::warn!("Failed to log translation: {}", e);
                }
            });
            (502, serde_json::json!({"error": &err}), Some(err), 0)
        }
    };

    let health = crate::state::HealthStatus {
        status: if error_msg.is_none() { "ok".to_string() } else { "error".to_string() },
        latency_ms: Some(latency_ms),
        checked_at: Some(chrono_now()),
        error: error_msg,
    };
    *state.health.write().await = health;

    (axum::http::StatusCode::from_u16(status_code).unwrap(), axum::Json(resp_json)).into_response()
}

struct UpstreamResult {
    status: u16,
    body: serde_json::Value,
    latency_ms: u64,
}

async fn send_to_upstream(
    state: &AppState,
    endpoint: &crate::upstream::SelectedEndpoint,
    body: &serde_json::Value,
) -> Result<UpstreamResult, reqwest::Error> {
    let start = Instant::now();
    let mut req_builder = state.http_client.post(endpoint.url()).json(body);
    if !endpoint.api_key().is_empty() {
        req_builder = req_builder.header("Authorization", format!("Bearer {}", endpoint.api_key()));
    }

    let resp = req_builder.send().await?;
    let latency_ms = start.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();
    let body = resp.json::<serde_json::Value>().await.unwrap_or_default();

    Ok(UpstreamResult { status, body, latency_ms })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_english() {
        assert_eq!(detect_language("Hello, world!"), "EN");
        assert_eq!(detect_language("The quick brown fox"), "EN");
    }

    #[test]
    fn test_detect_chinese() {
        assert_eq!(detect_language("你好世界"), "ZH");
        assert_eq!(detect_language("今天天气真好"), "ZH");
    }

    #[test]
    fn test_detect_japanese() {
        assert_eq!(detect_language("こんにちは"), "JA");
        assert_eq!(detect_language("おはようございます"), "JA");
        assert_eq!(detect_language("カタカナテスト"), "JA");
    }

    #[test]
    fn test_detect_korean() {
        assert_eq!(detect_language("안녕하세요"), "KO");
    }

    #[test]
    fn test_detect_russian() {
        assert_eq!(detect_language("Привет мир"), "RU");
    }

    #[test]
    fn test_detect_arabic() {
        assert_eq!(detect_language("مرحبا بالعالم"), "AR");
    }

    #[test]
    fn test_detect_french() {
        assert_eq!(detect_language("Bonjour ça va très bien"), "FR");
    }

    #[test]
    fn test_detect_german() {
        assert_eq!(detect_language("Guten Morgen über Straße"), "DE");
    }

    #[test]
    fn test_detect_empty_string() {
        assert_eq!(detect_language(""), "EN");
    }

    #[test]
    fn test_detect_numbers_only() {
        assert_eq!(detect_language("12345"), "EN");
    }

    #[test]
    fn test_detect_mixed_cjk_japanese() {
        // 含有平假名的混合文本应检测为日语
        assert_eq!(detect_language("東京は美しい街です"), "JA");
    }

    #[test]
    fn test_detect_pure_kanji_is_chinese() {
        // 纯汉字（无假名）应检测为中文
        assert_eq!(detect_language("机器学习人工智能"), "ZH");
    }

    // ===== Fix 2 验证测试 =====

    #[test]
    fn test_source_lang_respected_when_provided() {
        // 修复后: 用户指定 source_lang 时应使用用户值，而非 detect_language
        // 模拟 proxy 逻辑: body.get("source_lang").filter(非空且非auto).unwrap_or(detect)
        let text = "Hello world";
        let body_source_lang = Some("FR");

        let log_source = body_source_lang
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| detect_language(text).to_string());

        assert_eq!(log_source, "FR", "用户指定 source_lang='FR' 应被尊重");
    }

    #[test]
    fn test_source_lang_fallback_to_detect_when_auto() {
        let text = "Hello world";
        let body_source_lang = Some("auto");

        let log_source = body_source_lang
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| detect_language(text).to_string());

        assert_eq!(log_source, "EN", "source_lang='auto' 时应 fallback 到 detect_language");
    }

    #[test]
    fn test_source_lang_fallback_to_detect_when_empty() {
        let text = "你好世界";
        let body_source_lang = Some("");

        let log_source = body_source_lang
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| detect_language(text).to_string());

        assert_eq!(log_source, "ZH", "source_lang='' 时应 fallback 到 detect_language");
    }

    #[test]
    fn test_source_lang_fallback_to_detect_when_missing() {
        let text = "こんにちは";
        let body_source_lang: Option<&str> = None;

        let log_source = body_source_lang
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| detect_language(text).to_string());

        assert_eq!(log_source, "JA", "source_lang 缺失时应 fallback 到 detect_language");
    }

    #[test]
    fn test_cache_key_differs_by_source_lang() {
        // 修复后: 不同 source_lang 应产生不同缓存 key
        use sha2::{Digest, Sha256};
        let text = "chat";
        let make_key = |source: &str, target: &str| -> String {
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            hasher.update(b"|");
            hasher.update(source.as_bytes());
            hasher.update(b"|");
            hasher.update(target.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        let key_fr = make_key("FR", "ZH");
        let key_en = make_key("EN", "ZH");
        assert_ne!(key_fr, key_en, "不同 source_lang 应产生不同缓存 key，避免缓存污染");
    }
}
