use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

use crate::models::BankTransaction;
use crate::util::{align_row, digest_rows, value_ref_to_string};

pub fn open_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建数据库目录失败: {}", parent.display()))?;
        }
    }
    let conn =
        Connection::open(path).with_context(|| format!("打开数据库失败: {}", path.display()))?;
    conn.execute_batch("PRAGMA encoding='UTF-8'; PRAGMA journal_mode=WAL;")?;
    Ok(conn)
}

pub fn ensure_meta_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS raw_tables (
            table_name TEXT PRIMARY KEY,
            case_id TEXT NOT NULL,
            source_file TEXT NOT NULL,
            sheet_name TEXT NOT NULL,
            row_count INTEGER NOT NULL,
            column_count INTEGER NOT NULL,
            columns_json TEXT NOT NULL,
            import_batch_id TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            imported_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS import_issues (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            case_id TEXT,
            source_file TEXT,
            sheet_name TEXT,
            row_no INTEGER,
            raw_table TEXT,
            issue_type TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )?;
    Ok(())
}

pub fn ensure_standard_tables(conn: &Connection) -> Result<()> {
    ensure_meta_tables(conn)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS bank_transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            case_id TEXT NOT NULL,
            source_file TEXT NOT NULL,
            sheet_name TEXT NOT NULL,
            row_no INTEGER NOT NULL,
            bank_name TEXT,
            account_no TEXT,
            account_name TEXT,
            account_id_no TEXT,
            account_type TEXT,
            txn_time TEXT,
            direction TEXT,
            amount REAL,
            balance REAL,
            counterparty_account TEXT,
            counterparty_name TEXT,
            counterparty_id_no TEXT,
            counterparty_bank TEXT,
            summary TEXT,
            channel TEXT,
            location TEXT,
            ip TEXT,
            mac TEXT,
            currency TEXT,
            txn_serial_no TEXT,
            voucher_no TEXT,
            is_success INTEGER,
            raw_table TEXT NOT NULL,
            raw_row_id INTEGER NOT NULL,
            dedup_key TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS account_registry (
            account_no TEXT PRIMARY KEY,
            account_name TEXT,
            account_id_no TEXT,
            account_type TEXT,
            bank_name TEXT,
            first_source_file TEXT,
            is_subject INTEGER DEFAULT 1
        );
        "#,
    )?;
    Ok(())
}

pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn existing_table_names(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
}

pub fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", quote_ident(table)))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn table_row_count(conn: &Connection, table: &str) -> Result<usize> {
    let count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM {}", quote_ident(table)),
        [],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

pub fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            params![table],
            |row| row.get(0),
        )
        .optional()?;
    Ok(exists.is_some())
}

