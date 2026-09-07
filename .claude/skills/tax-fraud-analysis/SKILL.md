---
name: tax-fraud-analysis
description: "基于税务登记、发票销方/购方数据、银行交易流水和工商信息进行涉税犯罪分析，适用于虚开专票、虚开普票、骗取出口退税、变票虚开、对开环开等场景。数据处理必须使用 excelcli，本技能保留原始涉税技战法、指标和报告要求。"
---

# 涉税犯罪数据分析

## ⛔ 绝对禁止事项（违反将导致严重浪费时间）

> **禁止使用 Python、pandas、pip、venv 等任何 Python 相关操作。** 运行环境中 Python 依赖不全，尝试安装 pandas/venv 会连续失败，浪费 5+ 分钟。
> **所有数据操作只能用 `excelcli` 命令完成。** 这是唯一的正确方式。如果 excelcli 某个子命令执行失败，查看 `--help` 调整参数，绝不回退到 Python。

## excelcli 工具使用指南（强制）

本技能的所有数据操作必须通过 `excelcli` 完成。**禁止** 智能体自行用 Python/pandas 读取 Excel、自建入库流程、pip install、python3 -c 等任何 Python 操作。

### excelcli 工作流总览

excelcli 采用 **导入 → 检查 → 涉税映射 → 标准化 → 分析** 的五步工作流：

```
Excel/CSV 文件  →  import        →  SQLite 数据库
                      ↓
                  inspect       →  确认表结构和列名
                      ↓
                  tax map       →  涉税字段映射 JSON
                      ↓
                  tax normalize →  涉税标准表/视图
                      ↓
         tax analyze / tax invoices / query  →  分析结果
```

如同时有银行流水数据，还需执行银行流水标准化子工作流：

```
银行流水数据  →  map        →  银行字段映射 JSON
                  ↓
              normalize   →  bank_transactions 标准表
                  ↓
     analyze fund / transactions / query  →  资金分析结果
```

### 命令速查表

| 命令 | 用途 | 必填参数 |
|---|---|---|
| `excelcli import <INPUT> --case-id <ID> --db <DB>` | 导入 Excel/CSV 到 SQLite | INPUT, --case-id, --db |
| `excelcli inspect <DB>` | 查看表结构和样例数据 | DB |
| `excelcli tax map <DB>` | 涉税表自动识别与映射 | DB |
| `excelcli tax normalize <DB> --mapping <FILE>` | 涉税表标准化 | DB, --mapping |
| `excelcli tax analyze <DB>` | 涉税风险概览分析 | DB |
| `excelcli tax invoices <DB>` | 发票明细筛选 | DB |
| `excelcli map <DB>` | 生成银行流水字段映射 | DB |
| `excelcli normalize <DB> --mapping <FILE>` | 标准化为 bank_transactions 表 | DB, --mapping |
| `excelcli analyze fund <DB>` | 基础资金分析 | DB |
| `excelcli transactions <DB>` | 交易流水多维筛选 | DB |
| `excelcli query <DB> --sql <SQL>` | 执行自定义 SQL 查询 | DB, --sql 或 --file |

### 涉税分析完整步骤（按顺序执行）

> **核心原则：用最少次数的工具调用完成任务。** 每个步骤尽量用 `&&` 串联多条命令在一次 bash 调用中完成。SQL 查询用 `--file` 批量执行，不要逐条调用 `excelcli query`。

**Step 1 — 导入 + 检查 + 映射 + 标准化（一次 bash 调用完成）**

将多个 Excel 文件逐个 import 到同一个数据库，然后依次执行 inspect、tax map、tax normalize。以下命令用 `&&` 串联，**一次 bash 调用全部完成**：

```bash
cd /数据目录 && \
excelcli import 专票销方.xlsx --case-id tax001 --db tax001.db && \
excelcli import 专票购方.xlsx --case-id tax001 --db tax001.db && \
excelcli import 普票购方.xlsx --case-id tax001 --db tax001.db && \
excelcli import 纳税人信息.xlsx --case-id tax001 --db tax001.db && \
excelcli inspect tax001.db && \
excelcli tax map tax001.db --out tax_mapping.json && \
excelcli tax normalize tax001.db --mapping tax_mapping.json
```

**命令格式要点（不要用其他格式）**：
- `excelcli import <文件路径> --case-id <案件编号> --db <数据库路径>` — `--db` 是 `--flag` 形式，不是位置参数
- `excelcli inspect <数据库路径>` — 数据库路径是位置参数，不用 `--db`
- `excelcli tax map <数据库路径> --out <映射文件>` — 数据库路径是位置参数；`--out` 指定输出映射 JSON 文件
- `excelcli tax normalize <数据库路径> --mapping <映射文件>` — 两个参数都是位置参数形式
- 如果 import 报错，检查文件路径是否含中文，确保路径正确；不要回退到 Python

**标准化后产出的表/视图**（后续分析全部基于这些表）：
- `tax_invoices` — 统一发票表
- `seller_invoice` — 销方发票视图
- `buyer_invoice` — 购方发票视图
- `tax_registrations` — 税务登记标准表
- `business_registration` — 工商信息标准表（如有）

**Step 2 — 银行流水标准化（如有银行数据，一次调用完成）**

```bash
cd /数据目录 && \
excelcli import 银行流水.xlsx --case-id tax001 --db tax001.db && \
excelcli map tax001.db --out bank_mapping.json && \
excelcli normalize tax001.db --mapping bank_mapping.json
```

**Step 3 — 涉税风险概览 + 发票筛选（一次调用完成）**

```bash
cd /数据目录 && excelcli tax analyze tax001.db && excelcli tax invoices tax001.db --csv
```

- `tax analyze` 输出基础风险分析摘要
- `tax invoices` 输出完整发票明细（加 `--csv` 导出）

**Step 4 — 自定义 SQL 查询（批量执行，核心分析步骤）**

**关键策略**：不要逐条执行 SQL，而是将每个阶段的所有查询写入一个 `.sql` 文件，用 `excelcli query <DB> --file <FILE>` 一次性执行。每个阶段只需 **1 次** 工具调用。

```bash
# 示例：将阶段A的所有查询写入文件并批量执行
cat > /tmp/phase_a.sql << 'SQLEOF'
SELECT 'A1_涉案企业' as tag, taxpayer_id, taxpayer_name, taxpayer_status, registration_type, industry_type, legal_person_name, registration_date FROM tax_registrations;
SELECT 'A2_发票类型分布' as tag, invoice_type, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax FROM tax_invoices GROUP BY invoice_type;
-- ... 更多查询见下方"完整分析所需的 SQL 查询集"
SQLEOF
excelcli query tax001.db --file /tmp/phase_a.sql --json --limit 200
```

- `--sql` 和 `--file` 二选一，**优先用 `--file` 批量执行**
- `--limit`：返回行数上限（默认 100），重要：**始终设置合理的 --limit**，避免返回过多数据撑爆 prompt 上下文
- **每个阶段写一个 .sql 文件，一次 bash 调用完成**：`cat > /tmp/phase_X.sql << 'EOF' ... EOF && excelcli query <DB> --file /tmp/phase_X.sql --json --limit 200`

### 涉税标准表列名（必须掌握）

`tax normalize` 完成后，原始中文列名会被映射为以下**英文标准列名**。后续所有 `excelcli query --sql` 查询必须使用这些列名，**不要使用原始中文列名**，否则会报 `no such column` 错误。

#### tax_invoices / seller_invoice / buyer_invoice

| 标准列名 | 含义 | 备注 |
| --- | --- | --- |
| `id` | 自增主键 | INTEGER PK |
| `case_id` | 案件编号 | TEXT |
| `source_file` | 来源文件 | TEXT |
| `sheet_name` | 工作表名 | TEXT |
| `row_no` | 原始行号 | INTEGER |
| `invoice_code` | 发票代码 | TEXT |
| `invoice_number` | 发票号码 | TEXT |
| `invoice_type` | 发票类型 | TEXT, 如"增值税专用发票"/"增值税普通发票" |
| `invoice_date` | 开票日期 | TEXT, 格式 YYYY-MM-DD |
| `invoice_status` | 发票状态 | TEXT, 如"正常"/"作废"/"红冲" |
| `void_flag` | 作废标志 | TEXT, 如"是"/"否" |
| `seller_tax_id` | 销方纳税人识别号 | TEXT |
| `seller_name` | 销方名称 | TEXT |
| `seller_tax_authority` | 销方税务机关 | TEXT, 可能为空 |
| `buyer_tax_id` | 购方纳税人识别号 | TEXT |
| `buyer_name` | 购方名称 | TEXT |
| `buyer_tax_authority` | 购方税务机关 | TEXT, 可能为空 |
| `goods_name` | 货品名称 | TEXT |
| `goods_code` | 商品编码 | TEXT, 可能为空 |
| `quantity` | 数量 | REAL |
| `unit` | 单位 | TEXT, 可能为空 |
| `unit_price` | 单价 | REAL |
| `amount` | 货物金额（不含税） | REAL |
| `tax_rate` | 税率 | REAL, 如 0.13 表示 13% |
| `tax_amount` | 税额 | REAL |
| `total_amount` | 价税合计 | REAL |
| `is_deducted` | 是否认证抵扣 | INTEGER, 1=已认证 |
| `certify_date` | 认证日期 | TEXT, 可能为空 |
| `is_remote` | 异地发票标志 | TEXT, 可能为空 |
| `ip` | 开票IP地址 | TEXT, 可能为空 |
| `mac` | 开票MAC地址 | TEXT, 可能为空 |
| `device_serial` | 设备序列号 | TEXT, 可能为空 |
| `remark` | 备注 | TEXT, 可能为空 |
| `raw_table` | 原始表名 | TEXT |
| `raw_row_id` | 原始行ID | INTEGER |
| `dedup_key` | 去重键 | TEXT |

#### tax_registrations

| 标准列名 | 含义 | 备注 |
| --- | --- | --- |
| `id` | 自增主键 | INTEGER PK |
| `case_id` | 案件编号 | TEXT |
| `source_file` | 来源文件 | TEXT |
| `taxpayer_id` | 纳税人识别号 | TEXT |
| `taxpayer_name` | 纳税人名称 | TEXT |
| `credit_code` | 统一社会信用代码 | TEXT |
| `taxpayer_status` | 纳税人状态 | TEXT, 如"正常"/"非正常"/"注销" |
| `registration_type` | 登记注册类型 | TEXT |
| `industry_type` | 行业种类 | TEXT |
| `subject_type` | 课征主题类型名称 | TEXT, 可能为空 |
| `legal_person_name` | 法定代表人姓名 | TEXT |
| `legal_person_id` | 法定代表人身份证号码 | TEXT |
| `legal_person_phone` | 法定代表人电话 | TEXT, 可能为空 |
| `finance_officer_name` | 财务负责人姓名 | TEXT, 可能为空 |
| `finance_officer_id` | 财务负责人身份证件号码 | TEXT, 可能为空 |
| `finance_officer_phone` | 财务负责人电话 | TEXT, 可能为空 |
| `tax_handler_name` | 办税人姓名 | TEXT, 可能为空 |
| `tax_handler_id` | 办税人身份证件号码 | TEXT, 可能为空 |
| `tax_handler_phone` | 办税人电话 | TEXT, 可能为空 |
| `registration_date` | 登记日期 | TEXT |
| `business_start_date` | 开业设立日期 | TEXT |
| `tax_authority` | 主管税务局 | TEXT |
| `registered_address` | 注册地址 | TEXT, 可能为空 |
| `registered_address_code` | 注册地址行政区划代码 | TEXT, 可能为空 |
| `business_address` | 生产经营地址 | TEXT, 可能为空 |
| `business_address_code` | 生产经营地址行政区划代码 | TEXT, 可能为空 |
| `raw_table` | 原始表名 | TEXT |
| `raw_row_id` | 原始行ID | INTEGER |

#### business_registration（工商信息，如有）

