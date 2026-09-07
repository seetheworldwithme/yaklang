use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, ToSql, params};

use crate::db::{insert_issue, open_db, quote_ident, table_columns, table_exists};
use crate::inspect::inspect_command;
use crate::mapping::find_col;
use crate::models::{
    RiskIndicator, TaxAnalysis, TaxColumnMapping, TaxCounterpartyStat, TaxInvoiceFilter,
    TaxMappingFile, TaxMonthlyStat, TaxNormalizeSummary, TaxOverview, TaxRoleTypeStat,
    TaxTableMapping,
};
use crate::query::QueryOutput;
use crate::util::{
    get, make_dedup_key, normalize_match_key, normalize_time, parse_amount, value_ref_to_string,
};

pub fn tax_map_command(db: &Path) -> Result<TaxMappingFile> {
    let inspect = inspect_command(db, 0)?;
    let tables = inspect
        .tables
        .into_iter()
        .filter(|table| !is_internal_table(&table.table_name))
        .filter_map(|table| {
            let mapping = infer_tax_mapping(&table.table_name, &table.columns);
            (mapping.score >= 3).then_some(mapping)
        })
        .collect();
    Ok(TaxMappingFile {
        profile: "tax-fraud".into(),
        tables,
    })
}

pub fn tax_normalize_command(db: &Path, mapping_path: &Path) -> Result<TaxNormalizeSummary> {
    let mut conn = open_db(db)?;
    ensure_tax_tables(&conn)?;
    clear_tax_tables(&conn)?;
    let mapping: TaxMappingFile = serde_json::from_str(
        &fs::read_to_string(mapping_path)
            .with_context(|| format!("读取涉税映射文件失败: {}", mapping_path.display()))?,
    )?;

    let mut invoices = 0;
    let mut tax_registrations = 0;
    let mut business_registrations = 0;
    let mut skipped_tables = 0;
    let mut issues = 0;

    for table_mapping in mapping.tables {
        match table_mapping.table_type.as_str() {
            "seller_invoice" | "buyer_invoice" | "invoice" => {
                invoices += normalize_invoice_table(&mut conn, &table_mapping, &mut issues)?;
            }
            "tax_registration" => {
                tax_registrations += normalize_registration_table(
                    &mut conn,
                    &table_mapping,
                    "tax_registrations",
                    &mut issues,
                )?;
            }
            "business_registration" => {
                business_registrations += normalize_registration_table(
                    &mut conn,
                    &table_mapping,
                    "tax_business_registrations",
                    &mut issues,
                )?;
            }
            _ => skipped_tables += 1,
        }
    }
    create_tax_views(&conn)?;

    Ok(TaxNormalizeSummary {
        invoices,
        tax_registrations,
        business_registrations,
        skipped_tables,
        issues,
    })
}

pub fn tax_analyze_command(
    db: &Path,
    taxpayer: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    top: usize,
) -> Result<TaxAnalysis> {
    let conn = open_db(db)?;
    if !table_exists(&conn, "tax_invoices")? {
        bail!("缺少 tax_invoices，请先执行 tax normalize");
    }
    let (where_sql, params) =
        invoice_where_clause(taxpayer, None, None, start, end, None, None, None);
    let overview = query_tax_overview(&conn, &where_sql, &params)?;
    let by_role_type = query_role_type_stats(&conn, &where_sql, &params)?;
    let monthly_trend = query_monthly_stats(&conn, &where_sql, &params)?;
    let top_buyers = query_tax_counterparties(&conn, "buyer", &where_sql, &params, top)?;
    let top_sellers = query_tax_counterparties(&conn, "seller", &where_sql, &params, top)?;
    let risk_indicators = build_tax_risks(&conn, &overview, &where_sql, &params)?;

    Ok(TaxAnalysis {
        overview,
        by_role_type,
        monthly_trend,
        top_buyers,
        top_sellers,
        risk_indicators,
    })
}

pub fn tax_invoices_command(db: &Path, filter: TaxInvoiceFilter) -> Result<QueryOutput> {
    let conn = open_db(db)?;
    if !table_exists(&conn, "tax_invoices")? {
        bail!("缺少 tax_invoices，请先执行 tax normalize");
    }
    let mut sql = String::from(
        r#"
        SELECT
          id, invoice_role, invoice_type, invoice_date, invoice_status,
          invoice_code, invoice_no,
          seller_tax_id, seller_name, buyer_tax_id, buyer_name,
          taxpayer_tax_id, taxpayer_name, counterparty_tax_id, counterparty_name,
          goods_name, amount, tax_amount, total_amount, tax_rate,
          source_file, sheet_name, raw_table, raw_row_id
        FROM tax_invoices
        WHERE 1=1
        "#,
    );
    let mut values: Vec<Box<dyn ToSql>> = Vec::new();
    push_invoice_filters(&mut sql, &mut values, &filter);
    sql.push_str(" ORDER BY invoice_date, id LIMIT ?");
    values.push(Box::new(filter.limit as i64));
    let refs = values
        .iter()
        .map(|v| v.as_ref() as &dyn ToSql)
        .collect::<Vec<_>>();
    query_with_params(&conn, &sql, &refs)
}

