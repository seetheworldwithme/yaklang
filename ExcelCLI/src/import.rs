use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use calamine::{Data, Reader, open_workbook_auto};
use encoding_rs::{Encoding, GB18030, UTF_8};
use rusqlite::Connection;
use uuid::Uuid;

use crate::db::{
    ensure_meta_tables, existing_table_names, insert_issue, open_db, verify_table, write_raw_table,
};
use crate::models::{ImportSummary, ImportedTable};
use crate::util::{align_row, sanitize_name, unique_column_names};

pub fn import_command(
    input: &Path,
    case_id: &str,
    db: &Path,
    verify: bool,
    encoding: Option<&str>,
) -> Result<ImportSummary> {
    let mut conn = open_db(db)?;
    ensure_meta_tables(&conn)?;
    let batch_id = Uuid::new_v4().to_string();
    let mut imported_tables = Vec::new();
    let mut issues = Vec::new();
    let files = collect_input_files(input)?;

    if files.is_empty() {
        bail!("未找到支持的 xls/xlsx/csv 文件: {}", input.display());
    }

    let mut used_names = existing_table_names(&conn)?;
    for file in files {
        match import_file(
            &mut conn,
            case_id,
            &batch_id,
            &file,
            verify,
            encoding,
            &mut used_names,
        ) {
            Ok(mut tables) => imported_tables.append(&mut tables),
            Err(err) => {
                let message = format!("{}: {err:#}", file.display());
                insert_issue(
                    &conn,
                    case_id,
                    &file,
                    "",
                    None,
                    "",
                    "import_error",
                    &message,
                )?;
                issues.push(message);
            }
        }
    }

    Ok(ImportSummary {
        case_id: case_id.to_string(),
        db: db.display().to_string(),
        batch_id,
        imported_tables,
        issues,
    })
}

fn collect_input_files(input: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if input.is_file() {
        if is_supported_file(input) {
            files.push(input.to_path_buf());
        }
    } else if input.is_dir() {
        collect_supported_files(input, &mut files)?;
        files.sort();
    }
    Ok(files)
}

fn collect_supported_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("读取目录失败: {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_supported_files(&path, files)?;
        } else if path.is_file() && is_supported_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_supported_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|v| v.to_str())
            .map(|v| v.to_ascii_lowercase())
            .as_deref(),
        Some("xlsx" | "xls" | "csv")
    )
}

fn import_file(
    conn: &mut Connection,
    case_id: &str,
    batch_id: &str,
    file: &Path,
    verify: bool,
    encoding: Option<&str>,
    used_names: &mut HashSet<String>,
) -> Result<Vec<ImportedTable>> {
    let ext = file
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ext == "csv" {
        import_csv(conn, case_id, batch_id, file, verify, encoding, used_names)
    } else {
        import_excel(conn, case_id, batch_id, file, verify, used_names)
    }
}

fn import_csv(
    conn: &mut Connection,
    case_id: &str,
    batch_id: &str,
    file: &Path,
    verify: bool,
    encoding: Option<&str>,
    used_names: &mut HashSet<String>,
) -> Result<Vec<ImportedTable>> {
    let bytes = fs::read(file).with_context(|| format!("读取 CSV 失败: {}", file.display()))?;
    let text = decode_text(&bytes, encoding)?;
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(Cursor::new(text));
    let columns = unique_column_names(
        rdr.headers()
            .context("CSV 缺少表头")?
            .iter()
            .map(ToString::to_string)
            .collect(),
    );
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        let mut row = rec.iter().map(ToString::to_string).collect::<Vec<_>>();
        align_row(&mut row, columns.len());
        rows.push(row);
    }
    let table = make_table_name(&file_stem(file), "csv", used_names);
    write_raw_table(
        conn, case_id, batch_id, file, "csv", &table, &columns, &rows,
    )?;
    let verified = !verify || verify_table(conn, &table, &columns, &rows)?;
    Ok(vec![ImportedTable {
        table_name: table,
        source_file: file.display().to_string(),
        sheet_name: "csv".into(),
        rows: rows.len(),
        columns: columns.len(),
        verified,
    }])
}

fn decode_text(bytes: &[u8], encoding: Option<&str>) -> Result<String> {
    let enc = encoding
        .and_then(|name| Encoding::for_label(name.as_bytes()))
        .unwrap_or_else(|| {
            if std::str::from_utf8(bytes).is_ok() {
                UTF_8
            } else {
                GB18030
            }
        });
    let (cow, _, had_errors) = enc.decode(bytes);
    if had_errors && encoding.is_some() {
        bail!("按指定编码解码失败");
    }
    Ok(cow.into_owned())
}

