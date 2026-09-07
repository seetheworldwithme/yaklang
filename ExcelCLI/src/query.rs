use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, ToSql};
use serde::Serialize;

use crate::util::value_ref_to_string;

#[derive(Debug, Serialize)]
pub struct QueryOutput {
    pub columns: Vec<String>,
    pub rows: Vec<BTreeMap<String, String>>,
    pub row_count: usize,
}

#[derive(Debug, Default)]
pub struct TransactionFilter {
    pub account: Option<String>,
    pub counterparty: Option<String>,
    pub direction: Option<String>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub keyword: Option<String>,
    pub limit: usize,
}

pub fn query_command(
    db: &Path,
    sql: Option<&str>,
    file: Option<&Path>,
    limit: usize,
) -> Result<QueryOutput> {
    let sql = match (sql, file) {
        (Some(sql), None) => sql.to_string(),
        (None, Some(file)) => fs::read_to_string(file)
            .with_context(|| format!("读取 SQL 文件失败: {}", file.display()))?,
        (Some(_), Some(_)) => bail!("--sql 和 --file 只能选择一个"),
        (None, None) => bail!("必须提供 --sql 或 --file"),
    };
    run_readonly_query(db, &limit_sql(&sql, limit))
}

pub fn transactions_command(db: &Path, filter: TransactionFilter) -> Result<QueryOutput> {
    let mut sql = String::from(
        r#"
        SELECT
          id, case_id, source_file, sheet_name, row_no,
          account_no, account_name, txn_time, direction, amount, balance,
          counterparty_account, counterparty_name, counterparty_bank,
          summary, channel, raw_table, raw_row_id
        FROM bank_transactions
        WHERE 1=1
        "#,
    );
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(account) = filter.account {
        sql.push_str(" AND (account_no = ? OR counterparty_account = ?)");
        params.push(Box::new(account.clone()));
        params.push(Box::new(account));
    }
    if let Some(counterparty) = filter.counterparty {
        sql.push_str(" AND (counterparty_account = ? OR counterparty_name LIKE ?)");
        params.push(Box::new(counterparty.clone()));
        params.push(Box::new(format!("%{counterparty}%")));
    }
    if let Some(direction) = filter.direction {
        let normalized = normalize_direction_arg(&direction)?;
        sql.push_str(" AND direction = ?");
        params.push(Box::new(normalized));
    }
    if let Some(min_amount) = filter.min_amount {
        sql.push_str(" AND amount >= ?");
        params.push(Box::new(min_amount));
    }
    if let Some(max_amount) = filter.max_amount {
        sql.push_str(" AND amount <= ?");
        params.push(Box::new(max_amount));
    }
    if let Some(start) = filter.start {
        sql.push_str(" AND txn_time >= ?");
        params.push(Box::new(start));
    }
    if let Some(end) = filter.end {
        sql.push_str(" AND txn_time <= ?");
        params.push(Box::new(end));
    }
    if let Some(keyword) = filter.keyword {
        sql.push_str(" AND (summary LIKE ? OR channel LIKE ? OR counterparty_name LIKE ?)");
        let pattern = format!("%{keyword}%");
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern));
    }

    sql.push_str(" ORDER BY txn_time, id LIMIT ?");
    params.push(Box::new(filter.limit as i64));

    let refs = params
        .iter()
        .map(|v| v.as_ref() as &dyn ToSql)
        .collect::<Vec<_>>();
    let conn = open_readonly_db(db)?;
    query_with_params(&conn, &sql, &refs)
}

pub fn print_csv(output: &QueryOutput) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    writer.write_record(&output.columns)?;
    for row in &output.rows {
        let record = output
            .columns
            .iter()
            .map(|col| row.get(col).cloned().unwrap_or_default())
            .collect::<Vec<_>>();
        writer.write_record(record)?;
    }
    writer.flush()?;
    Ok(())
}

fn run_readonly_query(db: &Path, sql: &str) -> Result<QueryOutput> {
    validate_readonly_sql(sql)?;
    let conn = open_readonly_db(db)?;
    query_with_params(&conn, sql, &[])
}

fn open_readonly_db(db: &Path) -> Result<Connection> {
    Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("以只读模式打开数据库失败: {}", db.display()))
}

fn query_with_params(conn: &Connection, sql: &str, params: &[&dyn ToSql]) -> Result<QueryOutput> {
    let mut stmt = conn.prepare(sql)?;
    let columns = stmt
        .column_names()
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let rows_iter = stmt.query_map(params, |row| {
        let mut item = BTreeMap::new();
        for (idx, col) in columns.iter().enumerate() {
            item.insert(col.clone(), value_ref_to_string(row.get_ref(idx)?));
        }
        Ok(item)
    })?;
    let rows = rows_iter.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(QueryOutput {
        columns,
        row_count: rows.len(),
        rows,
    })
}

fn validate_readonly_sql(sql: &str) -> Result<()> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        bail!("SQL 不能为空");
    }
    let upper = trimmed.to_ascii_uppercase();
    if !(upper.starts_with("SELECT") || upper.starts_with("WITH") || upper.starts_with("PRAGMA")) {
        bail!("仅允许 SELECT/WITH/PRAGMA 只读查询");
    }
    let semicolon_count = trimmed.chars().filter(|c| *c == ';').count();
    if semicolon_count > 1 || (semicolon_count == 1 && !trimmed.ends_with(';')) {
        bail!("不允许执行多条 SQL");
    }
    for keyword in [
        "INSERT ",
        "UPDATE ",
        "DELETE ",
        "DROP ",
        "ALTER ",
        "CREATE ",
        "REPLACE ",
        "ATTACH ",
        "DETACH ",
        "VACUUM",
        "REINDEX",
        "PRAGMA WRITABLE_SCHEMA",
    ] {
        if upper.contains(keyword) {
            bail!("SQL 包含非只读关键字: {}", keyword.trim());
        }
    }
    Ok(())
}

fn limit_sql(sql: &str, limit: usize) -> String {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    let upper = trimmed.to_ascii_uppercase();
    if limit == 0 || upper.starts_with("PRAGMA") || upper.contains(" LIMIT ") {
        trimmed.to_string()
    } else {
        format!("{trimmed} LIMIT {limit}")
    }
}

fn normalize_direction_arg(value: &str) -> Result<String> {
    match value {
        "in" | "进" | "收入" | "转入" => Ok("in".into()),
        "out" | "出" | "支出" | "转出" => Ok("out".into()),
        _ => bail!("direction 仅支持 in/out/进/出"),
    }
}