pub fn raw_table_meta(conn: &Connection, table: &str) -> Result<Option<(String, String)>> {
    if !table_exists(conn, "raw_tables")? {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            "SELECT source_file, sheet_name FROM raw_tables WHERE table_name=?1",
            params![table],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

pub fn sample_rows(
    conn: &Connection,
    table: &str,
    columns: &[String],
    sample: usize,
) -> Result<Vec<BTreeMap<String, String>>> {
    if sample == 0 || columns.is_empty() {
        return Ok(Vec::new());
    }
    let select = columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT {select} FROM {} LIMIT {sample}", quote_ident(table));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        let mut item = BTreeMap::new();
        for (idx, col) in columns.iter().enumerate() {
            item.insert(col.clone(), value_ref_to_string(row.get_ref(idx)?));
        }
        Ok(item)
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn insert_issue(
    conn: &Connection,
    case_id: &str,
    file: &Path,
    sheet_name: &str,
    row_no: Option<usize>,
    raw_table: &str,
    issue_type: &str,
    message: &str,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO import_issues (
            case_id, source_file, sheet_name, row_no, raw_table, issue_type, message
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            case_id,
            file.display().to_string(),
            sheet_name,
            row_no.map(|v| v as i64),
            raw_table,
            issue_type,
            message
        ],
    )?;
    Ok(())
}

pub fn write_raw_table(
    conn: &mut Connection,
    case_id: &str,
    batch_id: &str,
    file: &Path,
    sheet_name: &str,
    table: &str,
    columns: &[String],
    rows: &[Vec<String>],
) -> Result<()> {
    let mut all_cols = vec![
        "_case_id".to_string(),
        "_source_file".to_string(),
        "_sheet_name".to_string(),
        "_row_no".to_string(),
        "_import_batch_id".to_string(),
    ];
    all_cols.extend(columns.iter().cloned());
    let defs = all_cols
        .iter()
        .map(|c| format!("{} TEXT", quote_ident(c)))
        .collect::<Vec<_>>()
        .join(", ");
    conn.execute(&format!("DROP TABLE IF EXISTS {}", quote_ident(table)), [])?;
    conn.execute(&format!("CREATE TABLE {} ({defs})", quote_ident(table)), [])?;

    let cols_sql = all_cols
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = vec!["?"; all_cols.len()].join(", ");
    let insert_sql = format!(
        "INSERT INTO {} ({cols_sql}) VALUES ({placeholders})",
        quote_ident(table)
    );
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(&insert_sql)?;
        for (idx, row) in rows.iter().enumerate() {
            let mut values = vec![
                case_id.to_string(),
                file.display().to_string(),
                sheet_name.to_string(),
                (idx + 1).to_string(),
                batch_id.to_string(),
            ];
            values.extend(row.iter().cloned());
            stmt.execute(rusqlite::params_from_iter(values))?;
        }
    }
    tx.commit()?;

    conn.execute(
        r#"
        INSERT OR REPLACE INTO raw_tables (
            table_name, case_id, source_file, sheet_name, row_count,
            column_count, columns_json, import_batch_id, content_hash
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            table,
            case_id,
            file.display().to_string(),
            sheet_name,
            rows.len() as i64,
            columns.len() as i64,
            serde_json::to_string(columns)?,
            batch_id,
            digest_rows(rows)
        ],
    )?;
    Ok(())
}

pub fn verify_table(
    conn: &Connection,
    table: &str,
    columns: &[String],
    rows: &[Vec<String>],
) -> Result<bool> {
    if table_row_count(conn, table)? != rows.len() {
        return Ok(false);
    }
    let db_columns = table_columns(conn, table)?;
    let expected = [
        "_case_id",
        "_source_file",
        "_sheet_name",
        "_row_no",
        "_import_batch_id",
    ]
    .into_iter()
    .map(String::from)
    .chain(columns.iter().cloned())
    .collect::<Vec<_>>();
    if db_columns != expected {
        return Ok(false);
    }
    if columns.is_empty() {
        return Ok(rows.is_empty());
    }

    let select = columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT {select} FROM {} ORDER BY rowid", quote_ident(table));
    let mut stmt = conn.prepare(&sql)?;
    let db_rows = stmt
        .query_map([], |row| {
            let mut values = Vec::with_capacity(columns.len());
            for idx in 0..columns.len() {
                values.push(value_ref_to_string(row.get_ref(idx)?));
            }
            Ok(values)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let src_rows = rows
        .iter()
        .map(|row| {
            let mut values = row.clone();
            align_row(&mut values, columns.len());
            values
        })
        .collect::<Vec<_>>();
    Ok(digest_rows(&db_rows) == digest_rows(&src_rows))
}

pub fn insert_bank_transaction(conn: &Connection, txn: &BankTransaction) -> Result<bool> {
    let changed = conn.execute(
        r#"
        INSERT OR IGNORE INTO bank_transactions (
            case_id, source_file, sheet_name, row_no, bank_name,
            account_no, account_name, account_id_no, account_type,
            txn_time, direction, amount, balance,
            counterparty_account, counterparty_name, counterparty_id_no,
            counterparty_bank, summary, channel, location, ip, mac, currency,
            txn_serial_no, voucher_no, is_success, raw_table, raw_row_id, dedup_key
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
            ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29
        )
        "#,
        params![
            txn.case_id,
            txn.source_file,
            txn.sheet_name,
            txn.row_no,
            txn.bank_name,
            txn.account_no,
            txn.account_name,
            txn.account_id_no,
            txn.account_type,
            txn.txn_time,
            txn.direction,
            txn.amount,
            txn.balance,
            txn.counterparty_account,
            txn.counterparty_name,
            txn.counterparty_id_no,
            txn.counterparty_bank,
            txn.summary,
            txn.channel,
            txn.location,
            txn.ip,
            txn.mac,
            txn.currency,
            txn.txn_serial_no,
            txn.voucher_no,
            txn.is_success as i32,
            txn.raw_table,
            txn.raw_row_id,
            txn.dedup_key
        ],
    )?;
    conn.execute(
        r#"
        INSERT OR IGNORE INTO account_registry (
            account_no, account_name, account_id_no, account_type, bank_name, first_source_file, is_subject
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)
        "#,
        params![
            txn.account_no,
            txn.account_name,
            txn.account_id_no,
            txn.account_type,
            txn.bank_name,
            txn.source_file
        ],
    )?;
    Ok(changed > 0)
}