fn import_excel(
    conn: &mut Connection,
    case_id: &str,
    batch_id: &str,
    file: &Path,
    verify: bool,
    used_names: &mut HashSet<String>,
) -> Result<Vec<ImportedTable>> {
    let mut workbook =
        open_workbook_auto(file).with_context(|| format!("读取 Excel 失败: {}", file.display()))?;
    let mut imported = Vec::new();

    for sheet_name in workbook.sheet_names().to_owned() {
        let range = workbook
            .worksheet_range(&sheet_name)
            .with_context(|| format!("读取工作表失败: {sheet_name}"))?;
        let Some((columns, rows)) = clean_excel_range(
            range
                .rows()
                .map(|row| row.iter().map(cell_to_text).collect()),
        ) else {
            insert_issue(
                conn,
                case_id,
                file,
                &sheet_name,
                None,
                "",
                "empty_sheet",
                "空工作表，已跳过",
            )?;
            continue;
        };
        let table = make_table_name(&file_stem(file), &sheet_name, used_names);
        write_raw_table(
            conn,
            case_id,
            batch_id,
            file,
            &sheet_name,
            &table,
            &columns,
            &rows,
        )?;
        let verified = !verify || verify_table(conn, &table, &columns, &rows)?;
        imported.push(ImportedTable {
            table_name: table,
            source_file: file.display().to_string(),
            sheet_name,
            rows: rows.len(),
            columns: columns.len(),
            verified,
        });
    }
    Ok(imported)
}

fn clean_excel_range<I>(rows: I) -> Option<(Vec<String>, Vec<Vec<String>>)>
where
    I: IntoIterator<Item = Vec<String>>,
{
    let mut rows = rows
        .into_iter()
        .map(trim_trailing_empty_cells)
        .collect::<Vec<_>>();
    while rows.first().is_some_and(|row| is_empty_row(row)) {
        rows.remove(0);
    }
    while rows.last().is_some_and(|row| is_empty_row(row)) {
        rows.pop();
    }
    if rows.is_empty() {
        return None;
    }

    let header_idx = infer_header_row(&rows);
    let mut data = rows.split_off(header_idx);
    let columns = unique_column_names(data.remove(0));
    if columns.is_empty() {
        return None;
    }

    let body = data
        .into_iter()
        .filter(|row| !is_empty_row(row))
        .map(|mut row| {
            align_row(&mut row, columns.len());
            row
        })
        .collect::<Vec<_>>();
    Some((columns, body))
}

fn trim_trailing_empty_cells(mut row: Vec<String>) -> Vec<String> {
    while row.last().is_some_and(|cell| cell.trim().is_empty()) {
        row.pop();
    }
    row
}

fn is_empty_row(row: &[String]) -> bool {
    row.iter().all(|cell| cell.trim().is_empty())
}

fn infer_header_row(rows: &[Vec<String>]) -> usize {
    let mut best_idx = 0;
    let mut best_score = (0usize, 0usize);
    for (idx, row) in rows.iter().take(50).enumerate() {
        let score = (
            header_alias_score(row),
            row.iter().filter(|cell| !cell.trim().is_empty()).count(),
        );
        if score > best_score {
            best_idx = idx;
            best_score = score;
        }
    }
    best_idx
}

fn header_alias_score(row: &[String]) -> usize {
    const ALIASES: &[&str] = &[
        "日期",
        "交易时间",
        "交易日期",
        "交易类型",
        "凭证种类",
        "凭证号",
        "对方户名",
        "对方账号",
        "对手户名",
        "对手账号",
        "摘要",
        "借方发生额",
        "贷方发生额",
        "余额",
        "收付标志",
        "借贷标志",
        "交易金额",
    ];
    row.iter()
        .filter(|cell| {
            let normalized = cell.trim().replace(' ', "");
            ALIASES.iter().any(|alias| normalized == *alias)
        })
        .count()
}

fn cell_to_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(v) => v.trim().to_string(),
        Data::Float(v) => {
            if v.fract() == 0.0 {
                format!("{v:.0}")
            } else {
                v.to_string()
            }
        }
        Data::Int(v) => v.to_string(),
        Data::Bool(v) => v.to_string(),
        Data::DateTime(v) => v.to_string(),
        Data::DateTimeIso(v) => v.clone(),
        Data::DurationIso(v) => v.clone(),
        Data::Error(v) => format!("{v:?}"),
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("file")
        .to_string()
}

fn make_table_name(workbook: &str, sheet: &str, used_names: &mut HashSet<String>) -> String {
    let base = format!("{}__{}", sanitize_name(workbook), sanitize_name(sheet));
    let base = if base == "__" || base.is_empty() {
        "raw_table".to_string()
    } else {
        base
    };
    let mut candidate = base.clone();
    let mut idx = 2;
    while used_names.contains(&candidate) {
        candidate = format!("{base}_{idx}");
        idx += 1;
    }
    used_names.insert(candidate.clone());
    candidate
}