| 标准列名 | 含义 | 备注 |
| --- | --- | --- |
| `id` | 自增主键 | INTEGER PK |
| `case_id` | 案件编号 | TEXT |
| `company_name` | 企业名称 | TEXT |
| `credit_code` | 统一社会信用代码 | TEXT |
| `registration_status` | 登记状态 | TEXT |
| `registered_capital` | 注册资本 | TEXT, 可能为空 |
| `paid_in_capital` | 实缴资本 | TEXT, 可能为空 |
| `business_scope` | 经营范围 | TEXT, 可能为空 |
| `legal_person_name` | 法定代表人 | TEXT |
| `company_type` | 企业(机构)类型 | TEXT, 可能为空 |
| `province` | 所属省份 | TEXT, 可能为空 |
| `city` | 所属城市 | TEXT, 可能为空 |
| `district` | 所属区县 | TEXT, 可能为空 |
| `address` | 企业地址 | TEXT, 可能为空 |
| `phone` | 电话 | TEXT, 可能为空 |
| `email` | 邮箱 | TEXT, 可能为空 |
| `industry_category` | 国标行业门类/大类 | TEXT, 可能为空 |
| `registration_authority` | 登记机关 | TEXT, 可能为空 |
| `raw_table` | 原始表名 | TEXT |
| `raw_row_id` | 原始行ID | INTEGER |

**关键易错点**：
- 发票类型列名是 `invoice_type`，值区分"增值税专用发票"和"增值税普通发票"
- 开票日期列名是 `invoice_date`，格式为 YYYY-MM-DD
- 金额列名是 `amount`（不含税金额），价税合计是 `total_amount`
- 作废标志是 `void_flag` 或 `invoice_status`，需确认具体取值
- 销方识别号是 `seller_tax_id`，购方识别号是 `buyer_tax_id`
- `seller_invoice` 和 `buyer_invoice` 是 `tax_invoices` 的视图，列名相同
- `amount` 为 REAL 类型，可以直接用于数值计算和聚合

**normalize 后的必备步骤**：执行 `PRAGMA table_info` 确认实际列名，避免 SQL 报错：

```bash
excelcli query <DB> --sql "PRAGMA table_info(tax_invoices)" --json
excelcli query <DB> --sql "PRAGMA table_info(tax_registrations)" --json
excelcli query <DB> --sql "PRAGMA table_info(business_registration)" --json
```

### 完整分析所需的批量 SQL 文件（每个阶段一次调用）

以下 SQL 按分析阶段组织为独立的 `.sql` 文件。**每个阶段用一次 `cat > /tmp/phase_X.sql << 'SQLEOF' ... SQLEOF && excelcli query <DB> --file /tmp/phase_X.sql --json --limit 200` 完成**。注意：所有查询均使用上方标准列名。

> **执行约束**：
> - 始终加 `--limit 200`（或更小），避免返回过多数据导致 prompt 上下文膨胀
> - 不要逐条执行 `excelcli query --sql`，必须用 `--file` 批量执行
> - 整个分析过程的工具调用次数不应超过 **15 次**

#### 阶段A：数据探查与企业画像（phase_a.sql，1 次调用）

```sql
-- A1. 涉案企业列表
SELECT 'A1_企业' as tag, taxpayer_id, taxpayer_name, taxpayer_status, registration_type, industry_type, legal_person_name, registration_date, business_start_date FROM tax_registrations ORDER BY taxpayer_name;

-- A2. 发票类型分布
SELECT 'A2_票型分布' as tag, invoice_type, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax, ROUND(SUM(total_amount),2) as total_with_tax FROM tax_invoices GROUP BY invoice_type;

-- A3. 销方发票概况（按专票/普票+状态）
SELECT 'A3_销方概况' as tag, invoice_type, invoice_status, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax, ROUND(SUM(total_amount),2) as total_with_tax FROM seller_invoice GROUP BY invoice_type, invoice_status ORDER BY invoice_type, invoice_status;

-- A4. 购方发票概况
SELECT 'A4_购方概况' as tag, invoice_type, invoice_status, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax, ROUND(SUM(total_amount),2) as total_with_tax FROM buyer_invoice GROUP BY invoice_type, invoice_status ORDER BY invoice_type, invoice_status;

-- A5. 进销对比
SELECT 'A5_进销对比' as tag, direction, cnt, total_amount, total_tax FROM (SELECT '销项' as direction, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax FROM seller_invoice WHERE invoice_status != '作废' UNION ALL SELECT '进项' as direction, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax FROM buyer_invoice WHERE invoice_status != '作废');

-- A6. 税率分布
SELECT 'A6_税率' as tag, tax_rate, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount FROM tax_invoices GROUP BY tax_rate ORDER BY tax_rate;

-- A7. 货品名称TOP20
SELECT 'A7_货品TOP20' as tag, goods_name, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(total_amount),2) as total_with_tax FROM tax_invoices GROUP BY goods_name ORDER BY total_with_tax DESC LIMIT 20;

-- A8. 开票月度趋势
SELECT 'A8_月度趋势' as tag, substr(invoice_date,1,7) as month, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(total_amount),2) as total_with_tax FROM tax_invoices WHERE invoice_status != '作废' GROUP BY month ORDER BY month;

-- A9. 认证情况统计
SELECT 'A9_认证' as tag, CASE WHEN is_deducted = 1 THEN '已认证' ELSE '未认证' END as certify_status, COUNT(*) as cnt, ROUND(SUM(total_amount),2) as total_with_tax FROM tax_invoices GROUP BY is_deducted;
```

执行方式：
```bash
cat > /tmp/phase_a.sql << 'SQLEOF'
（粘贴上方全部 SQL）
SQLEOF
excelcli query tax001.db --file /tmp/phase_a.sql --json --limit 200
```

#### 阶段B：发票异常特征筛查（phase_b.sql，1 次调用）

```sql
-- B1. 进销品名匹配（R01）
SELECT 'B1_进销品名' as tag, s.goods_name as out_goods, b.goods_name as in_goods, s.total as out_total, b.total as in_total, CASE WHEN s.goods_name = b.goods_name THEN '匹配' ELSE '不匹配' END as match_flag FROM (SELECT goods_name, ROUND(SUM(amount),2) as total FROM seller_invoice WHERE invoice_status != '作废' GROUP BY goods_name) s JOIN (SELECT goods_name, ROUND(SUM(amount),2) as total FROM buyer_invoice WHERE invoice_status != '作废' GROUP BY goods_name) b ON 1=1 ORDER BY (s.total + b.total) DESC LIMIT 30;

-- B2. 整额开票检测（R02）
SELECT 'B2_整额' as tag, COUNT(*) as round_cnt, ROUND(SUM(total_amount),2) as round_total, (SELECT COUNT(*) FROM tax_invoices WHERE invoice_status != '作废') as total_cnt, ROUND(CAST(COUNT(*) AS REAL) / (SELECT COUNT(*) FROM tax_invoices WHERE invoice_status != '作废') * 100, 2) as pct FROM tax_invoices WHERE CAST(total_amount AS INTEGER) = total_amount AND CAST(total_amount AS INTEGER) % 10000 = 0 AND total_amount >= 10000 AND invoice_status != '作废';

-- B3. 顶额开票检测（R03）
SELECT 'B3_顶额' as tag, CASE WHEN total_amount >= 9999000 THEN '千万级顶额(999.9万+)' WHEN total_amount >= 999000 THEN '百万级顶额(99.9万+)' WHEN total_amount >= 99000 THEN '十万级顶额(9.9万+)' ELSE '其他' END as level, COUNT(*) as cnt, ROUND(SUM(total_amount),2) as total FROM tax_invoices WHERE invoice_status != '作废' GROUP BY level ORDER BY total DESC;

-- B4. 作废率统计（R05）
SELECT 'B4_作废率' as tag, invoice_type, COUNT(*) as total_cnt, SUM(CASE WHEN invoice_status = '作废' OR void_flag = '是' THEN 1 ELSE 0 END) as void_cnt, ROUND(SUM(CASE WHEN invoice_status = '作废' OR void_flag = '是' THEN 1 ELSE 0 END) * 100.0 / COUNT(*), 2) as void_rate FROM tax_invoices GROUP BY invoice_type;

-- B5. 销方对手TOP10（R07）
SELECT 'B5_销方对手TOP10' as tag, buyer_name, COUNT(*) as cnt, ROUND(SUM(total_amount),2) as total FROM seller_invoice WHERE invoice_status != '作废' GROUP BY buyer_name ORDER BY total DESC LIMIT 10;

-- B6. 购方对手TOP10（R07）
SELECT 'B6_购方对手TOP10' as tag, seller_name, COUNT(*) as cnt, ROUND(SUM(total_amount),2) as total FROM buyer_invoice WHERE invoice_status != '作废' GROUP BY seller_name ORDER BY total DESC LIMIT 10;

-- B7. 集中开票天数（R04）
SELECT 'B7_集中开票' as tag, invoice_date, COUNT(*) as cnt, ROUND(SUM(total_amount),2) as total FROM tax_invoices WHERE invoice_status != '作废' GROUP BY invoice_date HAVING cnt > 50 OR total > 1000000 ORDER BY total DESC;

-- B8. 注册至首次开票天数（R14）
SELECT 'B8_注册开票间隔' as tag, t.taxpayer_id, t.taxpayer_name, t.registration_date, MIN(inv.invoice_date) as first_invoice_date, CAST(julianday(MIN(inv.invoice_date)) - julianday(t.registration_date) AS INTEGER) as days_to_first FROM tax_registrations t LEFT JOIN tax_invoices inv ON t.taxpayer_id = inv.seller_tax_id GROUP BY t.taxpayer_id ORDER BY days_to_first;

-- B9. 单价异常（R11）
SELECT 'B9_单价异常' as tag, goods_name, COUNT(*) as cnt, ROUND(MIN(unit_price),2) as min_price, ROUND(MAX(unit_price),2) as max_price, ROUND(AVG(unit_price),2) as avg_price, ROUND(MAX(unit_price) - MIN(unit_price),2) as price_range FROM tax_invoices WHERE unit_price > 0 AND invoice_status != '作废' GROUP BY goods_name HAVING cnt > 10 ORDER BY price_range DESC LIMIT 20;

-- B10. 数量异常（R12）
SELECT 'B10_数量异常' as tag, goods_name, quantity, COUNT(*) as same_qty_cnt FROM tax_invoices WHERE quantity > 0 AND invoice_status != '作废' GROUP BY goods_name, quantity HAVING same_qty_cnt > 3 ORDER BY same_qty_cnt DESC LIMIT 20;

-- B11. 经营性支出缺失（R13）
SELECT 'B11_经营支出' as tag, CASE WHEN goods_name LIKE '%电费%' OR goods_name LIKE '%水费%' THEN '水电费' WHEN goods_name LIKE '%运费%' OR goods_name LIKE '%物流%' OR goods_name LIKE '%运输%' THEN '运费物流' WHEN goods_name LIKE '%办公%' THEN '办公用品' WHEN goods_name LIKE '%工资%' OR goods_name LIKE '%劳务%' THEN '工资劳务' WHEN goods_name LIKE '%租金%' OR goods_name LIKE '%房租%' THEN '租金' ELSE '其他经营支出' END as expense_type, COUNT(*) as cnt, ROUND(SUM(tax_amount),2) as tax_total FROM buyer_invoice WHERE invoice_status != '作废' AND (goods_name LIKE '%电费%' OR goods_name LIKE '%水费%' OR goods_name LIKE '%运费%' OR goods_name LIKE '%物流%' OR goods_name LIKE '%运输%' OR goods_name LIKE '%办公%' OR goods_name LIKE '%工资%' OR goods_name LIKE '%劳务%' OR goods_name LIKE '%租金%' OR goods_name LIKE '%房租%') GROUP BY expense_type ORDER BY tax_total DESC;
```

#### 阶段C+D：虚开网络与团伙识别（phase_cd.sql，1 次调用）

