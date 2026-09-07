use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rusqlite::types::ValueRef;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::models::{BankColumnMapping, DirectionMap};

pub fn sanitize_name(value: &str) -> String {
    let mut out = String::new();
    for ch in value.nfkc().collect::<String>().trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            out.push(ch);
        } else if ch.is_whitespace() || matches!(ch, '-' | '/' | '\\' | '.' | ':' | '：') {
            out.push('_');
        }
    }
    let out = collapse_underscores(&out).trim_matches('_').to_string();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("t_{out}")
    } else {
        out
    }
}

pub fn normalize_match_key(value: &str) -> String {
    sanitize_name(value).replace('_', "").to_ascii_uppercase()
}

fn collapse_underscores(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_was_underscore = false;
    for ch in value.chars() {
        if ch == '_' {
            if !last_was_underscore {
                out.push(ch);
            }
            last_was_underscore = true;
        } else {
            out.push(ch);
            last_was_underscore = false;
        }
    }
    out
}

pub fn unique_column_names(raw: Vec<String>) -> Vec<String> {
    let mut seen = HashMap::<String, usize>::new();
    raw.into_iter()
        .enumerate()
        .map(|(idx, name)| {
            let mut base = sanitize_name(&name);
            if base.is_empty() {
                base = format!("col_{}", idx + 1);
            }
            let count = seen.entry(base.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                base
            } else {
                format!("{base}_{count}")
            }
        })
        .collect()
}

pub fn align_row(row: &mut Vec<String>, len: usize) {
    if row.len() < len {
        row.resize(len, String::new());
    } else if row.len() > len {
        row.truncate(len);
    }
}

pub fn parse_amount(value: &str) -> Option<f64> {
    let cleaned = value
        .replace(',', "")
        .replace('，', "")
        .replace('¥', "")
        .replace('￥', "")
        .replace("元", "")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        None
    } else {
        cleaned.parse::<f64>().ok()
    }
}

pub fn normalize_time(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    for fmt in [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y%m%d",
    ] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, fmt) {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
        if let Ok(date) = NaiveDate::parse_from_str(value, fmt) {
            return date
                .and_time(NaiveTime::MIN)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
        }
    }
    value.to_string()
}

pub fn normalize_direction(value: &str, direction_map: &DirectionMap) -> Option<String> {
    let value = value.trim().to_ascii_uppercase();
    if direction_map
        .r#in
        .iter()
        .any(|v| value == v.to_ascii_uppercase() || value.contains(&v.to_ascii_uppercase()))
    {
        return Some("in".into());
    }
    if direction_map
        .out
        .iter()
        .any(|v| value == v.to_ascii_uppercase() || value.contains(&v.to_ascii_uppercase()))
    {
        return Some("out".into());
    }
    None
}

pub fn parse_success(value: &str, direction_map: &DirectionMap) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return true;
    }
    let normalized = value.to_ascii_uppercase();
    !direction_map
        .fail
        .iter()
        .any(|fail| fail.to_ascii_uppercase() == normalized)
}

pub fn get(row: &HashMap<String, String>, col: &Option<String>) -> String {
    col.as_ref()
        .and_then(|name| row.get(name))
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub fn resolve_direction_amount(
    row: &HashMap<String, String>,
    cols: &BankColumnMapping,
    direction_map: &DirectionMap,
) -> anyhow::Result<(String, f64)> {
    if let Some(direction_col) = &cols.direction {
        let raw_dir = row.get(direction_col).cloned().unwrap_or_default();
        let direction = normalize_direction(&raw_dir, direction_map)
            .ok_or_else(|| anyhow::anyhow!("无法识别收付方向: {raw_dir}"))?;
        let amount = parse_amount(&get(row, &cols.amount))
            .ok_or_else(|| anyhow::anyhow!("无法解析交易金额"))?;
        return Ok((direction, amount.abs()));
    }

    let debit = parse_amount(&get(row, &cols.debit_amount))
        .unwrap_or(0.0)
        .abs();
    let credit = parse_amount(&get(row, &cols.credit_amount))
        .unwrap_or(0.0)
        .abs();
    if credit > 0.0 {
        Ok(("in".into(), credit))
    } else if debit > 0.0 {
        Ok(("out".into(), debit))
    } else {
        anyhow::bail!("无法通过借方/贷方金额识别方向和金额")
    }
}

pub fn infer_account_type(account_name: &str, id_no: &str) -> String {
    if account_name.contains("公司")
        || account_name.contains("有限")
        || account_name.contains("集团")
        || account_name.contains("合作社")
        || account_name.contains("个体工商户")
    {
        "company".into()
    } else if id_no.chars().filter(|c| c.is_ascii_digit()).count() >= 15 {
        "personal".into()
    } else {
        "unknown".into()
    }
}

pub fn infer_bank_name(source_file: &str, counterparty_bank: &str) -> String {
    for bank in [
        "工商银行",
        "农业银行",
        "建设银行",
        "中国银行",
        "交通银行",
        "招商银行",
        "邮储银行",
    ] {
        if source_file.contains(bank) || counterparty_bank.contains(bank) {
            return bank.to_string();
        }
    }
    String::new()
}

pub fn make_dedup_key(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\x1f");
    }
    to_hex(&hasher.finalize())
}

pub fn digest_rows(rows: &[Vec<String>]) -> String {
    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(row.join("\x1f").as_bytes());
        hasher.update(b"\x1e");
    }
    to_hex(&hasher.finalize())
}

pub fn value_ref_to_string(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(v) => v.to_string(),
        ValueRef::Real(v) => v.to_string(),
        ValueRef::Text(v) => String::from_utf8_lossy(v).to_string(),
        ValueRef::Blob(v) => to_hex(v),
    }
}

pub fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names_with_nfkc() {
        assert_eq!(
            sanitize_name(" ２０２６ 年－交易：金额 "),
            "t_2026_年_交易_金额"
        );
        assert_eq!(sanitize_name("ＡＢＣ１２３"), "ABC123");
        assert_eq!(normalize_match_key("借 方　发 生 额"), "借方发生额");
    }

    #[test]
    fn parses_direction_amount_from_debit_credit() {
        let cols = BankColumnMapping {
            debit_amount: Some("借方金额".into()),
            credit_amount: Some("贷方金额".into()),
            ..Default::default()
        };
        let mut row = HashMap::new();
        row.insert("借方金额".into(), String::new());
        row.insert("贷方金额".into(), "1,200.50".into());
        let (direction, amount) =
            resolve_direction_amount(&row, &cols, &DirectionMap::default()).unwrap();
        assert_eq!(direction, "in");
        assert_eq!(amount, 1200.50);
    }
}
