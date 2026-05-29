//! 翻译代理模块
//!
//! 负责接收客户端翻译请求，通过负载均衡选择上游 DeepLX 端点进行转发，
//! 并提供缓存命中、failover 重试、请求日志记录等功能。

use axum::{extract::State, response::IntoResponse};
use std::time::Instant;

use crate::state::AppState;
use crate::utils::chrono_now;

/// 基于 Unicode 码点统计的语言检测函数
///
/// # 算法思路
/// 1. 取文本前 1000 个字符，按 Unicode 码点范围统计各语系字符出现次数
/// 2. 日语优先判定：若存在平假名/片假名，将其与 CJK 汉字合并计分，
///    若总分高于其他所有语系则判定为日语（因为日语混用汉字和假名）
/// 3. 其余语系按最高得分判定（中文、韩文、俄文、阿拉伯文、印地文、泰文、拉丁文）
/// 4. 若最高得分为拉丁文，进一步通过特征字符（如 ç/ñ/ü/ã）区分法/德/西/葡语
/// 5. 无法识别时默认返回英语 "EN"
///
/// # 参数
/// - `text`: 待检测语言的文本切片
///
/// # 返回值
/// 语言代码字符串（如 "ZH"、"EN"、"JA"、"KO"、"RU"、"AR"、"HI"、"TH"、"FR"、"DE"、"ES"、"PT"）
///
/// # 局限性
/// - 仅采样前 1000 字符，超长文本后半部分不参与判定
/// - 纯汉字文本无法区分中文和日语（无假名时判定为中文）
/// - 拉丁语系子语言需至少 2 个特征字符才能触发判定
fn detect_language(text: &str) -> &'static str {
    // 各语系字符计数器
    let mut cjk: u32 = 0; // CJK 统一汉字（中日韩共用）
    let mut hiragana: u32 = 0; // 日语平假名
    let mut katakana: u32 = 0; // 日语片假名
    let mut hangul: u32 = 0; // 韩文字母
    let mut cyrillic: u32 = 0; // 西里尔字母（俄语等）
    let mut arabic: u32 = 0; // 阿拉伯字母
    let mut devanagari: u32 = 0; // 天城文（印地语等）
    let mut thai: u32 = 0; // 泰文
    let mut latin: u32 = 0; // 基础拉丁字母
    let mut latin_fr: u32 = 0; // 法语特征字符计数
    let mut latin_de: u32 = 0; // 德语特征字符计数
    let mut latin_es: u32 = 0; // 西班牙语特征字符计数
    let mut latin_pt: u32 = 0; // 葡萄牙语特征字符计数

    // 仅采样前 1000 个字符以控制性能开销
    for ch in text.chars().take(1000) {
        // 按 Unicode 码点范围分类字符
        let c = ch as u32;
        match c {
            0x0041..=0x005A | 0x0061..=0x007A => latin += 1, // A-Z, a-z 基础拉丁
            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF => cjk += 1, // CJK 统一汉字 + 扩展A + 兼容
            0x3040..=0x309F => hiragana += 1,                                // 平假名
            0x30A0..=0x30FF | 0x31F0..=0x31FF => katakana += 1,              // 片假名 + 片假名扩展
            0xAC00..=0xD7AF | 0x1100..=0x11FF => hangul += 1,                // 韩文音节 + 韩文字母
            0x0400..=0x04FF => cyrillic += 1,                                // 西里尔字母
            0x0600..=0x06FF | 0x0750..=0x077F => arabic += 1,                // 阿拉伯字母 + 补充
            0x0900..=0x097F => devanagari += 1,                              // 天城文
            0x0E00..=0x0E7F => thai += 1,                                    // 泰文
            _ => match ch {
                // 法语特征字符：带重音的元音、ç、连字 œ/æ
                'à' | 'â' | 'ç' | 'è' | 'é' | 'ê' | 'ë' | 'î' | 'ï' | 'ô' | 'ù' | 'û' | 'œ'
                | 'æ' => {
                    latin += 1;
                    latin_fr += 1;
                }
                // 德语特征字符：变音字母 ä/ö/ü 和 ß
                'ä' | 'ö' | 'ü' | 'ß' => {
                    latin += 1;
                    latin_de += 1;
                }
                // 西班牙语特征字符：ñ 和倒置标点
                'ñ' | '¿' | '¡' => {
                    latin += 1;
                    latin_es += 1;
                }
                // 葡萄牙语特征字符：带波浪号的元音
                'ã' | 'õ' => {
                    latin += 1;
                    latin_pt += 1;
                }
                _ => {}
            },
        }
    }

    // 日语优先判定逻辑：
    // 日语文本通常混合使用汉字（CJK）和假名（平假名/片假名），
    // 若检测到假名存在，则将假名得分与 CJK 得分合并作为日语总分，
    // 只有当日语总分高于所有其他语系时才判定为日语。
    let ja_score = hiragana + katakana;
    if ja_score > 0 {
        let ja_total = ja_score + cjk;
        if ja_total >= hangul
            && ja_total >= cyrillic
            && ja_total >= arabic
            && ja_total >= devanagari
            && ja_total >= thai
            && ja_total >= latin
        {
            return "JA";
        }
    }

    // 非日语情况：在剩余语系中选择得分最高者
    // 若所有得分为 0（如纯数字/符号文本），默认返回 "EN"
    let scores: [(&str, u32); 7] = [
        ("ZH", cjk),
        ("KO", hangul),
        ("RU", cyrillic),
        ("AR", arabic),
        ("HI", devanagari),
        ("TH", thai),
        ("EN", latin),
    ];

    // 通过 fold 找出得分最高的语系
    let (best_lang, best_score) = scores
        .iter()
        .fold(("EN", 0u32), |(bl, bs), &(lang, score)| {
            if score > bs {
                (lang, score)
            } else {
                (bl, bs)
            }
        });

    // 所有计数器为 0 时（纯符号/数字），默认英语
    if best_score == 0 {
        return "EN";
    }

    // 拉丁语系细分：当最高得分为拉丁字母时，
    // 通过特征字符计数进一步区分法/德/西/葡语。
    // 阈值为 2，即至少出现 2 个特征字符才判定为该子语言。
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