```sql
-- C1. TOP10上游供应商
SELECT 'C1_上游TOP10' as tag, seller_tax_id, seller_name, seller_tax_authority, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax, ROUND(SUM(total_amount),2) as total_with_tax, ROUND(AVG(total_amount),2) as avg_per_invoice FROM buyer_invoice WHERE invoice_status != '作废' GROUP BY seller_tax_id, seller_name ORDER BY total_with_tax DESC LIMIT 10;

-- C2. TOP10下游客户
SELECT 'C2_下游TOP10' as tag, buyer_tax_id, buyer_name, buyer_tax_authority, COUNT(*) as cnt, ROUND(SUM(amount),2) as total_amount, ROUND(SUM(tax_amount),2) as total_tax, ROUND(SUM(total_amount),2) as total_with_tax, ROUND(AVG(total_amount),2) as avg_per_invoice FROM seller_invoice WHERE invoice_status != '作废' GROUP BY buyer_tax_id, buyer_name ORDER BY total_with_tax DESC LIMIT 10;

-- C3. 对开企业检测
SELECT 'C3_对开' as tag, a.seller_tax_id as entity_a, a.buyer_tax_id as entity_b, a.total as a_to_b, b.total as b_to_a FROM (SELECT seller_tax_id, buyer_tax_id, ROUND(SUM(total_amount),2) as total FROM tax_invoices WHERE invoice_status != '作废' GROUP BY seller_tax_id, buyer_tax_id) a JOIN (SELECT seller_tax_id, buyer_tax_id, ROUND(SUM(total_amount),2) as total FROM tax_invoices WHERE invoice_status != '作废' GROUP BY seller_tax_id, buyer_tax_id) b ON a.seller_tax_id = b.buyer_tax_id AND a.buyer_tax_id = b.seller_tax_id WHERE a.seller_tax_id < a.buyer_tax_id;

-- C4. 进销时间倒挂
SELECT 'C4_时间倒挂' as tag, s.seller_tax_id, s.goods_name as out_goods, s.invoice_date as out_date, b.goods_name as in_goods, b.invoice_date as in_date FROM seller_invoice s JOIN buyer_invoice b ON s.seller_tax_id = b.buyer_tax_id AND s.goods_name = b.goods_name WHERE s.invoice_date < b.invoice_date AND s.invoice_status != '作废' AND b.invoice_status != '作废';

-- D1. IP设备关联
SELECT 'D1_IP关联' as tag, ip, GROUP_CONCAT(DISTINCT seller_tax_id) as entities, COUNT(DISTINCT seller_tax_id) as entity_count, COUNT(*) as invoice_count, ROUND(SUM(total_amount),2) as total FROM tax_invoices WHERE ip IS NOT NULL AND ip != '' AND invoice_status != '作废' GROUP BY ip HAVING entity_count >= 2 ORDER BY entity_count DESC;

-- D2. MAC设备关联
SELECT 'D2_MAC关联' as tag, mac, GROUP_CONCAT(DISTINCT seller_tax_id) as entities, COUNT(DISTINCT seller_tax_id) as entity_count, COUNT(*) as invoice_count, ROUND(SUM(total_amount),2) as total FROM tax_invoices WHERE mac IS NOT NULL AND mac != '' AND invoice_status != '作废' GROUP BY mac HAVING entity_count >= 2 ORDER BY entity_count DESC;

-- D3. 同法人关联
SELECT 'D3_同法人' as tag, legal_person_name, legal_person_id, GROUP_CONCAT(DISTINCT taxpayer_name) as companies, COUNT(DISTINCT taxpayer_id) as company_count FROM tax_registrations WHERE legal_person_id IS NOT NULL AND legal_person_id != '' GROUP BY legal_person_id HAVING company_count >= 2 ORDER BY company_count DESC;

-- D4. 同办税人关联
SELECT 'D4_同办税人' as tag, tax_handler_name, tax_handler_id, GROUP_CONCAT(DISTINCT taxpayer_name) as companies, COUNT(DISTINCT taxpayer_id) as company_count FROM tax_registrations WHERE tax_handler_id IS NOT NULL AND tax_handler_id != '' GROUP BY tax_handler_id HAVING company_count >= 2 ORDER BY company_count DESC;

-- D5. 同财务负责人关联
SELECT 'D5_同财务' as tag, finance_officer_name, finance_officer_id, GROUP_CONCAT(DISTINCT taxpayer_name) as companies, COUNT(DISTINCT taxpayer_id) as company_count FROM tax_registrations WHERE finance_officer_id IS NOT NULL AND finance_officer_id != '' GROUP BY finance_officer_id HAVING company_count >= 2 ORDER BY company_count DESC;

-- D6. 同注册地址关联
SELECT 'D6_同地址' as tag, registered_address, GROUP_CONCAT(DISTINCT taxpayer_name) as companies, COUNT(DISTINCT taxpayer_id) as company_count FROM tax_registrations WHERE registered_address IS NOT NULL AND registered_address != '' GROUP BY registered_address HAVING company_count >= 2 ORDER BY company_count DESC;

-- D7. 法人年龄异常
SELECT 'D7_法人年龄' as tag, taxpayer_name, legal_person_name, legal_person_id, CAST(strftime('%Y','now') - CAST(substr(legal_person_id,7,4) AS INTEGER) AS INTEGER) as age FROM tax_registrations WHERE length(legal_person_id) = 18 AND CAST(strftime('%Y','now') - CAST(substr(legal_person_id,7,4) AS INTEGER) AS INTEGER) >= 70;

-- D8. 同电话关联
SELECT 'D8_同电话' as tag, phone, companies, company_count FROM (SELECT legal_person_phone as phone, GROUP_CONCAT(DISTINCT taxpayer_name) as companies, COUNT(DISTINCT taxpayer_id) as company_count FROM tax_registrations WHERE legal_person_phone IS NOT NULL AND legal_person_phone != '' GROUP BY legal_person_phone HAVING company_count >= 2 UNION ALL SELECT tax_handler_phone as phone, GROUP_CONCAT(DISTINCT taxpayer_name) as companies, COUNT(DISTINCT taxpayer_id) as company_count FROM tax_registrations WHERE tax_handler_phone IS NOT NULL AND tax_handler_phone != '' GROUP BY tax_handler_phone HAVING company_count >= 2) ORDER BY company_count DESC;
```

#### 阶段E：资金流分析（phase_e.sql，1 次调用，如有银行数据）

```sql
-- E1. 资金回流筛查
SELECT 'E1_资金回流' as tag, bt.account_no, bt.counterparty_account, bt.counterparty_name, ROUND(SUM(CASE WHEN bt.direction='in' THEN bt.amount ELSE 0 END),2) as in_amount, ROUND(SUM(CASE WHEN bt.direction='out' THEN bt.amount ELSE 0 END),2) as out_amount FROM bank_transactions bt WHERE bt.counterparty_account != '' AND bt.counterparty_account != bt.account_no GROUP BY bt.account_no, bt.counterparty_account HAVING in_amount > 0 AND out_amount > 0 ORDER BY (in_amount + out_amount) DESC;

-- E2. 快进快出天数
SELECT 'E2_快进快出' as tag, date(txn_time) as d, ROUND(SUM(CASE WHEN direction='in' THEN amount ELSE 0 END),2) as in_amt, ROUND(SUM(CASE WHEN direction='out' THEN amount ELSE 0 END),2) as out_amt FROM bank_transactions WHERE amount > 0 GROUP BY d HAVING in_amt > 50000 AND out_amt > 50000;

-- E3. 交易对手地域
SELECT 'E3_对手地域' as tag, counterparty_bank, COUNT(DISTINCT counterparty_account) as accounts, COUNT(*) as cnt, ROUND(SUM(amount),2) as total FROM bank_transactions WHERE counterparty_bank IS NOT NULL AND counterparty_bank != '' GROUP BY counterparty_bank ORDER BY total DESC;
```

#### 阶段F+H：时间转折点与可疑指标（phase_fh.sql，1 次调用）

```sql
-- F1. 月度开票趋势
SELECT 'F1_月度趋势' as tag, substr(invoice_date,1,7) as month, COUNT(*) as cnt, ROUND(SUM(total_amount),2) as total, COUNT(DISTINCT buyer_tax_id) as buyer_count FROM seller_invoice WHERE invoice_status != '作废' GROUP BY month ORDER BY month;

-- F2. 月度环比增长率
SELECT 'F2_环比增长' as tag, month, total, ROUND((total - LAG(total) OVER (ORDER BY month)) * 100.0 / NULLIF(LAG(total) OVER (ORDER BY month), 0), 2) as growth_rate_pct FROM (SELECT substr(invoice_date,1,7) as month, ROUND(SUM(total_amount),2) as total FROM seller_invoice WHERE invoice_status != '作废' GROUP BY month) ORDER BY month;

-- H1. 够罪条件（专票）
SELECT 'H1_够罪专票' as tag, seller_tax_id, seller_name, COUNT(*) as invoice_cnt, ROUND(SUM(tax_amount),2) as total_tax, ROUND(SUM(total_amount),2) as total_with_tax FROM tax_invoices WHERE invoice_type LIKE '%专用%' AND invoice_status != '作废' GROUP BY seller_tax_id ORDER BY total_tax DESC;

-- H2. 够罪条件（普票）
SELECT 'H2_够罪普票' as tag, seller_tax_id, seller_name, COUNT(*) as invoice_cnt, ROUND(SUM(total_amount),2) as total_with_tax FROM tax_invoices WHERE invoice_type LIKE '%普通%' AND invoice_status != '作废' GROUP BY seller_tax_id ORDER BY total_with_tax DESC;

-- H3. 可疑指标汇总
SELECT 'H3_可疑指标' as tag, metric, value FROM (SELECT '进销品名匹配率' as metric, ROUND((SELECT COUNT(DISTINCT goods_name) FROM seller_invoice WHERE goods_name IN (SELECT DISTINCT goods_name FROM buyer_invoice) AND invoice_status != '作废') * 100.0 / NULLIF((SELECT COUNT(DISTINCT goods_name) FROM seller_invoice WHERE invoice_status != '作废'), 0), 2) as value UNION ALL SELECT '整额开票占比', ROUND(CAST(COUNT(*) AS REAL) * 100.0 / NULLIF((SELECT COUNT(*) FROM tax_invoices WHERE invoice_status != '作废'), 0), 2) FROM tax_invoices WHERE CAST(total_amount AS INTEGER) = total_amount AND CAST(total_amount AS INTEGER) % 10000 = 0 AND total_amount >= 10000 AND invoice_status != '作废' UNION ALL SELECT '作废率', ROUND(SUM(CASE WHEN invoice_status = '作废' THEN 1 ELSE 0 END) * 100.0 / COUNT(*), 2) FROM tax_invoices);
```

### 重要注意事项

1. **所有数据操作必须通过 excelcli 完成**：禁止使用 Python/pandas/venv/pip，禁止编写 Python 脚本。违反将浪费 5+ 分钟在环境依赖问题上。
2. **用 `&&` 串联命令**：多个连续的 excelcli 命令用 `&&` 连接在一次 bash 调用中完成，减少工具调用次数。
3. **用 `--file` 批量执行 SQL**：不要逐条调用 `excelcli query --sql`，将 SQL 写入 `.sql` 文件后用 `--file` 一次执行。
4. **始终设置 `--limit`**：SQL 查询加 `--limit 200`（或更小），避免返回数据过多撑爆 prompt 上下文导致 AI 调用失败。
5. **必须先 inspect 再分析**：不同系统的发票数据格式差异很大，先 inspect 确认列名和数据格式
6. **必须先 tax map 再 tax normalize**：normalize 依赖映射文件，map 生成的 JSON 文件是桥梁
7. **标准化后使用英文标准列名**：`tax_invoices` 等标准表使用上方列名对照表中的英文名，SQL 查询中禁止使用中文列名
8. **normalize 后先查 PRAGMA**：在 phase_a.sql 的最前面加 `PRAGMA table_info(tax_invoices);` 确认实际列名
9. **区分专票/普票**：通过 `invoice_type` 字段区分，后续所有金额统计必须分开呈现
10. **过滤作废发票**：正常分析时排除 `invoice_status = '作废'` 的发票，但作废率分析时需要统计