pub fn infer_tax_mapping(table: &str, columns: &[String]) -> TaxTableMapping {
    let cols = TaxColumnMapping {
        invoice_code: find_col(columns, &["发票代码", "发票代码号码", "发票代码_"]),
        invoice_no: find_col(columns, &["发票号码", "发票号", "发票No", "发票编号"]),
        invoice_date: find_col(
            columns,
            &["开票日期", "发票日期", "开具日期", "填开日期", "日期"],
        ),
        invoice_type: find_col(columns, &["发票类型", "开具发票类型", "票种", "发票种类"]),
        invoice_status: find_col(
            columns,
            &["作废标志", "发票状态", "状态", "红冲标志", "是否作废"],
        ),
        buyer_tax_id: find_col(
            columns,
            &[
                "购方识别号",
                "购买方识别号",
                "购货方识别号",
                "购方纳税人识别号",
                "购买方纳税人识别号",
                "购货方纳税人识别号",
                "购方税号",
                "购货方税号",
            ],
        ),
        buyer_name: find_col(
            columns,
            &["购方名称", "购买方名称", "购货方名称", "受票方名称"],
        ),
        buyer_region: find_col(
            columns,
            &["购方税务机关", "购方地区", "购买方地址", "购方地址"],
        ),
        seller_tax_id: find_col(
            columns,
            &[
                "销方识别号",
                "销售方识别号",
                "销货方识别号",
                "销方纳税人识别号",
                "销售方纳税人识别号",
                "销货方纳税人识别号",
                "销方税号",
                "销货方税号",
            ],
        ),
        seller_name: find_col(
            columns,
            &["销方名称", "销售方名称", "销货方名称", "开票方名称"],
        ),
        seller_region: find_col(
            columns,
            &["销方税务机关", "销方地区", "销售方地址", "销方地址"],
        ),
        goods_name: find_col(
            columns,
            &[
                "货品名称",
                "商品名称",
                "货物名称",
                "货物或应税劳务名称",
                "货物或劳务名称",
                "项目名称",
                "品名",
            ],
        ),
        goods_code: find_col(
            columns,
            &["商品编码", "货物编码", "货物或劳务编码", "税收分类编码"],
        ),
        amount: find_col(
            columns,
            &["货物金额", "金额", "不含税金额", "合计金额", "销售额"],
        ),
        tax_rate: find_col(columns, &["税率", "征收率"]),
        tax_amount: find_col(columns, &["税额", "合计税额"]),
        total_amount: find_col(
            columns,
            &["价税合计", "价税合计金额", "含税金额", "票面金额"],
        ),
        quantity: find_col(columns, &["数量", "货物数量"]),
        unit_price: find_col(columns, &["单价", "含税单价", "不含税单价"]),
        certify_date: find_col(columns, &["认证日期", "勾选日期", "抵扣日期"]),
        ip: find_col(
            columns,
            &["IP地址", "开票IP", "开票挨批地址", "挨批地址", "IP"],
        ),
        mac: find_col(
            columns,
            &[
                "MAC地址",
                "开票MAC",
                "开票麦克地址",
                "麦克地址",
                "设备MAC",
                "主板序列号",
            ],
        ),
        taxpayer_id: find_col(
            columns,
            &["纳税人识别号", "统一社会信用代码", "社会信用代码", "税号"],
        ),
        taxpayer_name: find_col(columns, &["纳税人名称", "企业名称", "公司名称", "名称"]),
        taxpayer_status: find_col(
            columns,
            &[
                "纳税人状态",
                "纳税人状态名称",
                "纳税人状态代码",
                "登记状态",
                "企业状态",
                "经营状态",
            ],
        ),
        register_date: find_col(
            columns,
            &[
                "登记日期",
                "注册日期",
                "成立日期",
                "开业日期",
                "开业设立日期",
                "设立日期",
            ],
        ),
        industry: find_col(
            columns,
            &[
                "行业种类",
                "行业",
                "行业名称",
                "行业代码",
                "国标行业",
                "行业门类",
                "行业大类",
            ],
        ),
        register_type: find_col(
            columns,
            &[
                "登记注册类型",
                "登记注册类型名称",
                "登记注册类型代码",
                "注册类型",
                "企业类型",
                "企业机构类型",
            ],
        ),
        address: find_col(
            columns,
            &["注册地址", "生产经营地址", "企业地址", "住所", "地址"],
        ),
        legal_person: find_col(
            columns,
            &["法定代表人姓名", "法定代表人", "法人", "法人姓名"],
        ),
        legal_person_id: find_col(
            columns,
            &[
                "法定代表人身份证号码",
                "法定代表人身份证件号码",
                "法人身份证号",
            ],
        ),
        finance_person: find_col(columns, &["财务负责人姓名", "财务负责人"]),
        finance_person_id: find_col(columns, &["财务负责人身份证件号码", "财务负责人身份证号码"]),
        tax_staff: find_col(columns, &["办税人姓名", "办税人", "购票人姓名"]),
        tax_staff_id: find_col(columns, &["办税人身份证件号码", "办税人身份证号码"]),
        phone: find_col(
            columns,
            &["联系电话", "电话", "手机号码", "移动电话", "固定电话"],
        ),
        email: find_col(columns, &["邮箱", "电子邮箱", "电子邮件"]),
        business_scope: find_col(columns, &["经营范围", "业务范围"]),
        registered_capital: find_col(columns, &["注册资本", "注册资金", "认缴出资额"]),
    };

    let invoice_score = [
        cols.invoice_code.is_some() || cols.invoice_no.is_some(),
        cols.invoice_date.is_some(),
        cols.buyer_tax_id.is_some() || cols.buyer_name.is_some(),
        cols.seller_tax_id.is_some() || cols.seller_name.is_some(),
        cols.amount.is_some() || cols.total_amount.is_some(),
        cols.tax_amount.is_some(),
        cols.goods_name.is_some() || cols.goods_code.is_some(),
    ]
    .into_iter()
    .filter(|v| *v)
    .count();
    let tax_reg_score = [
        cols.taxpayer_id.is_some(),
        cols.taxpayer_name.is_some(),
        cols.taxpayer_status.is_some(),
        cols.register_date.is_some(),
        cols.industry.is_some(),
        cols.legal_person.is_some(),
    ]
    .into_iter()
    .filter(|v| *v)
    .count();
    let business_score = [
        cols.taxpayer_id.is_some(),
        cols.taxpayer_name.is_some(),
        cols.business_scope.is_some(),
        cols.registered_capital.is_some(),
        cols.address.is_some(),
        cols.legal_person.is_some(),
    ]
    .into_iter()
    .filter(|v| *v)
    .count();

    let table_key = normalize_match_key(table);
    let (table_type, score) = if invoice_score >= tax_reg_score && invoice_score >= business_score {
        let kind = if table_key.contains("购方")
            || table_key.contains("进项")
            || table_key.contains("收票")
        {
            "buyer_invoice"
        } else if table_key.contains("销方")
            || table_key.contains("销售")
            || table_key.contains("开票")
            || table_key.contains("销项")
        {
            "seller_invoice"
        } else {
            "invoice"
        };
        (kind, invoice_score)
    } else if business_score >= tax_reg_score && business_score >= 3 {
        ("business_registration", business_score)
    } else {
        ("tax_registration", tax_reg_score)
    };

    TaxTableMapping {
        raw_table: table.to_string(),
        table_type: table_type.into(),
        score,
        columns: cols,
    }
}

