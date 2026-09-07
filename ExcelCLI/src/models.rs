use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ImportSummary {
    pub case_id: String,
    pub db: String,
    pub batch_id: String,
    pub imported_tables: Vec<ImportedTable>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedTable {
    pub table_name: String,
    pub source_file: String,
    pub sheet_name: String,
    pub rows: usize,
    pub columns: usize,
    pub verified: bool,
}

#[derive(Debug, Serialize)]
pub struct InspectOutput {
    pub tables: Vec<TableInfo>,
}

#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub table_name: String,
    pub source_file: Option<String>,
    pub sheet_name: Option<String>,
    pub rows: usize,
    pub columns: Vec<String>,
    pub samples: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingFile {
    pub profile: String,
    pub tables: Vec<TableMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMapping {
    pub raw_table: String,
    pub table_type: String,
    pub score: usize,
    pub columns: BankColumnMapping,
    #[serde(default)]
    pub direction_map: DirectionMap,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BankColumnMapping {
    pub account_no: Option<String>,
    pub account_name: Option<String>,
    pub account_id_no: Option<String>,
    pub txn_time: Option<String>,
    pub direction: Option<String>,
    pub amount: Option<String>,
    pub debit_amount: Option<String>,
    pub credit_amount: Option<String>,
    pub balance: Option<String>,
    pub counterparty_account: Option<String>,
    pub counterparty_name: Option<String>,
    pub counterparty_id_no: Option<String>,
    pub counterparty_bank: Option<String>,
    pub summary: Option<String>,
    pub channel: Option<String>,
    pub location: Option<String>,
    pub ip: Option<String>,
    pub mac: Option<String>,
    pub currency: Option<String>,
    pub txn_serial_no: Option<String>,
    pub voucher_no: Option<String>,
    pub is_success: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionMap {
    pub r#in: Vec<String>,
    pub out: Vec<String>,
    pub fail: Vec<String>,
}

impl Default for DirectionMap {
    fn default() -> Self {
        Self {
            r#in: [
                "进", "入", "收入", "收", "贷", "贷方", "C", "CR", "转入", "收款",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            out: [
                "出", "支出", "付", "借", "借方", "D", "DR", "转出", "付款", "取",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            fail: ["0", "失败", "交易失败", "FAIL", "F", "N", "否", "false"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NormalizeSummary {
    pub inserted: usize,
    pub duplicates: usize,
    pub issues: usize,
}

#[derive(Debug)]
pub struct BankTransaction {
    pub case_id: String,
    pub source_file: String,
    pub sheet_name: String,
    pub row_no: i64,
    pub bank_name: String,
    pub account_no: String,
    pub account_name: String,
    pub account_id_no: String,
    pub account_type: String,
    pub txn_time: String,
    pub direction: String,
    pub amount: f64,
    pub balance: Option<f64>,
    pub counterparty_account: String,
    pub counterparty_name: String,
    pub counterparty_id_no: String,
    pub counterparty_bank: String,
    pub summary: String,
    pub channel: String,
    pub location: String,
    pub ip: String,
    pub mac: String,
    pub currency: String,
    pub txn_serial_no: String,
    pub voucher_no: String,
    pub is_success: bool,
    pub raw_table: String,
    pub raw_row_id: i64,
    pub dedup_key: String,
}

#[derive(Debug, Serialize)]
pub struct FundAnalysis {
    pub overview: Overview,
    pub account_count: usize,
    pub counterparty_count: usize,
    pub monthly_trend: Vec<PeriodStat>,
    pub amount_distribution: Vec<AmountBinStat>,
    pub top_sources: Vec<CounterpartyStat>,
    pub top_destinations: Vec<CounterpartyStat>,
    pub concentration: Concentration,
    pub suspicious_indicators: Vec<RiskIndicator>,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    pub total_count: usize,
    pub total_amount: f64,
    pub income_count: usize,
    pub income_amount: f64,
    pub expense_count: usize,
    pub expense_amount: f64,
    pub first_time: Option<String>,
    pub last_time: Option<String>,
    pub balance_ratio: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PeriodStat {
    pub period: String,
    pub direction: String,
    pub count: usize,
    pub amount: f64,
}

#[derive(Debug, Serialize)]
pub struct AmountBinStat {
    pub bin: String,
    pub direction: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct CounterpartyStat {
    pub account: String,
    pub name: String,
    pub count: usize,
    pub amount: f64,
    pub average: f64,
}

#[derive(Debug, Serialize)]
pub struct Concentration {
    pub source_top1: Option<f64>,
    pub source_top3: Option<f64>,
    pub source_top5: Option<f64>,
    pub destination_top1: Option<f64>,
    pub destination_top3: Option<f64>,
    pub destination_top5: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct RiskIndicator {
    pub name: String,
    pub value: f64,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxMappingFile {
    pub profile: String,
    pub tables: Vec<TaxTableMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxTableMapping {
    pub raw_table: String,
    pub table_type: String,
    pub score: usize,
    pub columns: TaxColumnMapping,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaxColumnMapping {
    pub invoice_code: Option<String>,
    pub invoice_no: Option<String>,
    pub invoice_date: Option<String>,
    pub invoice_type: Option<String>,
    pub invoice_status: Option<String>,
    pub buyer_tax_id: Option<String>,
    pub buyer_name: Option<String>,
    pub buyer_region: Option<String>,
    pub seller_tax_id: Option<String>,
    pub seller_name: Option<String>,
    pub seller_region: Option<String>,
    pub goods_name: Option<String>,
    pub goods_code: Option<String>,
    pub amount: Option<String>,
    pub tax_rate: Option<String>,
    pub tax_amount: Option<String>,
    pub total_amount: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub certify_date: Option<String>,
    pub ip: Option<String>,
    pub mac: Option<String>,
    pub taxpayer_id: Option<String>,
    pub taxpayer_name: Option<String>,
    pub taxpayer_status: Option<String>,
    pub register_date: Option<String>,
    pub industry: Option<String>,
    pub register_type: Option<String>,
    pub address: Option<String>,
    pub legal_person: Option<String>,
    pub legal_person_id: Option<String>,
    pub finance_person: Option<String>,
    pub finance_person_id: Option<String>,
    pub tax_staff: Option<String>,
    pub tax_staff_id: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub business_scope: Option<String>,
    pub registered_capital: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaxNormalizeSummary {
    pub invoices: usize,
    pub tax_registrations: usize,
    pub business_registrations: usize,
    pub skipped_tables: usize,
    pub issues: usize,
}

#[derive(Debug, Default)]
pub struct TaxInvoiceFilter {
    pub role: Option<String>,
    pub taxpayer: Option<String>,
    pub counterparty: Option<String>,
    pub invoice_type: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub keyword: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct TaxAnalysis {
    pub overview: TaxOverview,
    pub by_role_type: Vec<TaxRoleTypeStat>,
    pub monthly_trend: Vec<TaxMonthlyStat>,
    pub top_buyers: Vec<TaxCounterpartyStat>,
    pub top_sellers: Vec<TaxCounterpartyStat>,
    pub risk_indicators: Vec<RiskIndicator>,
}

#[derive(Debug, Serialize)]
pub struct TaxOverview {
    pub invoice_count: usize,
    pub invoice_amount: f64,
    pub tax_amount: f64,
    pub total_amount: f64,
    pub seller_count: usize,
    pub buyer_count: usize,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaxRoleTypeStat {
    pub invoice_role: String,
    pub invoice_type: String,
    pub count: usize,
    pub amount: f64,
    pub tax_amount: f64,
    pub total_amount: f64,
}

#[derive(Debug, Serialize)]
pub struct TaxMonthlyStat {
    pub month: String,
    pub invoice_role: String,
    pub count: usize,
    pub amount: f64,
    pub tax_amount: f64,
    pub total_amount: f64,
}

#[derive(Debug, Serialize)]
pub struct TaxCounterpartyStat {
    pub tax_id: String,
    pub name: String,
    pub count: usize,
    pub total_amount: f64,
    pub tax_amount: f64,
}