## 执行编排（强制）

当用户提出”调用本技能分析某目录下涉税数据”时，必须按以下顺序执行，**总工具调用次数不超过 10-12 次**：

1. **第1次调用**：`excelcli import`（所有文件）`&& excelcli inspect && excelcli tax map && excelcli tax normalize` — 用 `&&` 串联，一次完成导入到标准化
2. **第2次调用**（如有银行数据）：`excelcli import + map + normalize` — 一次完成银行流水标准化
3. **第3次调用**：`excelcli tax analyze && excelcli tax invoices` — 风险概览
4. **第4次调用**：`cat > /tmp/phase_a.sql ... && excelcli query <DB> --file /tmp/phase_a.sql --json --limit 200` — 阶段A全部查询
5. **第5次调用**：`cat > /tmp/phase_b.sql ... && excelcli query <DB> --file /tmp/phase_b.sql --json --limit 200` — 阶段B全部查询
6. **第6次调用**：`cat > /tmp/phase_cd.sql ... && excelcli query <DB> --file /tmp/phase_cd.sql --json --limit 200` — 阶段C+D全部查询
7. **第7次调用**（如有银行数据）：`cat > /tmp/phase_e.sql ... && excelcli query <DB> --file /tmp/phase_e.sql --json --limit 200` — 阶段E全部查询
8. **第8次调用**：`cat > /tmp/phase_fh.sql ... && excelcli query <DB> --file /tmp/phase_fh.sql --json --limit 200` — 阶段F+H全部查询
9. **第9-12次调用**：按纳税人逐户写报告 + 生成总报告（`write_file`）

**核心约束**：
- **禁止逐条执行 `excelcli query --sql`**。每条 SQL 都必须写入 `.sql` 文件后用 `--file` 批量执行
- **禁止使用 Python**。所有操作只能用 excelcli 命令和 bash
- **SQL 查询结果加 `--limit`**。默认 `--limit 200`，避免返回数据过多
- **每次 bash 调用尽量包含多条命令**。用 `&&` 串联或用 heredoc 写文件

### 默认输出目录结构

```text
output/tax-fraud-analysis-0326/
  analysis.db
  table-mapping.json
  reports/
    taxpayer-{纳税人识别号或名称}.md
  summary-report.md
```

### 输出约束

- 不覆盖用户原始 xlsx
- 每次任务使用独立输出目录
- 报告正文使用 Markdown
- 关键中间结果优先通过 SQL 查询结果生成，不落无意义临时文件

### 执行方式约束（强制）

- **不落地中间脚本**：不应生成 Python/JS 等自定义分析脚本，数据处理统一由 `excelcli` 命令完成
- **SQL 查询批量执行**：用 `--file` 批量执行 SQL，不逐条调用 `excelcli query --sql`
- **Markdown 直接写入目标文件**：逐户报告和汇总报告直接写入 `.md` 文件
- **只有用户明确要求时才生成额外工具文件**；默认不产出代码文件
- **总工具调用不超过 10-12 次**：通过串联命令和批量执行来控制调用次数


## 数据文件说明

涉税犯罪分析数据包含以下五类文件（文件名可能不同，以实际列名为准）：

### 1. 纳税人信息

纳税人的税务登记基础信息，用于企业画像、人员关联分析和异常注册特征识别。


### 2. 专票销方

目标企业作为销方开具的增值税发票明细，每条记录为一张发票的一条明细行。用于分析开票行为、销售品名、税率分布、购方特征等。


### 3. 普票购方

目标企业作为购方接收的增值税发票明细，字段结构与销方数据表一致。用于分析进项结构、上游供应商特征、进销匹配等。

### 4. 专票购方

目标企业作为购方接收的增值税发票明细，字段结构与销方数据表一致。用于分析进项结构、上游供应商特征、进销匹配等。

### 5. 工商信息表（如有提供）

涉案企业的工商注册信息，用于企业关联分析、空壳企业识别和股东/法人穿透。


### 6. 银行交易明细表（如有提供）

涉案企业及关联人的银行账户交易流水，用于资金回流分析、交易对手地域分析和虚开资金链追踪。


**关键分析价值**：
- **资金回流分析**：追踪开票方收到的货款是否通过关联账户回流至受票方或其实际控制人，形成"资金闭环"
- **交易对手地域分析**：通过对手开户行归属地识别资金流向的地域特征，判断虚开团伙的地域范围（如资金集中流向某省某市）
- **开票与资金流匹配**：比对发票开具时间/金额与银行实际收付款时间/金额是否一致
- **可疑资金模式识别**：公转私频繁转账、资金快进快出、整额转账等特征

> **文件匹配原则**：用户提供的文件可能名称不同，但通过列名可以识别文件类型。优先匹配发票数据表（通过"发票代码""发票号码""购方识别号""销方识别号"等关键字段），再匹配税务登记表（通过"纳税人识别号""社会信用代码"等字段），再匹配工商信息表（通过"统一社会信用代码""经营范围"等字段），最后匹配银行交易明细表（通过"交易金额/发生额""对方户名""对方账号""收付标志"等字段）。各表之间通过"纳税人识别号/统一社会信用代码"和"账户名称/企业名称"进行关联。

## 分析框架

分析分为八个阶段，按顺序执行。每个阶段完成后检查产出是否齐全再进入下一阶段。

### 阶段A：数据探查与企业画像

**目标**：了解涉案企业全貌，建立基础数据认知，初步判断企业经营属性和纳税特征。

**操作**：使用 `excelcli import` 导入数据，`excelcli inspect` 确认列名和数据格式，`excelcli tax map` + `excelcli tax normalize` 完成标准化，然后通过 `excelcli tax analyze` 和 `excelcli query --sql` 执行 SQL 查询集 A1-A9 计算以下指标。

**首先确定分析主体**：读取税务登记信息表，确认涉案企业数量和基本信息。如果只有1个企业则为单主体分析；如果有多个企业，需在报告开头列出所有涉案企业，后续分析分别或合并进行。

**企业属性判断**：根据税务登记信息中的"课征主题类型名称""登记注册类型""行业种类"和工商信息中的"企业(机构)类型""国标行业门类/大类/中类/小类""经营范围"，判断企业的行业属性和经营性质。在涉税犯罪中，需特别关注商贸类、贸易类、进出口类企业。

**数据预处理**：
1. **数据去重（必须首先执行）**：任何导入的数据都必须去重。去重时排除无分析意义的字段（如"序号""登记序号"等人为编号字段），以其余所有字段联合判断是否重复。应优先使用 `excelcli query --sql` 执行 SQL 去重（如 `SELECT DISTINCT ...` 或去重视图/CTE），并记录去重前后条数。
2. 发票日期列按 SQL 可计算格式处理：优先用 `substr()`、`date()`、`julianday()` 等函数完成时间统计。
3. 过滤作废发票：如"作废标志"列存在，需区分正常发票和已作废发票，分别统计
4. 金额列计算时统一使用 `CAST(金额列 AS REAL)`，并对空值做 `CASE WHEN` 防错处理。
5. 用 `excelcli query <DB> --sql "PRAGMA table_info(表名)"` 和 `excelcli query <DB> --sql "SELECT * FROM 表名 LIMIT 3"` 确认实际列名与样例数据。
6. 关联各表：通过"纳税人识别号""社会信用代码""统一社会信用代码"将税务登记、发票数据、工商信息和银行交易明细关联
7. 数据质量检查：检查发票状态、作废标志等字段，确认数据完整性
8. **区分专票与普票**：通过"发票类型"或"开具发票类型"字段区分增值税专用发票（专票）和增值税普通发票（普票），后续所有统计须按专票/普票分别进行。专票和普票在定罪量刑上存在重大差异，必须在分析中明确区分

**必须产出的指标**：
- 涉案企业数量及各企业基本信息（名称、纳税人识别号、状态、注册类型、行业种类）
- 企业登记时间与经营时长
- 纳税人状态分布（正常、注销、非正常等）
- **专票/普票分类统计（核心）**：按"发票类型"或"开具发票类型"区分增值税专用发票和增值税普通发票，分别统计笔数、金额、税额。专票和普票的定罪量刑标准不同（详见"够罪条件分析"），后续所有发票分析均须按专票/普票分别呈现
- **销方数据概况**：开票总笔数（发票张数与明细行数）、开票总金额（货物金额）、总税额、价税合计总额、时间跨度；其中专票X张X元、普票X张X元
- **购方数据概况**：收票总笔数、收票总金额、总税额、价税合计总额、时间跨度；其中专票X张X元、普票X张X元
- **进销对比**：进项金额与销项金额的比值、进项税额与销项税额的比值
- **税率分布**：按税率值分别统计进项和销项的笔数和金额
- **货品名称分布**：按"货品名称"统计开票笔数和金额（TOP20）
- **商品编码分布**：按"商品编码"前几位统计大类分布
- **发票状态分布**：正常、作废、红冲等各状态的笔数和金额
- **认证情况统计**：已认证/未认证的发票笔数和金额占比
- **开票日期趋势**：按月/季度/年分组的开票笔数和金额趋势
- **购销方地域分布**：通过"购方税务机关""销方税务机关"或地址字段统计地域分布
- **异地发票占比**：异地发票标志的统计
- **银行交易概况（如有交易数据）**：交易总笔数、收入/支出金额、时间跨度、主要交易对手分布、资金流入流出地域特征

### 阶段B：发票异常特征筛查

**目标**：对发票数据进行系统性异常筛查，标记可疑发票和可疑交易对手。通过 `excelcli query --sql` 执行 SQL 查询集 B1-B10 完成以下筛查项目的量化计算。

**必须执行的筛查项目**：

1. **进销品名匹配检测（R01）**：比对进项发票货品名称与销项发票货品名称，检测是否存在"进A出B"（进项和销项货品大类不一致）的变票嫌疑
2. **整额开票检测（R02）**：统计价税合计为整万元的发票占比
3. **顶额开票检测（R03）**：统计接近发票限额（如价税合计接近10万、100万、1000万等整数上限）的发票占比
4. **集中开票检测（R04）**：统计单日开票超过一定笔数或金额的天数，检测是否存在短期内大量开票行为
5. **作废/红冲异常检测（R05）**：统计作废率（作废发票笔数/总开票笔数），以及作废后立即重开的模式
6. **税率异常检测（R06）**：检测是否存在与行业不匹配的税率（如贸易企业出现6%服务税率）、同一货品名称使用不同税率
7. **购销对手集中度检测（R07）**：分析购方/销方是否高度集中在少数几个企业
8. **异地开票检测（R08）**：统计异地发票标志的发票占比，检测跨区域虚开
9. **开票时间异常检测（R09）**：检测深夜开票、节假日集中开票等异常时间模式
10. **设备指纹关联检测（R10）**：通过IP、MAC、主板序列号识别不同企业是否使用同一设备开票
11. **单价异常检测（R11）**：对同一商品编码的单价进行统计分析，检测是否存在明显偏离市场价格的开票；特别关注农产品收购发票金额是否明显高于市场价格（来源：技战法序号47）；检测出口申报单价是否远高于同期同类商品均价（货值倒挂，来源：技战法序号35）
12. **数量单位异常检测（R12）**：检测数量为负数、小数点异常或数量与金额不匹配的情况；检测同一企业多张发票的货物数量是否完全一致（与正常经营不符，来源：技战法序号20）
13. **经营性支出发票缺失检测（R13）**：从进项发票中按货品名称分类统计经营性支出发票（电费、水费、运费/物流费、办公用品、工资/劳务费等），计算各类经营性支出发票税额占进项总税额的比率；空壳公司通常无水电、工资、物流等经营性支出的进项发票（来源：技战法序号7/14-18）
14. **注册即开票检测（R14）**：比对税务登记日期/开业设立日期与首张开票日期的间隔天数，检测是否存在注册后极短时间内（如3天内）即开始大量开票的行为（来源：技战法序号7）
15. **行业与品类匹配检测（R15）**：将税务登记信息中的"行业种类"与发票中的实际货品名称/商品编码进行交叉比对，检测企业名义行业（如农业）与实际开票品类（如电子产品）是否严重不符（来源：技战法序号32）
16. **开票IP地理位置检测（R16）**：将发票数据中的IP地址解析为地理位置，与税务登记信息中的注册地址行政区划、生产经营地址行政区划进行比对；统计开票IP对应的省份数量，如IP位于多省则高度异常（来源：技战法序号19/21）
17. **月度销售波动率检测（R17）**：计算月度销售额的环比增长率（本月-上月/上月）和同比增长率（本月-去年同月/去年同月），检测是否存在异常波动（来源：技战法序号28/29）
18. **原材料成本率检测（R18）**：计算进项发票中原材料类发票金额占销项总金额的比率，与行业合理区间比对，过低说明进货成本与销售不匹配（来源：技战法序号30）