fn is_internal_table(table: &str) -> bool {
    matches!(
        table,
        "raw_tables"
            | "import_issues"
            | "bank_transactions"
            | "account_registry"
            | "tax_invoices"
            | "tax_registrations"
            | "tax_business_registrations"
            | "seller_invoice"
            | "buyer_invoice"
            | "tax_registration"
            | "business_registration"
    )
}

fn ensure_tax_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS tax_invoices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            case_id TEXT,
            source_file TEXT,
            sheet_name TEXT,
            row_no INTEGER,
            invoice_role TEXT,
            invoice_type TEXT,
            invoice_code TEXT,
            invoice_no TEXT,
            invoice_date TEXT,
            invoice_status TEXT,
            buyer_tax_id TEXT,
            buyer_name TEXT,
            buyer_region TEXT,
            seller_tax_id TEXT,
            seller_name TEXT,
            seller_region TEXT,
            taxpayer_tax_id TEXT,
            taxpayer_name TEXT,
            counterparty_tax_id TEXT,
            counterparty_name TEXT,
            goods_name TEXT,
            goods_code TEXT,
            amount REAL,
            tax_rate TEXT,
            tax_amount REAL,
            total_amount REAL,
            quantity REAL,
            unit_price REAL,
            certify_date TEXT,
            ip TEXT,
            mac TEXT,
            raw_table TEXT NOT NULL,
            raw_row_id INTEGER NOT NULL,
            dedup_key TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS tax_registrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            taxpayer_id TEXT,
            taxpayer_name TEXT,
            taxpayer_status TEXT,
            register_date TEXT,
            industry TEXT,
            register_type TEXT,
            address TEXT,
            legal_person TEXT,
            legal_person_id TEXT,
            finance_person TEXT,
            finance_person_id TEXT,
            tax_staff TEXT,
            tax_staff_id TEXT,
            phone TEXT,
            email TEXT,
            business_scope TEXT,
            registered_capital TEXT,
            raw_table TEXT NOT NULL,
            raw_row_id INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tax_business_registrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            taxpayer_id TEXT,
            taxpayer_name TEXT,
            taxpayer_status TEXT,
            register_date TEXT,
            industry TEXT,
            register_type TEXT,
            address TEXT,
            legal_person TEXT,
            legal_person_id TEXT,
            finance_person TEXT,
            finance_person_id TEXT,
            tax_staff TEXT,
            tax_staff_id TEXT,
            phone TEXT,
            email TEXT,
            business_scope TEXT,
            registered_capital TEXT,
            raw_table TEXT NOT NULL,
            raw_row_id INTEGER NOT NULL
        );
        "#,
    )?;
    ensure_column(conn, "tax_invoices", "taxpayer_tax_id", "TEXT")?;
    ensure_column(conn, "tax_invoices", "taxpayer_name", "TEXT")?;
    ensure_column(conn, "tax_invoices", "counterparty_tax_id", "TEXT")?;
    ensure_column(conn, "tax_invoices", "counterparty_name", "TEXT")?;
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, column_type: &str) -> Result<()> {
    let columns = table_columns(conn, table)?;
    if !columns.iter().any(|existing| existing == column) {
        let sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            quote_ident(table),
            quote_ident(column),
            column_type
        );
        conn.execute(&sql, [])?;
    }
    Ok(())
}