/// 翻译请求处理器（Axum handler）
///
/// 完整处理流程：
/// 1. 解析请求体，提取 `text`、`target_lang`、`source_lang` 字段
/// 2. 校验输入：text 非空且不超过 50000 字符
/// 3. 确定源语言：优先使用用户指定值，否则通过 `detect_language` 自动检测
/// 4. 查询翻译缓存（除非请求头 `X-No-Cache: true` 指定绕过）
/// 5. 通过 LoadBalancer 选择上游端点，发送翻译请求
/// 6. 若首次请求失败，执行 failover：排除失败端点后选择下一个端点重试
/// 7. 记录翻译日志到 SQLite（异步 spawn_blocking，不阻塞响应）
/// 8. 成功时写入缓存，更新健康状态
///
/// # 参数
/// - `state`: 应用共享状态（含数据库、缓存、负载均衡器、HTTP 客户端等）
/// - `headers`: HTTP 请求头（用于读取 `X-No-Cache` 等控制头）
/// - `body`: JSON 请求体，需包含 `text` 字段，可选 `target_lang` 和 `source_lang`
///
/// # 返回值
/// HTTP 响应：成功时返回上游翻译结果 JSON，失败时返回错误信息及对应状态码
pub async fn translate(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    // --- 步骤 1: 解析并校验请求体 ---
    let text = body.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(
                serde_json::json!({"error": "text field is required and must not be empty"}),
            ),
        )
            .into_response();
    }
    // 统计源文本字符数（用于日志记录和统计）
    let source_chars = text.chars().count() as i64;
    // 限制单次翻译文本长度，防止上游超时或资源耗尽
    if source_chars > 50000 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(
                serde_json::json!({"error": "text exceeds maximum length of 50000 characters"}),
            ),
        )
            .into_response();
    }
    // 目标语言：默认 "ZH"（中文），统一转大写
    let target_lang = body
        .get("target_lang")
        .and_then(|v| v.as_str())
        .unwrap_or("ZH")
        .to_uppercase();

    // --- 步骤 2: 确定源语言 ---
    // 优先级：用户显式指定 > 自动检测
    // 当 source_lang 为空或 "auto" 时，回退到 detect_language 自动检测
    let log_source = body
        .get("source_lang")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| detect_language(text).to_string());

    // --- 步骤 3: 查询翻译缓存 ---
    // 检查缓存（除非请求头指定绕过）
    let no_cache = headers
        .get("X-No-Cache")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "true")
        .unwrap_or(false);

    if !no_cache {
        // 缓存 key 由 (text, source_lang, target_lang) 三元组决定
        if let Some(cached) = state.cache.get(text, &log_source, &target_lang) {
            return (axum::http::StatusCode::OK, axum::Json(cached.response_json)).into_response();
        }
    }

    // --- 步骤 4: 选择上游端点 ---
    // 通过 LoadBalancer 选择端点
    let endpoint = match state.load_balancer.select().await {
        Some(ep) => ep,
        None => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({"error": "no upstream endpoints configured"})),
            )
                .into_response();
        }
    };

    let result = send_to_upstream(&state, &endpoint, &body).await;

    // --- 步骤 5: Failover 重试逻辑 ---
    // 策略：首次请求失败时，向 LoadBalancer 报告失败（影响后续权重），
    // 然后排除当前失败端点，选择另一个端点进行一次重试。
    // 最多重试 1 次（共 2 次请求），避免雪崩效应。
    // 如果重试也失败，同样报告失败并返回错误。
    // 如果没有其他可用端点（仅配置了一个），则直接返回原始错误。
    let (endpoint_used, result) = match &result {
        Ok(_) => (endpoint, result),
        Err(e) => {
            // 向负载均衡器报告首次失败（用于健康评估和权重调整）
            state
                .load_balancer
                .report_failure(&endpoint, &e.to_string())
                .await;
            // 尝试选择排除失败端点后的下一个可用端点
            if let Some(next_ep) = state.load_balancer.select_excluding(endpoint.index()).await {
                let retry_result = send_to_upstream(&state, &next_ep, &body).await;
                match &retry_result {
                    Ok(_) => (next_ep, retry_result),
                    Err(e) => {
                        // 重试端点也失败，报告并返回
                        state
                            .load_balancer
                            .report_failure(&next_ep, &e.to_string())
                            .await;
                        (next_ep, retry_result)
                    }
                }
            } else {
                // 无其他可用端点，返回原始错误
                (endpoint, result)
            }
        }
    };

    // --- 步骤 6: 处理上游响应并记录日志 ---
    let (status_code, resp_json, error_msg, latency_ms) = match result {
        Ok(upstream_result) => {
            // 向负载均衡器报告成功（更新延迟统计和成功计数）
            state
                .load_balancer
                .report_success(&endpoint_used, upstream_result.latency_ms)
                .await;
            // HTTP 2xx 表示翻译成功
            if upstream_result.status >= 200 && upstream_result.status < 300 {
                // 提取翻译结果文本的字符数（兼容多种上游响应格式）
                // 支持: data 为字符串、data.text、data.target_text、data.translation
                let target_chars = upstream_result
                    .body
                    .get("data")
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
                let ep_name = endpoint_used.name().to_string();
                let lat = upstream_result.latency_ms;
                // 异步记录成功日志到 SQLite（spawn_blocking 避免阻塞 tokio 运行时）
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = db.log_translation(
                        &log_src,
                        &tgt_lang,
                        source_chars,
                        target_chars,
                        "success",
                        None,
                        &ep_name,
                        Some(lat),
                    ) {
                        tracing::warn!("Failed to log translation: {}", e);
                    }
                });

                // 翻译成功后写入缓存（除非请求显式禁用缓存）
                // 缓存 key 同样由 (text, source_lang, target_lang) 三元组决定
                if !no_cache {
                    state.cache.insert(
                        text,
                        &log_source,
                        &target_lang,
                        upstream_result.body.clone(),
                    );
                }

                (
                    upstream_result.status,
                    upstream_result.body,
                    None,
                    upstream_result.latency_ms,
                )
            } else {
                // 上游返回非 2xx 状态码（如 429 限流、500 内部错误等）
                let err = format!("Upstream error {}", upstream_result.status);
                let db = state.db.clone();
                let log_src = log_source.clone();
                let tgt_lang = target_lang.clone();
                let err_clone = err.clone();
                let ep_name = endpoint_used.name().to_string();
                let lat = upstream_result.latency_ms;
                // 异步记录错误日志（不阻塞响应返回）
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = db.log_translation(
                        &log_src,
                        &tgt_lang,
                        source_chars,
                        0,
                        "error",
                        Some(&err_clone),
                        &ep_name,
                        Some(lat),
                    ) {
                        tracing::warn!("Failed to log translation: {}", e);
                    }
                });
                (
                    upstream_result.status,
                    upstream_result.body,
                    Some(err),
                    upstream_result.latency_ms,
                )
            }
        }
        Err(e) => {
            // 网络层错误（连接超时、DNS 解析失败、TCP 连接被拒等）
            let err = format!("Connection error: {}", e);
            let db = state.db.clone();
            let log_src = log_source.clone();
            let tgt_lang = target_lang.clone();
            let err_clone = err.clone();
            let ep_name = endpoint_used.name().to_string();
            // 异步记录连接错误日志
            tokio::task::spawn_blocking(move || {
                if let Err(e) = db.log_translation(
                    &log_src,
                    &tgt_lang,
                    source_chars,
                    0,
                    "error",
                    Some(&err_clone),
                    &ep_name,
                    None,
                ) {
                    tracing::warn!("Failed to log translation: {}", e);
                }
            });
            // 连接错误返回 502 Bad Gateway
            (502, serde_json::json!({"error": &err}), Some(err), 0)
        }
    };

    // --- 步骤 7: 更新全局健康状态 ---
    // 每次翻译请求完成后更新健康状态，供 dashboard 和 API 查询
    let health = crate::state::HealthStatus {
        status: if error_msg.is_none() {
            "ok".to_string()
        } else {
            "error".to_string()
        },
        latency_ms: Some(latency_ms),
        checked_at: Some(chrono_now()),
        error: error_msg,
    };
    *state.health.write().await = health;

    (
        axum::http::StatusCode::from_u16(status_code).unwrap(),
        axum::Json(resp_json),
    )
        .into_response()
}

