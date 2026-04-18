use super::patterns::*;

/// 内容类型枚举
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "detail")]
pub enum ContentType {
    Json,
    Yaml,
    Url,
    Email,
    PhoneNumber,
    IdCard,
    MathExpression,
    Code(String),      // language hint
    TableData(String), // "tsv" | "csv"
    Image,
    FileList,
    PlainText,
    Unknown,
}

/// 规则分类器：纯规则，< 50ms
pub fn classify_by_rules(content: &str) -> ContentType {
    let trimmed = content.trim();

    // 1. URL
    if URL_RE.is_match(trimmed) {
        return ContentType::Url;
    }

    // 2. Email
    if EMAIL_RE.is_match(trimmed) {
        return ContentType::Email;
    }

    // 3. 手机号
    if PHONE_RE.is_match(trimmed) {
        return ContentType::PhoneNumber;
    }

    // 4. 身份证
    if ID_CARD_RE.is_match(trimmed) {
        return ContentType::IdCard;
    }

    // 5. JSON — try parse
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return ContentType::Json;
    }

    // 6. YAML — try parse (排除纯文本被误判)
    if trimmed.contains(':') && serde_yaml::from_str::<serde_yaml::Value>(trimmed).is_ok() {
        // 进一步检查：至少包含 key: value 结构
        if trimmed.lines().any(|l| {
            let l = l.trim();
            l.contains(": ") || l.ends_with(':')
        }) {
            return ContentType::Yaml;
        }
    }

    // 7. 数学表达式
    if MATH_EXPR_RE.is_match(trimmed) && trimmed.len() > 1 {
        return ContentType::MathExpression;
    }

    // 8. 表格数据检测 (TSV/CSV)
    if let Some(fmt) = detect_table_data(trimmed) {
        return ContentType::TableData(fmt);
    }

    // 9. 代码启发式检测
    if is_likely_code(trimmed) {
        let lang = detect_language(trimmed);
        return ContentType::Code(lang);
    }

    // 10. 默认纯文本
    ContentType::PlainText
}

/// 检测是否为表格数据 (TSV/CSV)，返回格式字符串
fn detect_table_data(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 2 {
        return None;
    }

    // 检测 TSV: 每行都包含 tab，且 tab 数量一致
    let tab_counts: Vec<usize> = lines.iter().map(|l| l.matches('\t').count()).collect();
    if tab_counts[0] > 0 && tab_counts.iter().all(|&c| c == tab_counts[0]) {
        return Some("tsv".to_string());
    }

    // 检测 CSV: 每行都包含逗号，数量一致，且不像代码或普通文本
    let comma_counts: Vec<usize> = lines.iter().map(|l| l.matches(',').count()).collect();
    if comma_counts[0] >= 1
        && comma_counts.iter().all(|&c| c == comma_counts[0])
        && !text.contains('{')  // 排除 JSON
        && !text.contains("import ")
        && !text.contains("function ")
    {
        return Some("csv".to_string());
    }

    None
}

fn is_likely_code(text: &str) -> bool {
    let code_indicators = [
        "function ",
        "fn ",
        "def ",
        "class ",
        "import ",
        "export ",
        "const ",
        "let ",
        "var ",
        "if (",
        "if(",
        "for (",
        "for(",
        "while ",
        "return ",
        "struct ",
        "enum ",
        "impl ",
        "pub fn",
        "async ",
        "await ",
        "=>",
        "->",
        "println!",
        "#include",
        "package ",
        "interface ",
        "namespace ",
    ];
    let count = code_indicators
        .iter()
        .filter(|&&kw| text.contains(kw))
        .count();
    count >= 2 || (count >= 1 && text.lines().count() >= 3)
}

fn detect_language(text: &str) -> String {
    if text.contains("fn ") || text.contains("let mut ") || text.contains("println!") {
        "rust".to_string()
    } else if text.contains("def ") || text.contains("import ") && text.contains(":") {
        "python".to_string()
    } else if text.contains("function ") || text.contains("const ") || text.contains("=>") {
        "javascript".to_string()
    } else if text.contains("public class ") || text.contains("System.out") {
        "java".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_json() {
        assert_eq!(
            classify_by_rules(r#"{"name": "test", "value": 42}"#),
            ContentType::Json
        );
    }

    #[test]
    fn test_classify_url() {
        assert_eq!(
            classify_by_rules("https://github.com/user/repo"),
            ContentType::Url
        );
    }

    #[test]
    fn test_classify_math() {
        assert_eq!(classify_by_rules("3 + 4 * 2"), ContentType::MathExpression);
    }

    #[test]
    fn test_classify_phone() {
        assert_eq!(classify_by_rules("13800138000"), ContentType::PhoneNumber);
    }

    #[test]
    fn test_classify_plain_text() {
        assert_eq!(
            classify_by_rules("这是一段普通文本"),
            ContentType::PlainText
        );
    }

    #[test]
    fn test_classify_tsv() {
        let tsv = "name\tage\tcity\nAlice\t30\tBeijing\nBob\t25\tShanghai";
        assert_eq!(
            classify_by_rules(tsv),
            ContentType::TableData("tsv".to_string())
        );
    }

    #[test]
    fn test_classify_csv() {
        let csv = "name,age,city\nAlice,30,Beijing\nBob,25,Shanghai";
        assert_eq!(
            classify_by_rules(csv),
            ContentType::TableData("csv".to_string())
        );
    }
}