19. **进项票据造假检测（R19）**：针对农产品收购发票，检测农户身份证号码是否存在"一证多人"（同一身份证号对应多个不同姓名）、"一人多证"（同一姓名对应多个不同身份证号）、号码格式错误（如出生年份不合理，年龄与农业种植不符如不满20岁）等异常；统计开票农户是否为受票方公司员工（如有员工数据可关联核查）（来源：技战法#16/19）
20. **收购价格波动检测（R20）**：对农产品收购发票，按月统计同一货品名称的平均收购单价，计算月间价格波动率；正常收购企业在不同时间、不同农户处收购的价格应有一定波动，如果各月均价完全一致或波动极小（<1%），则违背市场规律，存在虚构交易嫌疑（来源：技战法#20）
21. **货物增值率异常检测（R21）**：计算出口货物增值率 = 出口单价 ÷ 收购单价；对同一商品的进项收购单价与销项出口单价进行对比，增值率超过行业合理倍数（如超过3-5倍）为异常（来源：技战法#5）
22. **货物成品率异常检测（R22）**：对比进项发票中原材料购进数量与销项发票中成品销售数量，计算成品率 = 成品销售数量 ÷ 原材料购进数量；成品率远低于行业正常水平或出口销售数量超过购进原材料数量均为异常（来源：技战法#6）
23. **受票方单一农户开票金额巨大检测（R23）**：统计农产品收购发票中每个农户（按身份证号或姓名去重）的累计开票金额和发票张数，单一农户年度开票超过一定金额（如500万元以上）与个体农户正常生产体量明显不符（来源：技战法#18）
24. **企业经营异常名录/失信检测（R24）**：检查涉案企业是否因未按规定报送年度报告被列入经营异常名录，或存在失信等行为；结合工商信息表中登记状态字段判断（来源：技战法#15）

**产出**：每项筛查的量化结果、异常发票清单、可疑对手名单。

### 阶段C：虚开网络与票流追踪

**目标**：追踪发票的上下游流向，构建虚开发票网络，识别票流闭环和虚开链条。通过 `excelcli query --sql` 执行 SQL 查询集 C1-C5 完成上下游分析和闭环检测。

**子任务C1 — 上游供应商分析（进项端）**：

- 进项发票涉及多少个不同的销方企业
- TOP10上游供应商表：销方识别号、销方名称、销方税务机关、发票笔数、货物金额、税额、价税合计、平均单票金额、主要货品名称、特征标注
- 上游集中度：TOP1 / TOP3 / TOP5 占总进项的百分比
- 上游企业状态核查：通过纳税人识别号关联税务登记信息和工商信息，检查供应商是否存在注销、吊销、非正常等异常状态
- 上游企业地域分布：统计供应商所在地区

**子任务C2 — 下游客户分析（销项端）**：

- 销项发票涉及多少个不同的购方企业
- TOP10下游客户表：购方识别号、购方名称、购方税务机关、发票笔数、货物金额、税额、价税合计、平均单票金额、主要货品名称、特征标注
- 下游集中度：TOP1 / TOP3 / TOP5 占总销项的百分比
- 下游企业状态核查：同C1
- 下游企业地域分布

**子任务C3 — 票流闭环检测（R13）**：

构建发票流转有向图，检测A→B→C→...→A的环形票流路径：
1. 以企业识别号为节点，发票金额和开票日期为边属性，构建有向图
2. 销方数据中的购方企业 = 对外开票的下游；购方数据中的销方企业 = 接收发票的上游
3. 使用DFS或BFS检测3-5步以内的票流闭环
4. 对检测到的闭环路径，计算闭环金额、时间跨度、各环节品名变化

**子任务C4 — 进销匹配分析（R14）**：

1. 比对进项和销项的货品名称大类：是否存在进项为A类商品、销项为B类商品的"变票"行为
2. 比对进项和销项的金额关系：销项金额是否显著高于进项金额（高开），或进销金额高度一致（对开/环开）
3. 比对进项和销项的时间关系：是否存在"先开销项后取进项"的倒挂现象
4. 检测进项和销项的数量关系：进货数量与销货数量是否匹配

**子任务C5 — 发票认证时效分析（R15）**：

1. 统计发票从开具到认证的时间间隔分布
2. 检测是否存在开票后立即认证的异常行为（如当天开当天认证）
3. 检测临近认证期限集中认证的行为

**子任务C6 — 虚开链条扩线**：

基于已识别的可疑企业：
1. 从销方数据中找出可疑企业的所有购方，作为下游扩线对象
2. 从购方数据中找出可疑企业的所有销方，作为上游扩线对象
3. 对新发现的关联企业进行初步异常筛查
4. 循环迭代，扩展涉案企业范围

**分析要点**：
- 购方/销方信息分布在发票表的"购方识别号""购方名称""销方识别号""销方名称"等列中
- 可通过"纳税人识别号/统一社会信用代码"关联税务登记信息和工商信息，获取企业背景
- 同一企业可能同时出现在销方数据和购方数据中（既是开票方又是收票方）
- TOP10表中的"特征"列需标注：异常状态企业、高频开票、最大进/销项、双向交易、疑似空壳、疑似暴力虚开等

### 阶段D：人员关联与团伙识别

**目标**：通过法定代表人、财务负责人、办税人、设备指纹、地域关联和工商关系，识别犯罪团伙网络和组织架构。通过 `excelcli query --sql` 执行 SQL 查询集 D1-D8 完成人员关联和团伙识别。

**子任务D1 — 基于开票设备的团伙划分（R10）**：

发票数据表中的IP/MAC/主板序列号可用于设备关联，通过 `excelcli query --sql` 执行 SQL 查询 D1-D2 完成设备关联分析：
1. 从销方数据中提取IP、MAC、主板序列号信息
2. 统计每个IP/MAC/主板序列号涉及的企业（销方识别号）数量
3. 使用 Union-Find 算法：共享同一IP、MAC或主板序列号的企业归为同一团伙
4. 输出每个团伙的成员企业、共享设备信息、开票金额

**子任务D2 — 基于税务登记人员的关联分析**：

利用税务登记信息表进行人员关联，通过 `excelcli query --sql` 执行 SQL 查询 D3-D8 完成人员关联分析：
1. **同法人关联**：通过"法定代表人姓名""法定代表人身份证号码"识别由同一法人控制的多个企业；同时统计单个法人关联的企业数量，数量过多为异常（来源：技战法序号9）
2. **同财务负责人关联**：通过"财务负责人姓名""财务负责人身份证件号码"识别共用同一财务负责人的企业
3. **同办税人关联**：通过"办税人姓名""办税人身份证件号码"识别由同一办税人代办的企业
4. **同联系电话关联**：通过法定代表人、财务负责人、办税人的固定电话和移动电话进行交叉关联
5. **法人年龄异常检测**：从法定代表人身份证号码提取出生日期，计算年龄，标记年龄偏大（如70岁以上）的法人——虚开团伙常利用老年人身份注册空壳公司（来源：技战法序号8）
6. **人员交叉任职/互为法人监事检测**：检测关联企业间的法人、财务负责人、办税人是否存在A企业法人=B企业办税人、或多个企业法人与监事互为的情况（来源：技战法序号4）
7. 构建企业-人员关联网络图，标注关联类型

**子任务D3 — 基于工商信息的关联分析**：

利用工商信息表进行深层关联，通过 `excelcli query --sql` 查询 `business_registration` 表完成关联分析：
1. **同法定代表人关联**：与D2互补验证
2. **同注册地址关联**：通过"企业地址"识别注册在相同地址的企业（"同址不同企"为空壳公司典型特征，来源：技战法序号2）
3. **同电话/邮箱关联**：通过"电话""更多电话""邮箱""更多邮箱"识别共用联系方式的企业（来源：技战法序号1/3）
4. **同登记机关关联**：分析是否在同一登记机关集中注册
5. **企业状态分析**：标注已注销、吊销、异常的企业；检查是否因未按规定报送年度报告被列入经营异常名录或存在失信行为（来源：技战法#15）
6. **关联企业经营范围相似度检测**：对同一法人/办税人控制的多个企业，比对其"经营范围""国标行业大类"是否高度雷同——虚开团伙批量注册的空壳公司经营范围通常高度相似（来源：技战法序号5）
7. **企业关联性异常检测**：检测关联企业是否存在注册地址一致、董事/监事/高管交叉任职、对公账户流转存在多个相同交易对手等复合关联特征（来源：技战法#11）

**子任务D4 — 基于地域的聚合分析**：

1. 从税务登记信息中提取"生产经营地址行政区划数字代码""注册地址行政区划数字代码"
2. 从工商信息中提取"所属省份""所属城市""所属区县"
3. 将注册在相同区域的企业归为同一地域组
4. 重点关注虚开高发地区的企业集中度
5. 分析"注册地"与"经营地"不一致的企业
6. **税务政策宽松地区标注**：识别注册在税务政策较为宽松地区（如部分农业发达地区、经济开发区、招商引资园区等）的企业，这些地区可能被虚开团伙利用（来源：技战法税务异常一）
7. **商品流向合理性分析**：结合上下游企业地域和货品名称，检测商品流向是否合理（如煤炭从东部卖往产煤区、舍近求远选择较远报关地等，来源：技战法序号37/38）

**子任务D5 — 基于交易关系的团伙拓展**：

1. 稳定开票关系：高频次、大金额的固定上下游企业组
2. 对开关系：A开给B、B同时开给A的互开对手
3. 环开关系：A→B→C→A的环形开票链
4. 综合以上关联分析，构建完整的犯罪团伙网络架构

**数据缺失处理**：
- 如果发票数据中没有IP/MAC/主板序列号列：跳过D1，仅做D2-D5
- 如果没有税务登记信息表：跳过D2，在报告中注明"因未提供税务登记信息表，未进行税务人员关联分析"
- 如果没有工商信息表：跳过D3，在报告中注明"因未提供工商信息表，未进行工商关联分析"
- 不要因为部分数据缺失而放弃整个团伙分析章节

### 阶段E：资金流分析（如有银行交易数据）

**目标**：结合银行交易明细，追踪资金流向，验证发票交易的真实性，识别资金回流和虚开资金链。通过 `excelcli query --sql` 执行 SQL 查询集 E1-E3 完成资金流分析。需先通过 `excelcli map` + `excelcli normalize` 将银行流水标准化为 `bank_transactions` 表。

**子任务E1 — 资金回流分析**：
通过 `excelcli query --sql` 执行 SQL 查询 E1 完成资金回流筛查：
1. 将发票数据（购方→销方的开票关系）与银行交易数据（付款→收款的资金关系）进行比对
2. 追踪开票方收款后的资金去向：是否通过关联个人账户、中间账户回流至受票方或其实际控制人
3. 检测"快进快出"特征：大额资金入账后短时间内（如当日或次日）以相近金额转出
4. 识别资金闭环：受票方付款→开票方账户→关联个人→回流受票方