fn clear_tax_tables(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM tax_invoices", [])?;
    conn.execute("DELETE FROM tax_registrations", [])?;
    conn.execute("DELETE FROM tax_business_registrations", [])?;
    Ok(())
}

fn create_tax_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS seller_invoice;
        DROP VIEW IF EXISTS buyer_invoice;
        DROP VIEW IF EXISTS tax_registration;
        DROP VIEW IF EXISTS business_registration;
        CREATE VIEW seller_invoice AS SELECT * FROM tax_invoices WHERE invoice_role='seller';
        CREATE VIEW buyer_invoice AS SELECT * FROM tax_invoices WHERE invoice_role='buyer';
        CREATE VIEW tax_registration AS SELECT * FROM tax_registrations;
        CREATE VIEW business_registration AS SELECT * FROM tax_business_registrations;
        "#,
    )?;
    Ok(())
}

fn normalize_invoice_table(
    conn: &mut Connection,
    table_mapping: &TaxTableMapping,
    issues: &mut usize,
) -> Result<usize> {
    let rows = load_raw_rows(conn, &table_mapping.raw_table)?;
    let role = match table_mapping.table_type.as_str() {
        "seller_invoice" => "seller",
        "buyer_invoice" => "buyer",
        _ => "unknown",
    };
    let mut inserted = 0;
    let tx = conn.transaction()?;
    for (raw_row_id, row) in rows {
        let amount = parse_amount(&get(&row, &table_mapping.columns.amount));
        let tax_amount = parse_amount(&get(&row, &table_mapping.columns.tax_amount));
        let total_amount = parse_amount(&get(&row, &table_mapping.columns.total_amount))
            .or_else(|| amount.zip(tax_amount).map(|(a, t)| a + t))
            .or(amount);
        if amount.is_none() && tax_amount.is_none() && total_amount.is_none() {
            *issues += 1;
            insert_tax_issue(&tx, table_mapping, &row, raw_row_id, "缺少可解析的发票金额")?;
            continue;
        }
        let invoice_code = get(&row, &table_mapping.columns.invoice_code);
        let invoice_no = get(&row, &table_mapping.columns.invoice_no);
        let invoice_date = normalize_time(&get(&row, &table_mapping.columns.invoice_date));
        let buyer_tax_id = get(&row, &table_mapping.columns.buyer_tax_id);
        let buyer_name = get(&row, &table_mapping.columns.buyer_name);
        let seller_tax_id = get(&row, &table_mapping.columns.seller_tax_id);
        let seller_name = get(&row, &table_mapping.columns.seller_name);
        let goods_name = get(&row, &table_mapping.columns.goods_name);
        let (taxpayer_tax_id, taxpayer_name, counterparty_tax_id, counterparty_name) = match role {
            "seller" => (
                seller_tax_id.clone(),
                seller_name.clone(),
                buyer_tax_id.clone(),
                buyer_name.clone(),
            ),
            "buyer" => (
                buyer_tax_id.clone(),
                buyer_name.clone(),
                seller_tax_id.clone(),
                seller_name.clone(),
            ),
            _ => (String::new(), String::new(), String::new(), String::new()),
        };
        let total_text = total_amount.map(|v| format!("{v:.2}")).unwrap_or_default();
        let dedup_key = make_dedup_key(&[
            role,
            &invoice_code,
            &invoice_no,
            &invoice_date,
            &buyer_tax_id,
            &seller_tax_id,
            &goods_name,
            &total_text,
            &table_mapping.raw_table,
            &raw_row_id.to_string(),
        ]);
        let changed = tx.execute(
            r#"
            INSERT OR IGNORE INTO tax_invoices (
                case_id, source_file, sheet_name, row_no, invoice_role, invoice_type,
                invoice_code, invoice_no, invoice_date, invoice_status,
                buyer_tax_id, buyer_name, buyer_region,
                seller_tax_id, seller_name, seller_region,
                taxpayer_tax_id, taxpayer_name, counterparty_tax_id, counterparty_name,
                goods_name, goods_code, amount, tax_rate, tax_amount, total_amount,
                quantity, unit_price, certify_date, ip, mac, raw_table, raw_row_id, dedup_key
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                ?31, ?32, ?33, ?34
            )
            "#,
            params![
                row.get("_case_id").cloned().unwrap_or_default(),
                row.get("_source_file").cloned().unwrap_or_default(),
                row.get("_sheet_name").cloned().unwrap_or_default(),
                row.get("_row_no")
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(raw_row_id),
                role,
                infer_invoice_type(
                    &get(&row, &table_mapping.columns.invoice_type),
                    &table_mapping.raw_table,
                ),
                invoice_code,
                invoice_no,
                invoice_date,
                get(&row, &table_mapping.columns.invoice_status),
                buyer_tax_id,
                buyer_name,
                get(&row, &table_mapping.columns.buyer_region),
                seller_tax_id,
                seller_name,
                get(&row, &table_mapping.columns.seller_region),
                taxpayer_tax_id,
                taxpayer_name,
                counterparty_tax_id,
                counterparty_name,
                goods_name,
                get(&row, &table_mapping.columns.goods_code),
                amount,
                get(&row, &table_mapping.columns.tax_rate),
                tax_amount,
                total_amount,
                parse_amount(&get(&row, &table_mapping.columns.quantity)),
                parse_amount(&get(&row, &table_mapping.columns.unit_price)),
                normalize_time(&get(&row, &table_mapping.columns.certify_date)),
                get(&row, &table_mapping.columns.ip),
                get(&row, &table_mapping.columns.mac),
                table_mapping.raw_table,
                raw_row_id,
                dedup_key,
            ],
        )?;
        inserted += changed;
    }
    tx.commit()?;
    Ok(inserted)
}

