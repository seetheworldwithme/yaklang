use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use crate::db::{
    ensure_standard_tables, insert_bank_transaction, open_db, quote_ident, table_columns,
};
use crate::models::{BankTransaction, MappingFile, NormalizeSummary, TableMapping};
use crate::util::{
    get, infer_account_type, infer_bank_name, make_dedup_key, normalize_time, parse_amount,
    parse_success, resolve_direction_amount, value_ref_to_string,
};

pub fn normalize_command(db: &Path, mapping_path: &Path) -> Result<NormalizeSummary> {
    let conn = open_db(db)?;
    ensure_standard_tables(&conn)?;
    let mapping: MappingFile = serde_json::from_str(
        &fs::read_to_string(mapping_path)
            .with_context(|| format!("读取映射文件失败: {}", mapping_path.display()))?,
    )?;
    let mut inserted = 0;
    let mut duplicates = 0;
    let mut issues = 0;

    for table_mapping in mapping.tables {
        if table_mapping.table_type != "bank_transaction" {
            continue;
        }
        let columns = table_columns(&conn, &table_mapping.raw_table)?;
        let select = columns
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT rowid, {select} FROM {}",
            quote_ident(&table_mapping.raw_table)
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let raw_row_id: i64 = row.get(0)?;
            let mut values = HashMap::new();
            for (idx, col) in columns.iter().enumerate() {
                values.insert(col.clone(), value_ref_to_string(row.get_ref(idx + 1)?));
            }
            match normalize_row(&table_mapping, &values, raw_row_id) {
                Ok(Some(txn)) => match insert_bank_transaction(&conn, &txn) {
                    Ok(true) => inserted += 1,
                    Ok(false) => duplicates += 1,
                    Err(err) => {
                        issues += 1;
                        insert_normalize_issue(&conn, &txn, "insert_error", &err.to_string())?;
                    }
                },
                Ok(None) => issues += 1,
                Err(err) => {
                    issues += 1;
                    insert_raw_issue(&conn, &table_mapping, &values, raw_row_id, &err.to_string())?;
                }
            }
        }
    }

    Ok(NormalizeSummary {
        inserted,
        duplicates,
        issues,
    })
}

fn normalize_row(
    table_mapping: &TableMapping,
    row: &HashMap<String, String>,
    raw_row_id: i64,
) -> Result<Option<BankTransaction>> {
    let cols = &table_mapping.columns;
    if !parse_success(&get(row, &cols.is_success), &table_mapping.direction_map) {
        return Ok(None);
    }
    let (direction, amount) = resolve_direction_amount(row, cols, &table_mapping.direction_map)?;
    let txn_time = normalize_time(&get(row, &cols.txn_time));
    let account_no = get(row, &cols.account_no);
    let account_name = get(row, &cols.account_name);
    let account_id_no = get(row, &cols.account_id_no);
    let counterparty_account = get(row, &cols.counterparty_account);
    let counterparty_bank = get(row, &cols.counterparty_bank);
    let source_file = row.get("_source_file").cloned().unwrap_or_default();
    let bank_name = infer_bank_name(&source_file, &counterparty_bank);
    let txn_serial_no = get(row, &cols.txn_serial_no);
    let voucher_no = get(row, &cols.voucher_no);
    let amount_text = format!("{amount:.2}");
    let dedup_key = make_dedup_key(&[
        &bank_name,
        &account_no,
        &txn_time,
        &amount_text,
        &direction,
        &counterparty_account,
        &txn_serial_no,
        &voucher_no,
    ]);

    Ok(Some(BankTransaction {
        case_id: row.get("_case_id").cloned().unwrap_or_default(),
        source_file,
        sheet_name: row.get("_sheet_name").cloned().unwrap_or_default(),
        row_no: row
            .get("_row_no")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(raw_row_id),
        bank_name,
        account_no,
        account_name: account_name.clone(),
        account_id_no: account_id_no.clone(),
        account_type: infer_account_type(&account_name, &account_id_no),
        txn_time,
        direction,
        amount,
        balance: parse_amount(&get(row, &cols.balance)),
        counterparty_account,
        counterparty_name: get(row, &cols.counterparty_name),
        counterparty_id_no: get(row, &cols.counterparty_id_no),
        counterparty_bank,
        summary: get(row, &cols.summary),
        channel: get(row, &cols.channel),
        location: get(row, &cols.location),
        ip: get(row, &cols.ip),
        mac: get(row, &cols.mac),
        currency: get(row, &cols.currency),
        txn_serial_no,
        voucher_no,
        is_success: true,
        raw_table: table_mapping.raw_table.clone(),
        raw_row_id,
        dedup_key,
    }))
}

fn insert_normalize_issue(
    conn: &Connection,
    txn: &BankTransaction,
    issue_type: &str,
    message: &str,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO import_issues
        (case_id, source_file, sheet_name, row_no, raw_table, issue_type, message)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            txn.case_id,
            txn.source_file,
            txn.sheet_name,
            txn.raw_row_id,
            txn.raw_table,
            issue_type,
            message
        ],
    )?;
    Ok(())
}

fn insert_raw_issue(
    conn: &Connection,
    table_mapping: &TableMapping,
    row: &HashMap<String, String>,
    raw_row_id: i64,
    message: &str,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO import_issues
        (case_id, source_file, sheet_name, row_no, raw_table, issue_type, message)
        VALUES (?1, ?2, ?3, ?4, ?5, 'normalize_error', ?6)
        "#,
        params![
            row.get("_case_id").cloned().unwrap_or_default(),
            row.get("_source_file").cloned().unwrap_or_default(),
            row.get("_sheet_name").cloned().unwrap_or_default(),
            raw_row_id,
            table_mapping.raw_table,
            message
        ],
    )?;
    Ok(())
}
