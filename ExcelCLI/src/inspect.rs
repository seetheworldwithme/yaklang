use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::db::{raw_table_meta, sample_rows, table_columns, table_row_count};
use crate::models::{InspectOutput, TableInfo};

pub fn inspect_command(db: &Path, sample: usize) -> Result<InspectOutput> {
    let conn = Connection::open(db).with_context(|| format!("打开数据库失败: {}", db.display()))?;
    let mut stmt = conn.prepare(
        r#"
        SELECT name FROM sqlite_master
        WHERE type='table'
          AND name NOT LIKE 'sqlite_%'
        ORDER BY name
        "#,
    )?;
    let table_names = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut tables = Vec::new();
    for table in table_names {
        let columns = table_columns(&conn, &table)?;
        let rows = table_row_count(&conn, &table)?;
        let meta = raw_table_meta(&conn, &table)?;
        let samples = sample_rows(&conn, &table, &columns, sample)?;
        tables.push(TableInfo {
            table_name: table,
            source_file: meta.as_ref().map(|m| m.0.clone()),
            sheet_name: meta.as_ref().map(|m| m.1.clone()),
            rows,
            columns,
            samples,
        });
    }
    Ok(InspectOutput { tables })
}

pub fn print_inspect_text(output: &InspectOutput) {
    for table in &output.tables {
        println!(
            "表: {} 行数={} 列数={}",
            table.table_name,
            table.rows,
            table.columns.len()
        );
        if let Some(file) = &table.source_file {
            println!(
                "  来源: {} / {}",
                file,
                table.sheet_name.as_deref().unwrap_or("")
            );
        }
        println!("  列: {}", table.columns.join(", "));
    }
}
