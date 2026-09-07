use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::analyze::{analyze_fund_command, print_fund_text};
use crate::import::import_command;
use crate::inspect::{inspect_command, print_inspect_text};
use crate::mapping::map_command;
use crate::normalize::normalize_command;
use crate::query::{TransactionFilter, print_csv, query_command, transactions_command};
use crate::tax::{
    tax_analyze_command, tax_invoices_command, tax_map_command, tax_normalize_command,
};

#[derive(Parser)]
#[command(name = "excelcli")]
#[command(about = "经侦 Excel/CSV 原生分析 CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 原样导入 xls/xlsx/csv 到 SQLite 原始层
    Import {
        input: PathBuf,
        #[arg(long)]
        case_id: String,
        #[arg(long)]
        db: PathBuf,
        #[arg(long, default_value_t = false)]
        verify: bool,
        #[arg(long)]
        encoding: Option<String>,
    },
    /// 查看数据库表结构和样例
    Inspect {
        db: PathBuf,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long, default_value_t = 3)]
        sample: usize,
    },
    /// 根据列名生成银行流水字段映射
    Map {
        db: PathBuf,
        #[arg(long, default_value = "bank-transaction")]
        profile: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 将原始表标准化为 bank_transactions
    Normalize {
        db: PathBuf,
        #[arg(long)]
        mapping: PathBuf,
    },
    /// 执行分析
    Analyze {
        #[command(subcommand)]
        command: AnalyzeCommands,
    },
    /// 执行只读 SQL 查询，适合智能体自定义分析
    Query {
        db: PathBuf,
        #[arg(long)]
        sql: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long, default_value_t = false)]
        csv: bool,
    },
    /// 高频交易流水筛选：卡号、对手、金额、时间、方向、关键词
    Transactions {
        db: PathBuf,
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        counterparty: Option<String>,
        #[arg(long)]
        direction: Option<String>,
        #[arg(long)]
        min_amount: Option<f64>,
        #[arg(long)]
        max_amount: Option<f64>,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        keyword: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long, default_value_t = false)]
        csv: bool,
    },
    /// 涉税犯罪分析：表识别、标准化、发票筛选和风险分析
    Tax {
        #[command(subcommand)]
        command: TaxCommands,
    },
}

#[derive(Subcommand)]
enum AnalyzeCommands {
    /// 基础资金分析
    Fund {
        db: PathBuf,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum TaxCommands {
    /// 自动识别发票、税务登记、工商信息等涉税表并生成映射
    Map {
        db: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 根据涉税映射生成标准表和标准视图
    Normalize {
        db: PathBuf,
        #[arg(long)]
        mapping: PathBuf,
    },
    /// 涉税风险概览分析，支持按纳税人和日期范围过滤
    Analyze {
        db: PathBuf,
        #[arg(long)]
        taxpayer: Option<String>,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long, default_value_t = 10)]
        top: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 发票明细筛选：纳税人、对手、票种、金额、日期、关键词
    Invoices {
        db: PathBuf,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        taxpayer: Option<String>,
        #[arg(long)]
        counterparty: Option<String>,
        #[arg(long)]
        invoice_type: Option<String>,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        min_amount: Option<f64>,
        #[arg(long)]
        max_amount: Option<f64>,
        #[arg(long)]
        keyword: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long, default_value_t = false)]
        csv: bool,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Import {
            input,
            case_id,
            db,
            verify,
            encoding,
        } => print_json(&import_command(
            &input,
            &case_id,
            &db,
            verify,
            encoding.as_deref(),
        )?),
        Commands::Inspect { db, json, sample } => {
            let output = inspect_command(&db, sample)?;
            if json {
                print_json(&output)
            } else {
                print_inspect_text(&output);
                Ok(())
            }
        }
        Commands::Map {
            db,
            profile,
            out,
            json,
        } => {
            let mapping = map_command(&db, &profile)?;
            if let Some(path) = &out {
                fs::write(path, serde_json::to_string_pretty(&mapping)?)
                    .with_context(|| format!("写入映射文件失败: {}", path.display()))?;
            }
            if json || out.is_none() {
                print_json(&mapping)
            } else {
                Ok(())
            }
        }
        Commands::Normalize { db, mapping } => print_json(&normalize_command(&db, &mapping)?),
        Commands::Analyze { command } => match command {
            AnalyzeCommands::Fund { db, json } => {
                let analysis = analyze_fund_command(&db)?;
                if json {
                    print_json(&analysis)
                } else {
                    print_fund_text(&analysis);
                    Ok(())
                }
            }
        },
        Commands::Query {
            db,
            sql,
            file,
            limit,
            json,
            csv,
        } => {
            let output = query_command(&db, sql.as_deref(), file.as_deref(), limit)?;
            print_query_output(&output, json, csv)
        }
        Commands::Transactions {
            db,
            account,
            counterparty,
            direction,
            min_amount,
            max_amount,
            start,
            end,
            keyword,
            limit,
            json,
            csv,
        } => {
            let output = transactions_command(
                &db,
                TransactionFilter {
                    account,
                    counterparty,
                    direction,
                    min_amount,
                    max_amount,
                    start,
                    end,
                    keyword,
                    limit,
                },
            )?;
            print_query_output(&output, json, csv)
        }
        Commands::Tax { command } => match command {
            TaxCommands::Map { db, out, json } => {
                let mapping = tax_map_command(&db)?;
                if let Some(path) = &out {
                    fs::write(path, serde_json::to_string_pretty(&mapping)?)
                        .with_context(|| format!("写入涉税映射文件失败: {}", path.display()))?;
                }
                if json || out.is_none() {
                    print_json(&mapping)
                } else {
                    Ok(())
                }
            }
            TaxCommands::Normalize { db, mapping } => {
                print_json(&tax_normalize_command(&db, &mapping)?)
            }
            TaxCommands::Analyze {
                db,
                taxpayer,
                start,
                end,
                top,
                json,
            } => {
                let analysis = tax_analyze_command(
                    &db,
                    taxpayer.as_deref(),
                    start.as_deref(),
                    end.as_deref(),
                    top,
                )?;
                if json {
                    print_json(&analysis)
                } else {
                    print_json(&analysis)
                }
            }
            TaxCommands::Invoices {
                db,
                role,
                taxpayer,
                counterparty,
                invoice_type,
                start,
                end,
                min_amount,
                max_amount,
                keyword,
                limit,
                json,
                csv,
            } => {
                let output = tax_invoices_command(
                    &db,
                    crate::models::TaxInvoiceFilter {
                        role,
                        taxpayer,
                        counterparty,
                        invoice_type,
                        start,
                        end,
                        min_amount,
                        max_amount,
                        keyword,
                        limit,
                    },
                )?;
                print_query_output(&output, json, csv)
            }
        },
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_query_output<T: Serialize>(value: &T, json: bool, csv: bool) -> Result<()>
where
    T: AsQueryOutput,
{
    if csv {
        print_csv(value.as_query_output())
    } else if json || !csv {
        print_json(value)
    } else {
        Ok(())
    }
}

trait AsQueryOutput {
    fn as_query_output(&self) -> &crate::query::QueryOutput;
}

impl AsQueryOutput for crate::query::QueryOutput {
    fn as_query_output(&self) -> &crate::query::QueryOutput {
        self
    }
}
