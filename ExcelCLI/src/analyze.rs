use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};

use crate::db::{quote_ident, table_exists};
use crate::models::{
    AmountBinStat, Concentration, CounterpartyStat, FundAnalysis, Overview, PeriodStat,
    RiskIndicator,
};

pub fn analyze_fund_command(db: &Path) -> Result<FundAnalysis> {
    let conn = Connection::open(db).with_context(|| format!("打开数据库失败: {}", db.display()))?;
    if !table_exists(&conn, "bank_transactions")? {
        bail!("缺少 bank_transactions，请先执行 normalize");
    }
    let overview = query_overview(&conn)?;
    let account_count = query_count_distinct(&conn, "account_no")?;
    let counterparty_count = query_count_distinct(&conn, "counterparty_account")?;
    let monthly_trend = query_monthly_trend(&conn)?;
    let amount_distribution = query_amount_distribution(&conn)?;
    let top_sources = query_counterparties(&conn, "in", 10)?;
    let top_destinations = query_counterparties(&conn, "out", 10)?;
    let concentration = Concentration {
        source_top1: concentration(&top_sources, overview.income_amount, 1),
        source_top3: concentration(&top_sources, overview.income_amount, 3),
        source_top5: concentration(&top_sources, overview.income_amount, 5),
        destination_top1: concentration(&top_destinations, overview.expense_amount, 1),
        destination_top3: concentration(&top_destinations, overview.expense_amount, 3),
        destination_top5: concentration(&top_destinations, overview.expense_amount, 5),
    };
    let suspicious_indicators = build_risks(&conn, &overview, &concentration)?;
    Ok(FundAnalysis {
        overview,
        account_count,
        counterparty_count,
        monthly_trend,
        amount_distribution,
        top_sources,
        top_destinations,
        concentration,
        suspicious_indicators,
    })
}

fn query_overview(conn: &Connection) -> Result<Overview> {
    Ok(conn.query_row(
        r#"
        SELECT
          COUNT(*),
          COALESCE(SUM(amount), 0),
          COALESCE(SUM(CASE WHEN direction='in' THEN 1 ELSE 0 END), 0),
          COALESCE(SUM(CASE WHEN direction='in' THEN amount ELSE 0 END), 0),
          COALESCE(SUM(CASE WHEN direction='out' THEN 1 ELSE 0 END), 0),
          COALESCE(SUM(CASE WHEN direction='out' THEN amount ELSE 0 END), 0),
          NULLIF(MIN(NULLIF(txn_time, '')), ''),
          NULLIF(MAX(NULLIF(txn_time, '')), '')
        FROM bank_transactions
        "#,
        [],
        |row| {
            let income_amount: f64 = row.get(3)?;
            let expense_amount: f64 = row.get(5)?;
            let total = income_amount + expense_amount;
            Ok(Overview {
                total_count: row.get::<_, i64>(0)? as usize,
                total_amount: row.get(1)?,
                income_count: row.get::<_, i64>(2)? as usize,
                income_amount,
                expense_count: row.get::<_, i64>(4)? as usize,
                expense_amount,
                first_time: row.get(6)?,
                last_time: row.get(7)?,
                balance_ratio: (total > 0.0)
                    .then_some((income_amount - expense_amount).abs() / total),
            })
        },
    )?)
}

fn query_count_distinct(conn: &Connection, column: &str) -> Result<usize> {
    let sql = format!(
        "SELECT COUNT(DISTINCT NULLIF({}, '')) FROM bank_transactions",
        quote_ident(column)
    );
    let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(count as usize)
}