fn normalize_registration_table(
    conn: &mut Connection,
    table_mapping: &TaxTableMapping,
    target_table: &str,
    issues: &mut usize,
) -> Result<usize> {
    let rows = load_raw_rows(conn, &table_mapping.raw_table)?;
    let sql = format!(
        r#"
        INSERT INTO {} (
            taxpayer_id, taxpayer_name, taxpayer_status, register_date, industry,
            register_type, address, legal_person, legal_person_id,
            finance_person, finance_person_id, tax_staff, tax_staff_id,
            phone, email, business_scope, registered_capital, raw_table, raw_row_id
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
        )
        "#,
        quote_ident(target_table)
    );
    let tx = conn.transaction()?;
    let mut inserted = 0;
    let mut seen = HashSet::new();
    for (raw_row_id, row) in rows {
        let taxpayer_id = get(&row, &table_mapping.columns.taxpayer_id);
        let taxpayer_name = get(&row, &table_mapping.columns.taxpayer_name);
        if taxpayer_id.is_empty() && taxpayer_name.is_empty() {
            *issues += 1;
            insert_tax_issue(
                &tx,
                table_mapping,
                &row,
                raw_row_id,
                "缺少纳税人识别号和名称",
            )?;
            continue;
        }
        let taxpayer_status = get(&row, &table_mapping.columns.taxpayer_status);
        let register_date = normalize_time(&get(&row, &table_mapping.columns.register_date));
        let industry = get(&row, &table_mapping.columns.industry);
        let register_type = get(&row, &table_mapping.columns.register_type);
        let address = get(&row, &table_mapping.columns.address);
        let legal_person = get(&row, &table_mapping.columns.legal_person);
        let legal_person_id = get(&row, &table_mapping.columns.legal_person_id);
        let finance_person = get(&row, &table_mapping.columns.finance_person);
        let finance_person_id = get(&row, &table_mapping.columns.finance_person_id);
        let tax_staff = get(&row, &table_mapping.columns.tax_staff);
        let tax_staff_id = get(&row, &table_mapping.columns.tax_staff_id);
        let phone = get(&row, &table_mapping.columns.phone);
        let email = get(&row, &table_mapping.columns.email);
        let business_scope = get(&row, &table_mapping.columns.business_scope);
        let registered_capital = get(&row, &table_mapping.columns.registered_capital);
        let dedup_key = make_dedup_key(&[
            &taxpayer_id,
            &taxpayer_name,
            &taxpayer_status,
            &register_date,
            &industry,
            &register_type,
            &address,
            &legal_person,
            &legal_person_id,
            &finance_person,
            &finance_person_id,
            &tax_staff,
            &tax_staff_id,
            &phone,
            &email,
            &business_scope,
            &registered_capital,
        ]);
        if !seen.insert(dedup_key) {
            continue;
        }
        inserted += tx.execute(
            &sql,
            params![
                taxpayer_id,
                taxpayer_name,
                taxpayer_status,
                register_date,
                industry,
                register_type,
                address,
                legal_person,
                legal_person_id,
                finance_person,
                finance_person_id,
                tax_staff,
                tax_staff_id,
                phone,
                email,
                business_scope,
                registered_capital,
                table_mapping.raw_table,
                raw_row_id,
            ],
        )?;
    }
    tx.commit()?;
    Ok(inserted)
}