**子任务E2 — 交易对手地域分析**：
通过 `excelcli query --sql` 执行 SQL 查询 E3 完成交易对手地域分析：
1. 通过对手开户行归属地统计资金流入/流出的地域分布
2. 识别资金集中流向的地域（如资金大量转至某省某市），结合发票流向判断虚开团伙的地域特征
3. 例如：发票开给本市企业，但资金却转至河南某市，说明背后可能是河南的虚开团伙

**子任务E3 — 开票与资金匹配分析**：
1. 比对发票日期/金额与银行实际收付款日期/金额，检测是否存在"有票无款"或"有款无票"
2. 检测付款方是否与发票购方一致（第三方付款为异常信号）
3. 统计公转私频繁转账、整额转账等可疑资金模式

**数据缺失处理**：如未提供银行交易数据，跳过本阶段，在报告中注明"因未提供银行交易明细，未进行资金流分析。建议调取涉案企业及关联人的银行流水进行资金回流验证"。

### 阶段F：时间维度转折点分析

**目标**：识别企业从正常经营转为虚开行为的时间节点，分析资金和开票行为在时间上的特征变化。通过 `excelcli query --sql` 执行 SQL 查询集 F1-F2 完成时间维度分析。

**操作**：
1. **经营阶段划分**：按月统计开票金额、开票频率、交易对手数量、货品名称变化，识别出明显的行为转折点（如某月开始开票金额陡增、交易对手突变、品名大类改变）
2. **正常经营期 vs 异常期特征对比**：对比转折点前后的开票均值、交易对手稳定性、品名集中度等指标，量化行为变化幅度
3. **资金行为时间特征（如有银行数据）**：对比转折点前后的资金流入流出模式变化，如转折点后出现大量公转私、资金快进快出等
4. **总结每家企业的时间特征**：如"该企业202X年X月前为正常经营期（月均开票X万元，主要对手为本地企业），202X年X月起进入虚开期（月均开票陡增至X万元，新增大量外地交易对手）"

**产出**：每家涉案企业的时间线分析，标注关键转折点和前后对比数据。

### 阶段G：犯罪类型判断、够罪分析与模式研判

**目标**：基于前六阶段的分析结果，匹配涉税犯罪类型，进行够罪条件分析，深度研判犯罪模式。

**操作**：
1. 将前六阶段的量化指标与下文"犯罪模式识别规则"逐一比对，确定最可能的犯罪类型
2. **对每家涉案企业打标签**：根据其发票特征和行为模式，为每家企业标注虚开类型标签（如"暴力虚开型""变票型""对开/环开型"等），而非将暴力虚开、变票虚开等作为独立分析模块。标签应作为企业画像的一部分
3. **够罪条件分析（必须）**：根据专票和普票分别计算涉案金额，对照法律标准判断是否够罪（详见下文"够罪条件标准"）
4. **虚开模式深度研判（必须）**：不仅判断犯罪类型，还需深入分析犯罪模式的内在逻辑（详见下文"虚开模式深度分析要点"）

### 阶段H：可疑特征量化与研判

**目标**：对前七阶段的发现进行系统性量化评估，形成结论。通过 `excelcli query --sql` 执行 SQL 查询集 H1-H3 完成够罪条件计算和可疑指标汇总。（详见下方"可疑指标与阈值"），逐项给出数值和判定结论，作为报告"可疑点分析"章节的依据。完成后综合所有指标和模式匹配结果，形成最终研判结论。

---

## 够罪条件标准

虚开增值税专用发票与虚开增值税普通发票的定罪量刑标准存在重大差异，分析中必须按专票/普票分别计算涉案金额并对照以下标准。

### 虚开增值税专用发票罪（刑法第205条）

| 量刑档次 | 虚开税额 | 对应刑罚 |
| --- | --- | --- |
| 第一档 | 税额≥5万元 或 造成国家税款损失≥3万元 | 三年以下有期徒刑或拘役，并处二万至二十万罚金 |
| 第二档 | 税额≥50万元 或 造成国家税款损失≥30万元 | 三年以上十年以下有期徒刑，并处五万至五十万罚金 |
| 第三档 | 税额≥500万元 或 造成国家税款损失≥300万元 | 十年以上有期徒刑或无期徒刑，并处五万至五十万罚金或没收财产 |

> **注意**：虚开专票主要以"虚开税额"和"造成国家税款损失"为标准，税额计算以发票上注明的税额为准。

### 虚开增值税普通发票罪（刑法第205条之一）

| 量刑档次 | 虚开金额（票面金额） | 或虚开份数 | 对应刑罚 |
| --- | --- | --- | --- |
| 入罪标准 | 金额≥40万元（价税合计） | 或 100份以上 | 二年以下有期徒刑、拘役或管制，并处罚金 |
| 情节严重 | 金额≥200万元（价税合计） | 或 500份以上 | 二年以上七年以下有期徒刑，并处罚金 |

> **注意**：虚开普票以"票面金额"（价税合计）和"份数"为标准，不以税额为标准。

### 分析要求

1. 在报告中必须明确区分专票和普票的涉案金额/税额/份数
2. 分别对照上述标准给出够罪判断及可能适用的量刑档次
3. 如涉案金额较小未达入罪标准，应明确指出"涉案金额未达刑事追诉标准，建议行政处理"
4. 如同时存在虚开专票和普票，需分别计算，不可混合

---

## 虚开模式深度分析要点

分析不仅要识别犯罪类型，还需回答以下深层问题：

### 1. 地域选择逻辑
- 为什么虚开企业注册在这个地区？是否存在税收优惠政策（如经济开发区、招商引资园区税收返还）？
- 资金流向和发票流向的地域是否一致？如本地开票但资金流向外地，需分析外地团伙的可能性
- 交易对手的地域集中特征（如下游客户集中在某市、资金集中流向某省）

### 2. 行业选择逻辑
- 为什么选择这个行业（如技术服务、咨询服务、贸易类）？是否因为该行业难以核实真实交易？
- 技术类/服务类公司虚开的特殊性：无实物流转，难以通过物流验证，仅凭合同和发票即可完成交易形式
- 行业税率特征是否被利用（如利用农产品收购发票的免税/低税政策）

### 3. 开票额度突破手法
- 新开业公司一般只有10万元的开票额度，如何快速提升至百万甚至千万级别？
- 常见手法：虚构交易业绩申请增量/增额、利用多家小额企业分散开票、先正常经营积累信用后突然大量虚开
- 分析涉案企业的开票额度提升时间线（如有数据）

### 4. 团伙运作模式
- 团伙的组织架构和分工（控制人、开票人、介绍人、资金通道）
- 团伙的获利方式（开票费比例通常为票面金额的X%）
- 团伙的风险规避手段（频繁更换企业、使用他人身份注册、资金多层中转）

---

## 可疑指标与阈值

以下是核心可疑指标的计算方法和判定阈值。分析时逐项计算，每项给出具体数值和判定结论。

### 发票基础异常指标

| 序号 | 指标名称 | 规则来源 | 计算方法 | 正常范围 | 异常阈值 | 高度异常 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 进销品名匹配率 | R01 | 进项与销项货品名称大类一致的比例 | > 80% | < 60% | < 30% |
| 2 | 整额开票占比 | R02 | 价税合计为整万的发票笔数 / 总笔数 | < 10% | > 25% | > 40% |
| 3 | 顶额开票占比 | R03 | 接近发票限额的发票笔数 / 总笔数 | < 5% | > 15% | > 30% |
| 4 | 发票作废率 | R05 | 作废发票笔数 / 总开票笔数 | < 5% | > 10% | > 20% |
| 5 | 异地发票占比 | R08 | 异地发票笔数 / 总笔数 | < 20% | > 40% | > 60% |
| 6 | 单日最大开票金额 | R04 | 单日价税合计最大值 | 视企业规模 | 显著超出日常均值 | 超出均值5倍以上 |
| 7 | 月均开票增长率 | R04 | 最近3月均开票金额 / 前期月均开票金额 | < 50% | > 100% | > 300% |

### 虚开犯罪专属指标

| 序号 | 指标名称 | 规则来源 | 计算方法 | 正常范围 | 异常阈值 | 高度异常 |
| --- | --- | --- | --- | --- | --- | --- |
| 8 | 进销金额匹配度 | R14 | abs(销项金额-进项金额) / max(销项金额,进项金额) | > 20% | < 10% | < 3% |
| 9 | 销项对手集中度TOP3 | R07 | TOP3购方金额 / 总销项金额 | < 40% | > 60% | > 80% |
| 10 | 进项对手集中度TOP3 | R07 | TOP3销方金额 / 总进项金额 | < 40% | > 60% | > 80% |
| 11 | 票流闭环数 | R13 | 检测到的A→B→...→A环形票流路径数 | 0 | >= 1 | >= 3 |
| 12 | 设备共享企业团伙数 | R10 | 共享IP/MAC/主板序列号的企业组数 | 0 | >= 1 | >= 3 |
| 13 | 进项先于销项比例 | R14 | 进项发票日期晚于对应销项发票日期的比例 | < 5% | > 15% | > 30% |
| 14 | 对开企业数 | C5 | 互相开票的企业对数 | 0 | >= 1 | >= 3 |
| 15 | 税率混用率 | R06 | 同一货品使用不同税率的笔数占比 | < 3% | > 10% | > 20% |
| 16-a | 经营性支出发票缺失率 | R13 | 进项中水电/运费/办公等经营性支出占比 | > 5% | < 2% | < 0.5% |
| 16-b | 注册至首次开票天数 | R14 | 税务登记日期到首张发票日期的间隔 | > 90天 | < 30天 | < 3天 |
| 16-c | 行业品类匹配率 | R15 | 开票品类与税务登记行业种类匹配比例 | > 80% | < 50% | < 20% |
| 16-d | 开票IP跨省数 | R16 | 开票IP对应不同省份的数量 | 1 | >= 2 | >= 4 |
| 16-e | 月度环比最大波动率 | R17 | max(abs(本月-上月)/上月) | < 100% | > 200% | > 500% |
| 16-f | 原材料成本率 | R18 | 原材料进项金额 / 销项总金额 | 30%-80% | < 15% 或 > 90% | < 5% |
| 16-g | 票面数量一致率 | R12 | 数量完全相同的发票笔数占比 | < 10% | > 30% | > 50% |

### 企业关联异常指标

| 序号 | 指标名称 | 规则来源 | 计算方法 | 正常范围 | 异常阈值 | 高度异常 |
| --- | --- | --- | --- | --- | --- | --- |
| 16 | 同法人控制企业数 | D2 | 同一法人（身份证号）关联的企业数 | 1 | >= 3 | >= 5 |
| 17 | 同财务负责人企业数 | D2 | 同一财务负责人关联的企业数 | 1 | >= 3 | >= 5 |
| 18 | 同办税人企业数 | D2 | 同一办税人关联的企业数 | 1 | >= 3 | >= 5 |
| 19 | 同地址企业数 | D3 | 注册在相同地址的企业数 | 1 | >= 3 | >= 5 |
| 20 | 同电话/邮箱企业数 | D3 | 共用同一电话或邮箱的企业数 | 1 | >= 2 | >= 4 |
| 21 | 空壳企业特征数 | D3 | 满足空壳特征（注册资本极低、无实缴、成立时间短、已注销等）的企业数 | 0 | >= 2 | >= 5 |
| 22 | 非正常户占比 | A | 纳税人状态为"非正常"的企业占涉案企业比例 | 0% | > 20% | > 50% |
| 23 | 注册经营地不一致率 | D4 | 注册地址与生产经营地址不一致的企业占比 | < 10% | > 30% | > 50% |
| 24 | 法人年龄偏大率 | D2 | 法人年龄>=70岁的企业占涉案企业比例 | 0% | > 10% | > 30% |
| 25 | 关联企业经营范围相似度 | D3 | 同一法人/办税人控制企业的经营范围Jaccard相似度 | < 30% | > 60% | > 80% |
| 26 | IP与注册地不符率 | R16 | 开票IP地理位置≠注册地且≠经营地的发票占比 | < 10% | > 30% | > 50% |

