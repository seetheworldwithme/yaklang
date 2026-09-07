use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn imports_maps_normalizes_and_analyzes_multiple_csv_files() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("case-data");
    fs::create_dir(&data_dir).unwrap();

    fs::write(
        data_dir.join("工商银行_公司流水.csv"),
        "交易时间,交易卡号,交易户名,收付标志,交易金额,交易余额,交易对手账卡号,对手户名,摘要说明,交易流水号\n\
2026-01-01 10:00:00,1001,甲公司,进,100000,100000,9001,客户A,货款,T001\n\
2026-01-01 15:00:00,1001,甲公司,出,80000,20000,8001,张三,转账,T002\n",
    )
    .unwrap();
    fs::write(
        data_dir.join("建设银行_个人流水.csv"),
        "交易时间,交易卡号,交易户名,收付标志,交易金额,交易余额,交易对手账卡号,对手户名,摘要说明,交易流水号\n\
2026-01-02 09:00:00,8001,张三,进,80000,80000,1001,甲公司,转账,P001\n\
2026-01-02 12:00:00,8001,张三,出,78000,2000,7001,李四,提现,P002\n",
    )
    .unwrap();

    let db = temp.path().join("analysis.db");
    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "import",
            data_dir.to_str().unwrap(),
            "--case-id",
            "A001",
            "--db",
            db.to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"imported_tables\""));

    Command::cargo_bin("excelcli")
        .unwrap()
        .args(["inspect", db.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("工商银行_公司流水__csv"));

    let mapping = temp.path().join("mapping.json");
    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "map",
            db.to_str().unwrap(),
            "--profile",
            "bank-transaction",
            "--out",
            mapping.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "normalize",
            db.to_str().unwrap(),
            "--mapping",
            mapping.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"inserted\": 4"));

    let output = Command::cargo_bin("excelcli")
        .unwrap()
        .args(["analyze", "fund", db.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["overview"]["total_count"], 4);
    assert_eq!(json["account_count"], 2);
    assert!(json["top_destinations"].as_array().unwrap().len() >= 2);

    let query_output = Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "query",
            db.to_str().unwrap(),
            "--sql",
            "SELECT direction, COUNT(*) AS count FROM bank_transactions GROUP BY direction ORDER BY direction",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let query_json: Value = serde_json::from_slice(&query_output).unwrap();
    assert_eq!(query_json["row_count"], 2);

    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "query",
            db.to_str().unwrap(),
            "--sql",
            "DELETE FROM bank_transactions",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("仅允许"));

    let txn_output = Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "transactions",
            db.to_str().unwrap(),
            "--account",
            "1001",
            "--direction",
            "out",
            "--min-amount",
            "50000",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let txn_json: Value = serde_json::from_slice(&txn_output).unwrap();
    assert_eq!(txn_json["row_count"], 1);
}

#[test]
fn imports_normalizes_and_analyzes_tax_invoice_files() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("tax-data");
    fs::create_dir(&data_dir).unwrap();

    fs::write(
        data_dir.join("专票销方.csv"),
        "发票代码,发票号码,开票日期,发票类型,作废标志,购方识别号,购方名称,销方识别号,销方名称,货品名称,金额,税率,税额,价税合计\n\
110,0001,2026-01-01,增值税专用发票,正常,B001,下游公司,S001,涉案公司,服务器,87000,13%,13000,100000\n\
110,0002,2026-01-02,增值税普通发票,正常,B002,客户公司,S001,涉案公司,技术服务,400000,0%,0,400000\n",
    )
    .unwrap();
    fs::write(
        data_dir.join("专票购方.csv"),
        "发票代码,发票号码,开票日期,发票类型,作废标志,购方识别号,购方名称,销方识别号,销方名称,货品名称,金额,税率,税额,价税合计\n\
210,1001,2026-01-03,增值税专用发票,作废,S001,涉案公司,U001,上游公司,煤炭,90000,13%,11700,101700\n",
    )
    .unwrap();
    fs::write(
        data_dir.join("纳税人信息.csv"),
        "纳税人识别号,纳税人名称,纳税人状态,登记日期,行业种类,法定代表人姓名\n\
S001,涉案公司,正常,2025-12-20,信息技术服务,张三\n",
    )
    .unwrap();

    let db = temp.path().join("tax.db");
    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "import",
            data_dir.to_str().unwrap(),
            "--case-id",
            "T001",
            "--db",
            db.to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .success();

    let mapping = temp.path().join("tax-mapping.json");
    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "tax",
            "map",
            db.to_str().unwrap(),
            "--out",
            mapping.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "tax",
            "normalize",
            db.to_str().unwrap(),
            "--mapping",
            mapping.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"invoices\": 3"));

    let analysis_output = Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "tax",
            "analyze",
            db.to_str().unwrap(),
            "--taxpayer",
            "S001",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let analysis_json: Value = serde_json::from_slice(&analysis_output).unwrap();
    assert_eq!(analysis_json["overview"]["invoice_count"], 3);
    assert_eq!(analysis_json["by_role_type"].as_array().unwrap().len(), 3);

    let invoice_output = Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "tax",
            "invoices",
            db.to_str().unwrap(),
            "--invoice-type",
            "专票",
            "--min-amount",
            "100000",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let invoice_json: Value = serde_json::from_slice(&invoice_output).unwrap();
    assert_eq!(invoice_json["row_count"], 2);
}