fn query_monthly_trend(conn: &Connection) -> Result<Vec<PeriodStat>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT substr(txn_time, 1, 7) AS period, direction, COUNT(*), SUM(amount)
        FROM bank_transactions
        WHERE txn_time != ''
        GROUP BY period, direction
        ORDER BY period, direction
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PeriodStat {
            period: row.get(0)?,
            direction: row.get(1)?,
            count: row.get::<_, i64>(2)? as usize,
            amount: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn query_amount_distribution(conn: &Connection) -> Result<Vec<AmountBinStat>> {
    let bins = [
        ("0-1千", 0.0, 1000.0),
        ("1千-5千", 1000.0, 5000.0),
        ("5千-1万", 5000.0, 10000.0),
        ("1万-5万", 10000.0, 50000.0),
        ("5万-10万", 50000.0, 100000.0),
        ("10万-50万", 100000.0, 500000.0),
        ("50万以上", 500000.0, f64::INFINITY),
    ];
    let mut output = Vec::new();
    for (label, min, max) in bins {
        for direction in ["in", "out"] {
            let count: i64 = if max.is_infinite() {
                conn.query_row(
                    "SELECT COUNT(*) FROM bank_transactions WHERE direction=?1 AND amount>?2",
                    params![direction, min],
                    |row| row.get(0),
                )?
            } else {
                conn.query_row(
                    "SELECT COUNT(*) FROM bank_transactions WHERE direction=?1 AND amount>?2 AND amount<=?3",
                    params![direction, min, max],
                    |row| row.get(0),
                )?
            };
            output.push(AmountBinStat {
                bin: label.to_string(),
                direction: direction.to_string(),
                count: count as usize,
            });
        }
    }
    Ok(output)
}

fn query_counterparties(
    conn: &Connection,
    direction: &str,
    limit: usize,
) -> Result<Vec<CounterpartyStat>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
          COALESCE(NULLIF(counterparty_account, ''), '无对手账号') AS account,
          COALESCE(NULLIF(counterparty_name, ''), '未知') AS name,
          COUNT(*) AS count,
          SUM(amount) AS amount,
          AVG(amount) AS average
        FROM bank_transactions
        WHERE direction=?1
        GROUP BY account, name
        ORDER BY amount DESC
        LIMIT ?2
        "#,
    )?;
    let rows = stmt.query_map(params![direction, limit as i64], |row| {
        Ok(CounterpartyStat {
            account: row.get(0)?,
            name: row.get(1)?,
            count: row.get::<_, i64>(2)? as usize,
            amount: row.get(3)?,
            average: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn concentration(items: &[CounterpartyStat], total: f64, n: usize) -> Option<f64> {
    (total > 0.0).then_some(items.iter().take(n).map(|item| item.amount).sum::<f64>() / total)
}

fn build_risks(
    conn: &Connection,
    overview: &Overview,
    concentration: &Concentration,
) -> Result<Vec<RiskIndicator>> {
    let mut risks = Vec::new();
    if let Some(value) = overview.balance_ratio {
        risks.push(RiskIndicator {
            name: "收支平衡度".into(),
            value,
            level: risk_level(value, 0.05, 0.01, true),
            message: format!("收支差额占总收支比例为 {:.2}%", value * 100.0),
        });
    }

    let round_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bank_transactions WHERE CAST(amount AS INTEGER) % 10000 = 0 AND amount >= 10000",
        [],
        |row| row.get(0),
    )?;
    let round_ratio = if overview.total_count > 0 {
        round_count as f64 / overview.total_count as f64
    } else {
        0.0
    };
    risks.push(RiskIndicator {
        name: "整万交易占比".into(),
        value: round_ratio,
        level: risk_level(round_ratio, 0.30, 0.45, false),
        message: format!(
            "整万交易 {} 笔，占比 {:.2}%",
            round_count,
            round_ratio * 100.0
        ),
    });

    let keyword_count: i64 = conn.query_row(
        r#"
        SELECT COUNT(*) FROM bank_transactions
        WHERE summary LIKE '%换汇%' OR summary LIKE '%换钱%' OR summary LIKE '%换币%'
           OR summary LIKE '%兑换%' OR summary LIKE '%好处费%' OR summary LIKE '%返利%'
        "#,
        [],
        |row| row.get(0),
    )?;
    risks.push(RiskIndicator {
        name: "可疑摘要关键词".into(),
        value: keyword_count as f64,
        level: if keyword_count >= 10 {
            "高度异常"
        } else if keyword_count >= 1 {
            "异常"
        } else {
            "正常"
        }
        .into(),
        message: format!("命中可疑摘要关键词 {} 笔", keyword_count),
    });

    push_concentration_risk(&mut risks, "来源TOP3集中度", concentration.source_top3);
    push_concentration_risk(&mut risks, "去向TOP3集中度", concentration.destination_top3);
    Ok(risks)
}

fn risk_level(value: f64, abnormal: f64, severe: f64, lower_is_risky: bool) -> String {
    let level = if lower_is_risky {
        if value < severe {
            "高度异常"
        } else if value < abnormal {
            "异常"
        } else {
            "正常"
        }
    } else if value > severe {
        "高度异常"
    } else if value > abnormal {
        "异常"
    } else {
        "正常"
    };
    level.into()
}

fn push_concentration_risk(risks: &mut Vec<RiskIndicator>, name: &str, value: Option<f64>) {
    if let Some(value) = value {
        risks.push(RiskIndicator {
            name: name.into(),
            value,
            level: risk_level(value, 0.40, 0.60, false),
            message: format!("{name}为 {:.2}%", value * 100.0),
        });
    }
}

pub fn print_fund_text(analysis: &FundAnalysis) {
    println!("交易笔数: {}", analysis.overview.total_count);
    println!("交易总额: {:.2}", analysis.overview.total_amount);
    println!(
        "收入: {}笔 / {:.2}",
        analysis.overview.income_count, analysis.overview.income_amount
    );
    println!(
        "支出: {}笔 / {:.2}",
        analysis.overview.expense_count, analysis.overview.expense_amount
    );
    println!("主体账户数: {}", analysis.account_count);
    println!("对手账户数: {}", analysis.counterparty_count);
}