### 骗取出口退税专属指标

| 序号 | 指标名称 | 规则来源 | 计算方法 | 正常范围 | 异常阈值 | 高度异常 |
| --- | --- | --- | --- | --- | --- | --- |
| 27 | 货物增值率 | R21 | 出口单价 ÷ 收购单价 | 1-3倍 | > 5倍 | > 10倍 |
| 28 | 货物成品率 | R22 | 成品销售数量 ÷ 原材料购进数量 | 50%-100% | < 30% 或 > 110% | < 20% 或 > 150% |
| 29 | 收汇国与贸易国不一致率 | 骗退指标6 | 收汇国≠出口贸易国的收汇金额占比 | < 20% | >= 50% | >= 80% |
| 30 | 从香港及东南亚结汇占比 | 骗退指标7 | 来自中国香港及东南亚的收汇金额 / 总收汇金额 | < 30% | >= 60% | >= 80% |
| 31 | 企业所得税税负率 | 骗退指标8 | 年度应纳企业所得税税额 ÷ 年度销售收入 | 行业参考值 | 低于行业值50% | 低于行业值80% |
| 32 | 出口敏感商品占比 | 骗退指标2 | 敏感商品出口金额 / 总出口金额 | < 50% | >= 80% | = 100% |
| 33 | 未足额收汇率 | 骗退指标5 | 1 - (收汇金额 ÷ 美元离岸价) | 0% | > 0% | > 20% |
| 34 | 同口岸频繁更换报关行 | 骗退指标9 | 同一年度同一出口口岸委托报关行数 | <= 3 | > 5（且单报关行报关单<5） | > 10 |
| 35 | 报关单尾号跳号率 | 骗退指标3 | 同商品同国别同口岸报关单尾号差额>300的报关单金额占比 | < 20% | >= 60% | >= 80% |
| 36 | 进项涉农收购发票占比 | 骗退指标4 | 进项中农产品收购发票金额 / 总进项金额 | 视行业 | > 50%（非农企业） | > 80%（非农企业） |

### 农产品虚开专属指标

| 序号 | 指标名称 | 规则来源 | 计算方法 | 正常范围 | 异常阈值 | 高度异常 |
| --- | --- | --- | --- | --- | --- | --- |
| 37 | 顶额开票占比（9000-10000区间） | R19/R03增强 | 价税合计在9000-10000元区间的发票笔数 / 总笔数 | < 30% | > 50% | > 80% |
| 38 | 单一农户最大累计开票金额 | R23 | 单一农户（身份证号）年度累计开票金额最大值 | < 100万 | > 500万 | > 2000万 |
| 39 | 农户身份异常率 | R19 | 存在"一证多人""一人多证""号码错误"的农户占比 | 0% | > 5% | > 15% |
| 40 | 收购价格月间波动率 | R20 | 同一品名各月平均收购单价的变异系数(CV) | > 5% | < 2% | < 0.5% |

补充指标（如数据支持，建议也计算）：
- 开票时间集中在非工作时间（22:00-08:00）的发票占比
- 认证时效异常率（开票后当天即认证的比例）
- 企业存续期间内的月均开票密度
- 进项与销项的上下游企业重合度
- 发票连号比例（发票号码连续的占比）
- 上下游企业中"僵尸公司"（无员工社保、无生产设备）占比（来源：技战法序号34，需外部数据）
- "三新"企业出口业务增长异常率：新注册/新备案/新变更法人的出口企业，年度出口增速超过50%（来源：技战法骗退指标1）
- 出口产品高报倍数：出口单价 / 行业核定价格，参考柳篮核定价16元预警价48元、木制家具（床）核定价9097元预警价4549元（来源：技战法骗退指标10）

---

## 犯罪模式识别规则

根据可疑指标的组合，匹配以下涉税犯罪模式。可能同时符合多种模式，逐一判断。

**重要原则**：以下模式分类仅作为分析标签使用，不作为独立的报告模块。在报告中应以企业为单位进行分析，为每家涉案企业标注其虚开类型标签（可多标签），而非按虚开类型分章节写。在法律上，无论是"暴力虚开"还是"变票虚开"都属于虚开增值税发票犯罪，模式分类是为了帮助理解犯罪手法和组织方式。

### 暴力虚开型（企业标签）

**核心特征**：
- 企业成立时间短（通常不超过1-2年），存续时间不足一年或几个月（来源：技战法序号6）
- 短期内大量集中开票，开票金额陡增（来源：技战法"税务异常五"）
- 注册后极短时间内即开始开票（如注册三日内即有开票记录，来源：技战法序号7）
- 发票作废率高（频繁作废重开）
- 顶额开票占比高（来源：技战法"税务异常四"）
- 整额开票占比高
- 纳税人状态已变为"非正常"或"注销"
- 企业无实际经营场所（注册地址为虚拟地址）
- 无对应进项或进项极少（进项税远高于销项税不成立，来源：技战法"税务异常六"）
- 无水电、工资、物流等经营性支出的进项发票（来源：技战法"税务异常七"、序号14-18）
- 法人、财务负责人、办税人可能为同一人或关联人
- 法人年龄偏大（如70岁以上），可能为冒用老人身份（来源：技战法序号8）
- 可能注册在税务政策宽松地区（来源：技战法"税务异常一"）

**典型链条**：
暴力虚开企业 →(开具发票)→ 受票企业 →(抵扣税款)

### 变票虚开型（企业标签）

**核心特征**：
- 进项发票货品名称与销项发票货品名称大类不一致（如进项为"煤炭"、销项为"化工产品"）
- 进销品名匹配率低
- 进项和销项金额大致匹配
- 下游企业行业与销项品名一致（表面合理），但与进项品名不符
- 可能存在多层变票（A→B→C，每一层改变品名）

**与暴力虚开的关键区别**：变票企业通常是有一定正常经营基础的企业（或者伪装成正常经营），其特点是利用进项发票的品名和销项发票品名不一致来实现虚开目的。因此**不能简单地以"无水电支出"判断变票企业为空壳**——变票企业恰恰可能有水电、房租等正常经营支出来维持正常经营的表象。判断变票的核心依据是进销品名不匹配，而非经营性支出缺失。

**典型链条**：
上游供应商 →(品名A发票)→ 变票企业 →(品名B发票)→ 下游受票企业

### 对开/环开虚开型（企业标签）

**核心特征**：
- 存在互相开票的企业对（A开给B、B也开给A）
- 存在环形票流（A→B→C→A）
- 进销金额高度一致（进销匹配度 < 3%）
- 票流闭环数 >= 1
- 企业间开票金额相近、时间相近
- 通常不涉及真实货物交付

**典型链条**：
企业A ⇄ 企业B（对开）
企业A → 企业B → 企业C → 企业A（环开）

### 骗取出口退税型（买单配票型，企业标签）

**核心特征**：
- 存在出口相关的货品名称（如外贸商品），且开票品类退税率高（来源：技战法"税务异常三"）
- 进项集中在少数供应商，且供应商可能为虚开企业或无生产能力的贸易公司（来源：技战法序号36）
- 销项开给外贸公司或自身为进出口企业
- 货品名称中可能出现常见骗税商品（电子产品、服装、农产品等）
- 出口敏感商品占比高（如纺织服装、木制家具、工艺品等36类敏感商品占出口额80%以上，来源：技战法骗退指标2）
- 异地发票占比高
- 上游供应商状态异常（非正常、已注销）、为无生产能力企业或"僵尸公司"（来源：技战法序号34）
- 出口申报单价远高于同期同类商品出口均价（货值倒挂/低价高报，来源：技战法#1/骗退指标10）
- 虚假提高收购发票单价，如自开品名"柳坯"单价达244元/个而市场正常价格仅15-20元，高出市场价12倍以上（来源：技战法#1）
- 货物增值率异常，如出口单价÷收购单价高达6倍以上（来源：技战法#5）
- 货物成品率异常，入账原材料数量远低于出口成品数量，或成品率仅为20%左右远低于正常水平（来源：技战法#6）
- 货源地与报关地不合理，舍近求远选择较远报关地（来源：技战法序号37/技战法#10）
- 报关出口海关非常分散（来源：技战法序号39）
- 固定客户较少，出口口岸、货运代理公司、客户分散，不符合正常企业经营规律（来源：技战法#2）
- 同一关区口岸频繁更换报关行：同一年度同一出口口岸委托报关行>5家且单报关行报关单<5张（来源：技战法骗退指标9）
- 同商品同国别同口岸报关单尾号异常跳号（尾号差额超过300），跳号报关单金额占全部出口金额比例达60%以上（来源：技战法骗退指标3）
- 商品流向不合理（如煤炭从东部卖往产煤区，来源：技战法序号38）
- 收汇国与出口贸易国不一致，如外汇账户90%从香港收汇而出口国为欧美（来源：技战法#3/骗退指标6）
- 从中国香港及东南亚结汇占比超过60%（来源：技战法骗退指标7）
- 未足额收汇，未在规定期限内完成全额收汇（来源：技战法骗退指标5）
- 企业所得税税负率远低于行业正常值（如行业参考1.5%但实际仅0.09%-0.29%），利用虚开发票虚抵所得税（来源：技战法#4/骗退指标8）
- 外贸企业实行"免抵退"政策不太可能出现亏损，若利润与出口报关人民币离岸价倒挂、长亏不倒则重大异常（来源：技战法骗退指标8）
- 结汇规模振幅极大，出口规模年度间剧烈波动（如连年翻倍后骤降再陡增），与正常生产经营企业产能逐步增长的规律不符（来源：技战法#7）
- "三新"企业出口业务增长异常：新注册/新备案/新变更法人股东的出口企业，出口额年度同比增速超过50%（来源：技战法骗退指标1）
- 进项属性来源高风险：进项发票中农产品收购发票占比超50%（来源：技战法骗退指标4）
- 出口企业、供应商、货代公司形成闭合关联网络（来源：技战法序号42）

**敏感商品类别参考表**（来源：国家税务总局《敏感商品类别参考表》，技战法骗退指标2）：

| 海关编码前2-4位 | 敏感商品类别 |
| --- | --- |
| 61、62 | 纺织服装（防寒服、针织衫、化纤服装、布等） |
| 9401、9403 | 木制家具 |
| 44、46 | 木制工艺品 |
| 0504、3001、2005、0308、0712、0710、2003 | 肠衣及其制品、什锦菜、海参、香菇 |
| 8517 | 手机、智能手表 |
| 8471 | 平板电脑 |
| 8544、8504、9032、8541 | 贵金属制品 |
| 8517（6299）、8537、8486 | 转换器、控制器、溅射靶材 |

**出口产品价格预警参考表**（来源：技战法骗退指标10）：

| 出口产品 | 核定价格（元） | 预警价格（元） | 说明 |
| --- | --- | --- | --- |
| 柳篮 | 16 | 48 | 核定价×3 |
| 木片篮 | 25 | 75 | 核定价×3 |
| 草篮 | 16 | 48 | 核定价×3 |
| 木制家具（床） | 9097.06 | 4548.53 | 已认定高报价÷2 |
| 木制家具（桌子） | 7589.60 | 3794.80 | 已认定高报价÷2 |
| 梳妆台 | 5262.08 | 2631.04 | 已认定高报价÷2 |

**典型链条**：
虚开供应商 →(增值税专用发票)→ 外贸企业/中间商 →(出口退税申报)→ 骗取退税款

### 团伙作案型（企业标签）