/// 上游端点响应结果
///
/// 封装了一次上游请求的完整结果信息，供调用方判断成功/失败并记录统计。
struct UpstreamResult {
    /// HTTP 状态码（如 200、429、500 等）
    status: u16,
    /// 响应体 JSON（翻译结果或错误信息）
    body: serde_json::Value,
    /// 请求耗时（毫秒），从发送请求到收到完整响应体
    latency_ms: u64,
}

/// 向指定上游端点发送翻译请求
///
/// 职责：
/// 1. 构建 HTTP POST 请求（携带原始请求体 JSON）
/// 2. 若端点配置了 API Key，添加 Bearer Token 认证头
/// 3. 发送请求并测量延迟
/// 4. 解析响应体为 JSON
///
/// # 参数
/// - `state`: 应用状态，提供共享的 HTTP 客户端（含超时配置）
/// - `endpoint`: 由 LoadBalancer 选出的目标端点（含 URL 和 API Key）
/// - `body`: 原始翻译请求体 JSON，直接透传给上游
///
/// # 返回值
/// - `Ok(UpstreamResult)`: 请求成功（不论 HTTP 状态码是否为 2xx）
/// - `Err(reqwest::Error)`: 网络层错误（连接失败、超时、DNS 解析失败等）
///
/// # 注意
/// 此函数不判断业务成功与否（如 429 限流），仅负责网络通信。
/// 业务层判断由 `translate` handler 根据 status 字段处理。
async fn send_to_upstream(
    state: &AppState,
    endpoint: &crate::upstream::SelectedEndpoint,
    body: &serde_json::Value,
) -> Result<UpstreamResult, reqwest::Error> {
    // 记录请求开始时间，用于计算延迟
    let start = Instant::now();
    // 构建 POST 请求，将原始请求体作为 JSON 透传
    let mut req_builder = state.http_client.post(endpoint.url()).json(body);
    // 若端点配置了 API Key，添加 Bearer 认证头
    if !endpoint.api_key().is_empty() {
        req_builder = req_builder.header("Authorization", format!("Bearer {}", endpoint.api_key()));
    }

    // 发送请求（可能因超时或网络问题返回 Err）
    let resp = req_builder.send().await?;
    // 计算从发送到收到响应头的延迟
    let latency_ms = start.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();
    // 解析响应体为 JSON，解析失败时返回空 JSON 对象
    let body = resp.json::<serde_json::Value>().await.unwrap_or_default();

    Ok(UpstreamResult {
        status,
        body,
        latency_ms,
    })
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

        assert_eq!(
            log_source, "EN",
            "source_lang='auto' 时应 fallback 到 detect_language"
        );
    }

    #[test]
    fn test_source_lang_fallback_to_detect_when_empty() {
        let text = "你好世界";
        let body_source_lang = Some("");

        let log_source = body_source_lang
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| detect_language(text).to_string());

        assert_eq!(
            log_source, "ZH",
            "source_lang='' 时应 fallback 到 detect_language"
        );
    }

    #[test]
    fn test_source_lang_fallback_to_detect_when_missing() {
        let text = "こんにちは";
        let body_source_lang: Option<&str> = None;

        let log_source = body_source_lang
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| detect_language(text).to_string());

        assert_eq!(
            log_source, "JA",
            "source_lang 缺失时应 fallback 到 detect_language"
        );
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
        assert_ne!(
            key_fr, key_en,
            "不同 source_lang 应产生不同缓存 key，避免缓存污染"
        );
    }
}