fn load_raw_rows(conn: &Connection, table: &str) -> Result<Vec<(i64, HashMap<String, String>)>> {
    let columns = table_columns(conn, table)?;
    let select = columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT rowid, {select} FROM {}", quote_ident(table));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| {
            let raw_row_id: i64 = row.get(0)?;
            let mut values = HashMap::new();
            for (idx, col) in columns.iter().enumerate() {
                values.insert(col.clone(), value_ref_to_string(row.get_ref(idx + 1)?));
            }
            Ok((raw_row_id, values))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn insert_tax_issue(
    conn: &Connection,
    table_mapping: &TaxTableMapping,
    row: &HashMap<String, String>,
    raw_row_id: i64,
    message: &str,
) -> Result<()> {
    let file = Path::new(row.get("_source_file").map(String::as_str).unwrap_or(""));
    insert_issue(
        conn,
        row.get("_case_id").map(String::as_str).unwrap_or(""),
        file,
        row.get("_sheet_name").map(String::as_str).unwrap_or(""),
        Some(raw_row_id as usize),
        &table_mapping.raw_table,
        "tax_normalize_error",
        message,
    )
}

fn normalize_invoice_type(value: &str) -> String {
    let key = normalize_match_key(value);
    if key.contains("专用") || key.contains("专票") {
        "special_vat".into()
    } else if key.contains("普通") || key.contains("普票") {
        "normal_vat".into()
    } else {
        value.trim().to_string()
    }
}

fn infer_invoice_type(value: &str, raw_table: &str) -> String {
    let normalized = normalize_invoice_type(value);
    if !normalized.is_empty() {
        return normalized;
    }
    normalize_invoice_type(raw_table)
}

fn push_invoice_filters(
    sql: &mut String,
    values: &mut Vec<Box<dyn ToSql>>,
    filter: &TaxInvoiceFilter,
) {
    if let Some(role) = &filter.role {
        sql.push_str(" AND invoice_role = ?");
        values.push(Box::new(normalize_invoice_role(role)));
    }
    if let Some(taxpayer) = &filter.taxpayer {
        sql.push_str(
            " AND (taxpayer_tax_id = ? OR taxpayer_name LIKE ? OR seller_tax_id = ? OR buyer_tax_id = ? OR seller_name LIKE ? OR buyer_name LIKE ?)",
        );
        values.push(Box::new(taxpayer.clone()));
        let pattern = format!("%{taxpayer}%");
        values.push(Box::new(pattern.clone()));
        values.push(Box::new(taxpayer.clone()));
        values.push(Box::new(taxpayer.clone()));
        values.push(Box::new(pattern.clone()));
        values.push(Box::new(pattern));
    }
    if let Some(counterparty) = &filter.counterparty {
        sql.push_str(
            " AND (counterparty_tax_id = ? OR counterparty_name LIKE ? OR seller_tax_id = ? OR buyer_tax_id = ? OR seller_name LIKE ? OR buyer_name LIKE ?)",
        );
        values.push(Box::new(counterparty.clone()));
        let pattern = format!("%{counterparty}%");
        values.push(Box::new(pattern.clone()));
        values.push(Box::new(counterparty.clone()));
        values.push(Box::new(counterparty.clone()));
        values.push(Box::new(pattern.clone()));
        values.push(Box::new(pattern));
    }
    if let Some(invoice_type) = &filter.invoice_type {
        sql.push_str(" AND invoice_type = ?");
        values.push(Box::new(normalize_invoice_type(invoice_type)));
    }
    if let Some(start) = &filter.start {
        sql.push_str(" AND invoice_date >= ?");
        values.push(Box::new(start.clone()));
    }
    if let Some(end) = &filter.end {
        sql.push_str(" AND invoice_date <= ?");
        values.push(Box::new(end.clone()));
    }
    if let Some(min_amount) = filter.min_amount {
        sql.push_str(" AND COALESCE(total_amount, amount, 0) >= ?");
        values.push(Box::new(min_amount));
    }
    if let Some(max_amount) = filter.max_amount {
        sql.push_str(" AND COALESCE(total_amount, amount, 0) <= ?");
        values.push(Box::new(max_amount));
    }
    if let Some(keyword) = &filter.keyword {
        sql.push_str(" AND (goods_name LIKE ? OR invoice_status LIKE ? OR invoice_type LIKE ?)");
        let pattern = format!("%{keyword}%");
        values.push(Box::new(pattern.clone()));
        values.push(Box::new(pattern.clone()));
        values.push(Box::new(pattern));
    }
}

fn normalize_invoice_role(value: &str) -> String {
    match normalize_match_key(value).as_str() {
        "销方" | "销项" | "销售" | "SELLER" | "OUT" => "seller".into(),
        "购方" | "进项" | "购买" | "BUYER" | "IN" => "buyer".into(),
        _ => value.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn invoice_where_clause(
    taxpayer: Option<&str>,
    role: Option<&str>,
    invoice_type: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    min_amount: Option<f64>,
    max_amount: Option<f64>,
    keyword: Option<&str>,
) -> (String, Vec<Box<dyn ToSql>>) {
    let mut sql = String::from(" WHERE 1=1");
    let mut values = Vec::<Box<dyn ToSql>>::new();
    let filter = TaxInvoiceFilter {
        role: role.map(str::to_string),
        taxpayer: taxpayer.map(str::to_string),
        counterparty: None,
        invoice_type: invoice_type.map(str::to_string),
        start: start.map(str::to_string),
        end: end.map(str::to_string),
        min_amount,
        max_amount,
        keyword: keyword.map(str::to_string),
        limit: 0,
    };
    push_invoice_filters(&mut sql, &mut values, &filter);
    (sql, values)
}

fn query_tax_overview(
    conn: &Connection,
    where_sql: &str,
    values: &[Box<dyn ToSql>],
) -> Result<TaxOverview> {
    let sql = format!(
        r#"
        SELECT
          COUNT(*),
          COALESCE(SUM(COALESCE(amount, 0)), 0),
          COALESCE(SUM(COALESCE(tax_amount, 0)), 0),
          COALESCE(SUM(COALESCE(total_amount, amount, 0)), 0),
          COUNT(DISTINCT COALESCE(NULLIF(seller_tax_id, ''), NULLIF(seller_name, ''))),
          COUNT(DISTINCT COALESCE(NULLIF(buyer_tax_id, ''), NULLIF(buyer_name, ''))),
          NULLIF(MIN(NULLIF(invoice_date, '')), ''),
          NULLIF(MAX(NULLIF(invoice_date, '')), '')
        FROM tax_invoices
        {where_sql}
        "#
    );
    let refs = to_refs(values);
    Ok(conn.query_row(&sql, refs.as_slice(), |row| {
        Ok(TaxOverview {
            invoice_count: row.get::<_, i64>(0)? as usize,
            invoice_amount: row.get(1)?,
            tax_amount: row.get(2)?,
            total_amount: row.get(3)?,
            seller_count: row.get::<_, i64>(4)? as usize,
            buyer_count: row.get::<_, i64>(5)? as usize,
            first_date: row.get(6)?,
            last_date: row.get(7)?,
        })
    })?)
}

fn query_role_type_stats(
    conn: &Connection,
    where_sql: &str,
    values: &[Box<dyn ToSql>],
) -> Result<Vec<TaxRoleTypeStat>> {
    let sql = format!(
        r#"
        SELECT
          COALESCE(NULLIF(invoice_role, ''), 'unknown'),
          COALESCE(NULLIF(invoice_type, ''), 'unknown'),
          COUNT(*),
          COALESCE(SUM(COALESCE(amount, 0)), 0),
          COALESCE(SUM(COALESCE(tax_amount, 0)), 0),
          COALESCE(SUM(COALESCE(total_amount, amount, 0)), 0)
        FROM tax_invoices
        {where_sql}
        GROUP BY invoice_role, invoice_type
        ORDER BY invoice_role, invoice_type
        "#
    );
    let refs = to_refs(values);
    let mut stmt = conn.prepare(&sql)?;
    Ok(stmt
        .query_map(refs.as_slice(), |row| {
            Ok(TaxRoleTypeStat {
                invoice_role: row.get(0)?,
                invoice_type: row.get(1)?,
                count: row.get::<_, i64>(2)? as usize,
                amount: row.get(3)?,
                tax_amount: row.get(4)?,
                total_amount: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn query_monthly_stats(
    conn: &Connection,
    where_sql: &str,
    values: &[Box<dyn ToSql>],
) -> Result<Vec<TaxMonthlyStat>> {
    let sql = format!(
        r#"
        SELECT
          substr(invoice_date, 1, 7),
          COALESCE(NULLIF(invoice_role, ''), 'unknown'),
          COUNT(*),
          COALESCE(SUM(COALESCE(amount, 0)), 0),
          COALESCE(SUM(COALESCE(tax_amount, 0)), 0),
          COALESCE(SUM(COALESCE(total_amount, amount, 0)), 0)
        FROM tax_invoices
        {where_sql} AND invoice_date != ''
        GROUP BY substr(invoice_date, 1, 7), invoice_role
        ORDER BY substr(invoice_date, 1, 7), invoice_role
        "#
    );
    let refs = to_refs(values);
    let mut stmt = conn.prepare(&sql)?;
    Ok(stmt
        .query_map(refs.as_slice(), |row| {
            Ok(TaxMonthlyStat {
                month: row.get(0)?,
                invoice_role: row.get(1)?,
                count: row.get::<_, i64>(2)? as usize,
                amount: row.get(3)?,
                tax_amount: row.get(4)?,
                total_amount: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn query_tax_counterparties(
    conn: &Connection,
    side: &str,
    where_sql: &str,
    values: &[Box<dyn ToSql>],
    limit: usize,
) -> Result<Vec<TaxCounterpartyStat>> {
    let (id_col, name_col) = if side == "buyer" {
        ("buyer_tax_id", "buyer_name")
    } else {
        ("seller_tax_id", "seller_name")
    };
    let sql = format!(
        r#"
        SELECT
          COALESCE(NULLIF({id_col}, ''), '无识别号'),
          COALESCE(NULLIF({name_col}, ''), '未知'),
          COUNT(*),
          COALESCE(SUM(COALESCE(total_amount, amount, 0)), 0),
          COALESCE(SUM(COALESCE(tax_amount, 0)), 0)
        FROM tax_invoices
        {where_sql}
        GROUP BY {id_col}, {name_col}
        ORDER BY SUM(COALESCE(total_amount, amount, 0)) DESC
        LIMIT ?
        "#
    );
    let limit_value = limit as i64;
    let mut refs = to_refs(values);
    refs.push(&limit_value);
    let mut stmt = conn.prepare(&sql)?;
    Ok(stmt
        .query_map(refs.as_slice(), |row| {
            Ok(TaxCounterpartyStat {
                tax_id: row.get(0)?,
                name: row.get(1)?,
                count: row.get::<_, i64>(2)? as usize,
                total_amount: row.get(3)?,
                tax_amount: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn build_tax_risks(
    conn: &Connection,
    overview: &TaxOverview,
    where_sql: &str,
    values: &[Box<dyn ToSql>],
) -> Result<Vec<RiskIndicator>> {
    let mut risks = Vec::new();
    let refs = to_refs(values);

    let special_tax: f64 = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(tax_amount), 0) FROM tax_invoices {where_sql} AND invoice_type='special_vat'"
        ),
        refs.as_slice(),
        |row| row.get(0),
    )?;
    risks.push(RiskIndicator {
        name: "专票虚开税额够罪判断".into(),
        value: special_tax,
        level: if special_tax >= 5_000_000.0 {
            "高度异常".into()
        } else if special_tax >= 500_000.0 {
            "异常".into()
        } else if special_tax >= 50_000.0 {
            "关注".into()
        } else {
            "正常".into()
        },
        message: format!(
            "专票税额合计 {:.2} 元（5万/50万/500万为关键档）",
            special_tax
        ),
    });

    let (normal_count, normal_total): (i64, f64) = conn.query_row(
        &format!(
            "SELECT COUNT(*), COALESCE(SUM(COALESCE(total_amount, amount, 0)), 0) FROM tax_invoices {where_sql} AND invoice_type='normal_vat'"
        ),
        refs.as_slice(),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    risks.push(RiskIndicator {
        name: "普票虚开金额够罪判断".into(),
        value: normal_total,
        level: if normal_total >= 2_000_000.0 || normal_count >= 500 {
            "异常".into()
        } else if normal_total >= 400_000.0 || normal_count >= 100 {
            "关注".into()
        } else {
            "正常".into()
        },
        message: format!(
            "普票价税合计 {:.2} 元、{} 份（40万/100份为入罪参考）",
            normal_total, normal_count
        ),
    });

    let round_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM tax_invoices {where_sql} AND CAST(COALESCE(total_amount, amount, 0) AS INTEGER) % 10000 = 0 AND COALESCE(total_amount, amount, 0) >= 10000"
        ),
        refs.as_slice(),
        |row| row.get(0),
    )?;
    let round_ratio = ratio(round_count as f64, overview.invoice_count as f64);
    risks.push(RiskIndicator {
        name: "整额开票占比".into(),
        value: round_ratio,
        level: threshold_level(round_ratio, 0.25, 0.40),
        message: format!(
            "整万元发票 {} 张，占比 {:.2}%",
            round_count,
            round_ratio * 100.0
        ),
    });

    let void_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM tax_invoices {where_sql} AND (invoice_status LIKE '%作废%' OR invoice_status LIKE '%红冲%')"
        ),
        refs.as_slice(),
        |row| row.get(0),
    )?;
    let void_ratio = ratio(void_count as f64, overview.invoice_count as f64);
    risks.push(RiskIndicator {
        name: "作废红冲占比".into(),
        value: void_ratio,
        level: threshold_level(void_ratio, 0.10, 0.20),
        message: format!(
            "作废/红冲发票 {} 张，占比 {:.2}%",
            void_count,
            void_ratio * 100.0
        ),
    });

    let near_limit_count: i64 = conn.query_row(
        &format!(
            r#"
            SELECT COUNT(*) FROM tax_invoices
            {where_sql}
              AND (
                COALESCE(total_amount, amount, 0) BETWEEN 90000 AND 100000
                OR COALESCE(total_amount, amount, 0) BETWEEN 900000 AND 1000000
                OR COALESCE(total_amount, amount, 0) BETWEEN 9000000 AND 10000000
              )
            "#
        ),
        refs.as_slice(),
        |row| row.get(0),
    )?;
    let near_limit_ratio = ratio(near_limit_count as f64, overview.invoice_count as f64);
    risks.push(RiskIndicator {
        name: "顶额开票占比".into(),
        value: near_limit_ratio,
        level: threshold_level(near_limit_ratio, 0.15, 0.30),
        message: format!(
            "接近10万/100万/1000万限额发票 {} 张，占比 {:.2}%",
            near_limit_count,
            near_limit_ratio * 100.0
        ),
    });

    Ok(risks)
}

fn ratio(value: f64, total: f64) -> f64 {
    if total > 0.0 { value / total } else { 0.0 }
}

fn threshold_level(value: f64, abnormal: f64, high: f64) -> String {
    if value >= high {
        "高度异常".into()
    } else if value >= abnormal {
        "异常".into()
    } else {
        "正常".into()
    }
}

fn query_with_params(conn: &Connection, sql: &str, params: &[&dyn ToSql]) -> Result<QueryOutput> {
    let mut stmt = conn.prepare(sql)?;
    let columns = stmt
        .column_names()
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let rows_iter = stmt.query_map(params, |row| {
        let mut item = std::collections::BTreeMap::new();
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

fn to_refs(values: &[Box<dyn ToSql>]) -> Vec<&dyn ToSql> {
    values.iter().map(|v| v.as_ref() as &dyn ToSql).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_invoice_table() {
        let columns = vec![
            "发票代码".into(),
            "发票号码".into(),
            "开票日期".into(),
            "购方识别号".into(),
            "销方识别号".into(),
            "价税合计".into(),
        ];
        let mapping = infer_tax_mapping("专票销方", &columns);
        assert_eq!(mapping.table_type, "seller_invoice");
        assert_eq!(mapping.columns.total_amount.as_deref(), Some("价税合计"));
    }
}