**核心特征**：
- 多个企业共享同一设备开票（IP/MAC/主板序列号），同一MAC地址对应多台相同税控设备（来源：技战法序号22/46）
- 多个企业共用同一法人、财务负责人或办税人（来源：技战法序号4/12）
- 关联企业法人、监事相同或互为法人/监事（来源：技战法序号4）
- 多个企业注册在同一地址（来源：技战法序号2），共用同一电话、邮箱（来源：技战法序号1/3）
- 企业间形成稳定的上下游开票关系
- 团伙内企业互相开票，虚增进项
- 成员企业多为近期新注册，经营范围雷同（来源：技战法序号5）
- 法人年龄偏大、名下资产状况较弱（来源：技战法序号8/"人员异常一"）
- 存在大额频繁的公转私转账（来源：技战法序号49，需银行流水数据辅助验证）

**典型链条**：
团伙控制人 → 注册多个空壳企业A/B/C/D → 企业间互相开票 → 对外向受票企业开票 → 受票企业抵扣税款

### 走逃失联型（企业标签）

**核心特征**：
- 纳税人状态为"非正常"（走逃失联）
- 开票后短期内走逃
- 开票金额在走逃前集中爆发
- 法人/办税人联系电话无法联系
- 注册地址为虚假地址
- 无正常的进项取得

### 农产品虚开型（收购发票虚开型，企业标签）

**核心特征**：
- 企业经营范围涉及农产品收购、加工
- 进项发票中农产品收购发票金额明显高于市场价格（来源：技战法序号47/技战法#1）
- 向大量个人代开农产品收购发票，其中特定籍贯自然人占比高（来源：技战法#9）
- 顶额开票占比极高，如价税合计9000-10000元的发票占总票数的80%以上（来源：技战法#17）
- 单一农户累计开票金额巨大，与个体农户正常生产体量明显不符（如单一农户年开票达数千万元，来源：技战法#18）
- 开票方和受票方关系紧密：开票"农户"实为受票方公司员工，或存在员工提供家属、同乡人员信息作为虚假"农户"（来源：技战法#19）
- 进项票据造假：农户身份证号出现"一证多人""一人多证"、身份证号码字段逻辑错误（如出现不合理出生年份）、持证人年龄与种植/养殖主体不符（如19岁的中草药种植户）（来源：技战法#16）
- 收购价格无波动特征：在不同时间、不同地点、不同农户处收购的同一品种价格完全一致，违背畜牧业/农业生产和市场规律（来源：技战法#20）
- 存在"资金回流闭环"：受票方资金→加工企业账户→关联个人账户（伪装为"农户"）→回流至受票方实际控制人（来源：技战法序号48，需银行流水辅助验证）
- 虚进虚出模式：上游开票企业将资金转入上上游原材料加工企业，通过上上游企业关联个人账户将资金回流至骗税团伙，各环节留存部分资金作为开票费（来源：技战法"三"资金回流部分）
- 上游为关联企业或"僵尸公司"，无实际生产能力（来源：技战法序号34）
- 注册在农业政策宽松地区（来源：技战法"税务异常一"）
- 进项发票中原材料成本率异常（来源：技战法序号30）
- 使用大量现金或第三方支付逃避银行监管（来源：技战法序号45，需银行流水辅助验证）
- 舍近求远采购：进项票据中存在大量异地企业开具的票据，增加运输成本但无对应运输发票（来源：技战法#10）
- 进销项票据不匹配：大量销项无对应进项票据，缺少必要的原材料进项（来源：技战法#13）

**典型链条**：
虚假"农户"/关联人 →(农产品收购发票)→ 加工企业 →(增值税专用发票)→ 受票企业 →(资金回流至关联人)

匹配到模式后，在报告中按"涉税犯罪票流-企业-人员-资金四维模型"描述犯罪运作模式：
1. **票流维度**：发票从哪里来、到哪里去，品名如何变化、金额如何传递
2. **企业维度**：涉案企业的注册特征、经营状态、存续时间、行业分布
3. **人员维度**：实际控制人、法人代表、财务负责人、办税人之间的关联关系
4. **资金维度**：资金流向、回流路径、开票费结算方式（如有银行交易数据）

---

## 常见问题处理

分析过程中遇到问题时参考以下指引，不要卡住。

### 数据列名或表名不匹配

由于 `excelcli import` 会按原始文件和工作表导入，实际原始表名通常是 `文件名__工作表名`。因此必须先通过 `excelcli tax map` 和 `excelcli tax normalize` 建立标准表/视图，再执行后续 SQL。

建议流程：

1. 先列出全部表：

```sql
SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;
```

2. 查看候选表列名：

```sql
PRAGMA table_info(候选表名);
```

3. 用 `excelcli tax map` 生成字段映射，再用 `excelcli tax normalize` 创建 `seller_invoice`、`buyer_invoice`、`tax_registration`、`business_registration` 等标准表/视图。

4. 若无法识别到某类表：跳过该类分析项，并在报告明确说明。

> 不再依赖 `_raw_` 前缀列或固定映射表；以当前数据库真实列名为准。

### 分析结果异常

排查步骤：先用 `excelcli inspect` 核对原始表字段，再查看 `tax-mapping.json` 和 `import_issues`；必要时调整映射后重新执行 `excelcli tax normalize`。


### SQLite 参数绑定报错（中文列名常见）

错误根因：把 `:param` 用在表名/列名位置。

- 正确：`WHERE "纳税人识别号" = :tpid`
- 错误：`SELECT * FROM :table`、`SELECT :column FROM seller_invoice`

处理规则：
1. 参数绑定仅用于值
2. 动态表名/列名必须白名单校验
3. 通过双引号转义标识符后再拼接 SQL

1. `SELECT * FROM table_name LIMIT 3;` — 确认数据长什么样
2. `PRAGMA table_info(table_name);` — 确认列名和数据类型
3. `SELECT DISTINCT 列名 FROM table_name LIMIT 20;` — 确认列的实际取值
4. 根据以上信息调整 SQL

常见问题：
- 金额列为 TEXT：用 `CAST(列名 AS REAL)` 转数值
- 日期列格式不统一：用 `substr(日期列, 1, 7)` 提取年月，用 `substr(日期列, 1, 10)` 提取日期
- 空值已填充为空字符串：用 `!= ''` 而非 `IS NOT NULL` 过滤

### 需要外部数据辅助验证的规则

技战法中部分规则需要当前五张表以外的数据才能完整分析，在报告的"外部数据调取建议"章节中应明确列出这些数据需求。如果用户已经提供了银行交易明细，则在阶段E中直接分析，不需列为外部数据需求：
- **银行流水数据（如未提供）**：验证资金回流闭环（技战法序号48）、公转私异常转账（技战法序号49）、付款人与发票购方一致性（技战法序号43）；运用"八步回流战法"追踪资金回流路径：①快速筛回流（两库碰撞）②错位找回流（发票明细与个人对手账户比对）③反推充回流（反向推导资金流向）④凑点对回流（比对手续费比例）⑤循线追回流（追踪资金交汇点）⑥跨区查回流（银行卡归属地信息筛选）⑦断点续回流（现金回流路径查找）⑧中介连回流（中介参与资金传递揭示）（来源：技战法"三"资金回流部分）
- **海关数据**：报关单/提单/发票三单一致性核查（技战法序号40）、出口数据比对（技战法序号41）、报关地分散度分析（技战法序号39）；报关单尾号跳号检测（同商品同国别同口岸报关单尾号差额>300，来源：技战法骗退指标3）；同口岸频繁更换报关行检测（来源：技战法骗退指标9）；出口产品单价高报检测（来源：技战法骗退指标10）
- **外汇数据**：收汇国与出口贸易国一致性核查、未足额收汇检测、从中国香港及东南亚结汇占比分析（来源：技战法骗退指标5/6/7）；结汇规模年度振幅分析（来源：技战法#7）
- **物流数据**：验证货物运输真实性、货源地与报关地合理性（技战法序号37）；核查是否存在"舍近求远"采购但无对应运输发票的情况（来源：技战法#10）
- **社保数据**：验证企业是否有实际员工、法人社保缴纳地与注册地一致性（技战法序号10/34）
- **用电/用水数据**：辅助判断企业是否有实际生产经营（技战法序号14/18）
- **境外企业信息**：买方资质核查，是否为境外新注册空壳公司或虚拟办公场所（技战法序号44）
- **纳税申报数据**：企业所得税申报表用于计算企业所得税税负率（年度应纳税额÷年度销售收入），核查是否远低于行业正常值（来源：技战法#4/骗退指标8）；增值税申报表附表二用于提取涉农发票占比（来源：技战法骗退指标4）
- **经营异常名录数据**：核查涉案企业是否因未报送年度报告被列入经营异常名录或存在失信行为（来源：技战法#15）
- **行业协会/市场调研数据**：获取同行业同品名商品的市场参考价格，用于单价异常检测和高报识别（来源：技战法#1/骗退指标10）

### 分析结果不符合预期

- **进销金额严重不匹配**：检查是否只提供了部分数据（如只有销方数据没有购方数据），在报告中说明数据限制
- **货品名称全为空**：可能数据脱敏或提取不完整，尝试从商品编码推断品名大类
- **团伙划分结果只有1个大团伙**：检查是否有某个IP/MAC被大量企业共享（如税务代理的公共IP），考虑过滤掉关联企业数超过20的IP
- **票流闭环过多**：可能金额匹配阈值过宽，收紧金额容差比例和时间窗口
- **企业关联过多**：可能某个地址或电话是税务代理/财务公司，需排除中介机构的影响

### 数据量过少（<100张发票）

- 月度趋势改为按周统计
- 对手集中度分析可能意义有限，改为逐一列出所有对手
- 设备关联分析简化为直接列出共享设备的企业
- 票流闭环检测适当放宽时间窗口

---

## 报告结构

撰写正式报告时，优先读取并遵循同目录的 [report-template.md](report-template.md)。


| 章节 | 核心内容 | 数据来源 |
| --- | --- | --- |
| 一、基本情况 | 涉案企业信息(税务登记+工商) + 发票概况（区分专票/普票） + 数据结构特点 | 阶段A + 税务登记 + 工商信息 |
| 二、涉案企业逐户分析 | 以企业为单位逐户分析：企业画像 + 虚开类型标签 + 开票特征 + 时间转折点 + 关联关系 | 阶段A-F |
| 三、虚开网络与票流穿透 | 上下游关系 + 进销匹配 + 票流闭环 + 资金流分析 + 交易对手地域特征 | 阶段C + 阶段E |
| 四、团伙识别与犯罪模式 | 团伙划分 + 犯罪运作模式（票流/企业/人员/资金四维模型） + 虚开模式深度研判（地域/行业/政策/手法） | 阶段D + 阶段G |
| 五、够罪条件分析 | 分专票/普票分别计算涉案金额/税额/份数 + 对照法律标准 + 量刑档次判断 | 阶段G |
| 六、可疑点汇总 | 精炼列出关键可疑特征及数据支撑（合并同类项，避免冗余） | 阶段H |
| 七、下一步工作建议 | 立即措施 + 深度调查 + 外部数据调取建议 | 全部阶段 |
| 八、结论 | 核心发现 + 涉案规模 + 处置建议 | 全部阶段 |

**报告撰写原则**：
1. **以企业为单位组织分析**，而非以分析维度为单位。每家企业打上虚开类型标签，在同一段落中完成对该企业的全面刻画
2. **精炼不冗余**：对于上下游排名等统计，只需在表格中呈现，不需单独展开篇幅逐一描述（除非发现了特别值得深入分析的异常点）
3. **结论导向**：每段分析都要有明确的分析结论，避免纯数据堆砌
4. **区分专票/普票**：涉案金额统计必须分开呈现

**输出数据格式规范**：
1. 结构化数据（企业信息表、发票统计表等）以规范的Markdown表格输出
2. 避免在JSON中嵌入大段非结构化文本，如需输出JSON格式的中间数据，确保每个字段都是结构化的（数值、日期、分类标签等）
3. 报告正文以Markdown格式输出，内嵌表格辅助数据呈现



**落盘要求（新增）**：
1. 逐户报告输出到 `输出目录/reports/`
2. 汇总报告输出为 `输出目录/summary-report.md`
3. 输出目录示例：`output/tax-fraud-analysis-0326/`
4. 若用户指定了目录，严格按用户目录输出
