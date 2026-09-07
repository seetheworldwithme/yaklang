use std::path::Path;

use anyhow::{Result, bail};

use crate::inspect::inspect_command;
use crate::models::{BankColumnMapping, DirectionMap, MappingFile, TableMapping};
use crate::util::normalize_match_key;

pub fn map_command(db: &Path, profile: &str) -> Result<MappingFile> {
    if profile != "bank-transaction" {
        bail!("当前仅支持 profile=bank-transaction");
    }
    let inspect = inspect_command(db, 0)?;
    let tables = inspect
        .tables
        .into_iter()
        .filter(|table| !is_internal_table(&table.table_name))
        .map(|table| infer_bank_mapping(&table.table_name, &table.columns))
        .filter(|mapping| mapping.score >= 3)
        .collect();
    Ok(MappingFile {
        profile: profile.to_string(),
        tables,
    })
}

fn is_internal_table(table: &str) -> bool {
    matches!(
        table,
        "raw_tables" | "import_issues" | "bank_transactions" | "account_registry"
    )
}

pub fn infer_bank_mapping(table: &str, columns: &[String]) -> TableMapping {
    let mapping = BankColumnMapping {
        account_no: find_col(
            columns,
            &[
                "交易卡号",
                "本方账号",
                "账户账号",
                "账户号码",
                "主账号",
                "交易账号",
            ],
        ),
        account_name: find_col(
            columns,
            &["交易户名", "户名", "账户名称", "客户姓名", "账户开户名称"],
        ),
        account_id_no: find_col(
            columns,
            &["交易证件号", "证件号码", "身份证号", "身份证号码", "证件号"],
        ),
        txn_time: find_col(
            columns,
            &[
                "交易时间",
                "交易日期",
                "日期",
                "记账时间",
                "发生日期",
                "交易发生时间",
            ],
        ),
        direction: find_col(
            columns,
            &["收付标志", "借贷标志", "交易方向", "借贷方向", "收支标志"],
        ),
        amount: find_col(columns, &["交易金额", "金额", "发生额", "交易发生金额"]),
        debit_amount: find_col(
            columns,
            &["借方金额", "借方发生额", "支出金额", "付方金额", "转出金额"],
        ),
        credit_amount: find_col(
            columns,
            &["贷方金额", "贷方发生额", "收入金额", "收方金额", "转入金额"],
        ),
        balance: find_col(columns, &["交易余额", "余额", "账户余额", "当前余额"]),
        counterparty_account: find_col(
            columns,
            &[
                "交易对手账卡号",
                "对手账号",
                "对方账户",
                "对手卡号",
                "对方账号",
            ],
        ),
        counterparty_name: find_col(columns, &["对手户名", "对方户名", "对方姓名", "对手名称"]),
        counterparty_id_no: find_col(columns, &["对手证件号", "对手身份证号", "对方证件号码"]),
        counterparty_bank: find_col(
            columns,
            &["对手开户银行", "对手银行", "对方开户行", "对手开户网点"],
        ),
        summary: find_col(columns, &["摘要说明", "摘要", "交易摘要", "备注", "用途"]),
        channel: find_col(columns, &["现金标志", "交易渠道", "渠道", "转账方式"]),
        location: find_col(columns, &["交易发生地", "交易地点", "发生地", "地区"]),
        ip: find_col(columns, &["IP地址", "IP", "登录IP", "操作IP"]),
        mac: find_col(columns, &["MAC地址", "MAC", "设备MAC", "物理地址"]),
        currency: find_col(columns, &["交易币种", "币种"]),
        txn_serial_no: find_col(columns, &["交易流水号", "流水号", "交易序号"]),
        voucher_no: find_col(columns, &["凭证号", "传票号"]),
        is_success: find_col(
            columns,
            &["交易是否成功", "成功标志", "交易状态", "处理状态"],
        ),
    };

    let score = [
        mapping.account_no.is_some(),
        mapping.account_name.is_some(),
        mapping.txn_time.is_some(),
        mapping.direction.is_some()
            || (mapping.debit_amount.is_some() && mapping.credit_amount.is_some()),
        mapping.amount.is_some()
            || mapping.debit_amount.is_some()
            || mapping.credit_amount.is_some(),
        mapping.counterparty_account.is_some() || mapping.counterparty_name.is_some(),
    ]
    .into_iter()
    .filter(|v| *v)
    .count();

    TableMapping {
        raw_table: table.to_string(),
        table_type: if score >= 4 {
            "bank_transaction".into()
        } else {
            "unknown".into()
        },
        score,
        columns: mapping,
        direction_map: DirectionMap::default(),
    }
}

pub fn find_col(columns: &[String], aliases: &[&str]) -> Option<String> {
    let normalized_columns = columns
        .iter()
        .map(|col| (col, normalize_match_key(col)))
        .collect::<Vec<_>>();
    for alias in aliases {
        let normalized = normalize_match_key(alias);
        if let Some((found, _)) = normalized_columns
            .iter()
            .find(|(_, col)| col.as_str() == normalized)
        {
            return Some((*found).clone());
        }
    }
    for alias in aliases {
        let normalized = normalize_match_key(alias);
        if let Some((found, _)) = normalized_columns
            .iter()
            .find(|(_, col)| col.contains(&normalized))
        {
            return Some((*found).clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_bank_columns() {
        let cols = vec![
            "交易时间".into(),
            "交易卡号".into(),
            "交易户名".into(),
            "收付标志".into(),
            "交易金额".into(),
            "交易对手账卡号".into(),
            "对手户名".into(),
        ];
        let mapping = infer_bank_mapping("t", &cols);
        assert_eq!(mapping.table_type, "bank_transaction");
        assert_eq!(mapping.columns.txn_time.as_deref(), Some("交易时间"));
    }
}