#[test]
fn tax_mapping_handles_real_export_aliases_and_schema_queries() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("tax-real-aliases");
    fs::create_dir(&data_dir).unwrap();

    fs::write(
        data_dir.join("专票销方.csv"),
        "发票代码,发票号码,开票日期,作废标志,购货方识别号,购货方名称,销货方纳税人识别号,销货方名称,货物或应税劳务名称,货物或劳务编码,货物金额,税率,货物税额,价税合计,开票挨批地址,开票麦克地址\n\
110,0001,2026-01-01,N,B001,下游公司,S001,涉案公司,*纺织品*棉纱,10901,87000,13%,13000,100000,10.0.0.1,AA:BB\n",
    )
    .unwrap();
    fs::write(
        data_dir.join("纳税人信息.csv"),
        "纳税人识别号,纳税人名称,纳税人状态名称,登记日期,行业名称,登记注册类型名称,法定代表人姓名\n\
S001,涉案公司,正常,2025-12-20,棉纺纱加工,有限责任公司,张三\n\
S001,涉案公司,正常,2025-12-20,棉纺纱加工,有限责任公司,张三\n",
    )
    .unwrap();

    let db = temp.path().join("tax.db");
    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "import",
            data_dir.to_str().unwrap(),
            "--case-id",
            "T002",
            "--db",
            db.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mapping = temp.path().join("tax-mapping.json");
    let mapping_output = Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "tax",
            "map",
            db.to_str().unwrap(),
            "--out",
            mapping.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let mapping_json: Value = serde_json::from_slice(&mapping_output).unwrap();
    assert_eq!(
        mapping_json["tables"][0]["columns"]["buyer_tax_id"],
        "购货方识别号"
    );
    assert_eq!(
        mapping_json["tables"][0]["columns"]["goods_name"],
        "货物或应税劳务名称"
    );
    assert_eq!(mapping_json["tables"][0]["columns"]["ip"], "开票挨批地址");

    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "tax",
            "normalize",
            db.to_str().unwrap(),
            "--mapping",
            mapping.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"tax_registrations\": 1"));

    let query_output = Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "query",
            db.to_str().unwrap(),
            "--sql",
            "SELECT taxpayer_name, counterparty_name, invoice_type, goods_name, ip, mac FROM tax_invoices",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let query_json: Value = serde_json::from_slice(&query_output).unwrap();
    assert_eq!(query_json["rows"][0]["taxpayer_name"], "涉案公司");
    assert_eq!(query_json["rows"][0]["counterparty_name"], "下游公司");
    assert_eq!(query_json["rows"][0]["invoice_type"], "special_vat");
    assert_eq!(query_json["rows"][0]["goods_name"], "*纺织品*棉纱");
    assert_eq!(query_json["rows"][0]["ip"], "10.0.0.1");
    assert_eq!(query_json["rows"][0]["mac"], "AA:BB");

    Command::cargo_bin("excelcli")
        .unwrap()
        .args([
            "query",
            db.to_str().unwrap(),
            "--sql",
            "PRAGMA table_info(tax_invoices)",
            "--json",
        ])
        .assert()
        .success();
}
