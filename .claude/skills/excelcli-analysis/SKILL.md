---
name: excelcli-analysis
description: "使用 ExcelCLI 对经侦案件中的 Excel、XLS、XLSX、CSV 数据进行原生导入、字段识别、标准化、筛选和只读 SQL 分析。适用于银行流水、发票销方/购方、税务登记、工商信息等数据处理，是其他经侦技战法技能的数据底座。"
---

# ExcelCLI 数据底座

## 强制原则

涉及 `xls`、`xlsx`、`csv`、银行流水、发票、税务登记、工商信息等表格数据时，优先使用 `excelcli` 完成读取、入库、映射、标准化和基础统计。

不要自行编写 自写脚本或第三方表格库 入库或清洗脚本。除非 `excelcli` 明确无法读取文件，才说明限制并请求用户确认替代方案。

## 前置检查

```bash
excelcli --help
```

如果命令不可用，使用项目内二进制：

```bash
/Users/cnrstar/MegaVector/2026/经侦分析/ExcelCLI/target/release/excelcli --help
```

## 银行流水流程

```bash
excelcli import "./case-data" --case-id CASE001 --db "./output/analysis.db" --verify
excelcli inspect "./output/analysis.db" --json --sample 3
excelcli map "./output/analysis.db" --profile bank-transaction --out "./output/mapping.json" --json
excelcli normalize "./output/analysis.db" --mapping "./output/mapping.json"
excelcli analyze fund "./output/analysis.db" --json
```

标准化后优先使用：

- `bank_transactions`：统一银行交易明细表。
- `transactions`：按账户、对手、方向、金额、时间、关键词筛选交易。
- `query`：对 `bank_transactions` 执行只读 SQL 统计。

## 涉税数据流程

```bash
excelcli import "./tax-case-data" --case-id TAX001 --db "./output/analysis.db" --verify
excelcli inspect "./output/analysis.db" --json --sample 3
excelcli tax map "./output/analysis.db" --out "./output/tax-mapping.json" --json
excelcli tax normalize "./output/analysis.db" --mapping "./output/tax-mapping.json"
excelcli tax analyze "./output/analysis.db" --json
```

标准化后优先使用：

- `tax_invoices`：统一发票明细表，含 `taxpayer_name`、`counterparty_name`、票种、角色、金额、税额、货品、来源文件和原始行号。
- `seller_invoice`、`buyer_invoice`：销方/购方视图。
- `tax_registrations`、`tax_business_registrations`：税务登记和工商信息标准表。
- `tax invoices`：按纳税人、对手、票种、角色、金额、日期、关键词筛选发票。

## 常用命令

```bash
excelcli transactions "./output/analysis.db" --account "6222..." --limit 100 --json
excelcli transactions "./output/analysis.db" --direction out --min-amount 100000 --json
excelcli transactions "./output/analysis.db" --counterparty "张三" --json
```

```bash
excelcli tax invoices "./output/analysis.db" --taxpayer "涉案公司" --limit 100 --json
excelcli tax invoices "./output/analysis.db" --invoice-type "专票" --min-amount 100000 --json
excelcli tax invoices "./output/analysis.db" --role seller --keyword "作废" --json
```

## 自定义 SQL

只使用 `SELECT`、`WITH`、`PRAGMA`。禁止写操作。

```bash
excelcli query "./output/analysis.db" --sql "SELECT direction, COUNT(*) AS count, SUM(amount) AS amount FROM bank_transactions GROUP BY direction" --json
```

```bash
excelcli query "./output/analysis.db" --sql "SELECT invoice_role, invoice_type, COUNT(*) AS count, SUM(total_amount) AS total_amount, SUM(tax_amount) AS tax_amount FROM tax_invoices GROUP BY invoice_role, invoice_type" --json
```

复杂查询可放入 SQL 文件：

```bash
excelcli query "./output/analysis.db" --file "./queries/check.sql" --json
```

## 结果使用

报告和研判必须引用 `excelcli` 的结构化结果，并保留来源回溯字段：`source_file`、`sheet_name`、`row_no`、`raw_table`。

如 `normalize` 或 `tax normalize` 输出 `issues > 0`，先查询 `import_issues`，说明字段缺失、金额解析、方向识别或发票标准化问题，不要直接下确定结论。

## 多文件案件

同一案件有多个银行、多个主体、多个 Sheet 或多个发票文件时，直接对目录执行 `import`。不要手工拼表；统一以标准表分析，再用来源字段回溯证据。
