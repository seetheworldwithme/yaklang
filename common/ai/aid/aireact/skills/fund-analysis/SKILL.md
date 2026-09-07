---
name: fund-analysis
description: "对银行账户交易数据进行可疑交易资金分析，生成专业资金分析报告。适用于反洗钱、非法换汇、电信诈骗、地下钱庄等场景。数据读取、入库、标准化、筛选和基础统计必须使用 excelcli，本技能保留原始技战法、分析流程、指标体系和报告要求。"
---

# 资金交易分析

## excelcli 工具使用指南（强制）

本技能的所有数据操作必须通过 `excelcli` 完成，禁止智能体自行用 Python/pandas 读取 Excel 或自建入库流程。

### excelcli 工作流总览

excelcli 采用 **导入 → 检查 → 映射 → 标准化 → 分析** 的五步工作流：

```
Excel/CSV 文件  →  import     →  SQLite 数据库
                      ↓
                  inspect    →  确认表结构和列名
                      ↓
                  map        →  字段映射 JSON
                      ↓
                  normalize  →  bank_transactions 标准表
                      ↓
         analyze fund / transactions / query  →  分析结果
```

涉税数据有独立的子工作流：

```
数据库  →  tax map        →  涉税映射 JSON
              ↓
         tax normalize   →  标准涉税表
              ↓
   tax analyze / tax invoices  →  分析结果
```

### 命令速查表

| 命令 | 用途 | 必填参数 |
|---|---|---|
| `excelcli import <INPUT> --case-id <ID> --db <DB>` | 导入 Excel/CSV 到 SQLite | INPUT, --case-id, --db |
| `excelcli inspect <DB>` | 查看表结构和样例数据 | DB |
| `excelcli map <DB>` | 生成银行流水字段映射 | DB |
| `excelcli normalize <DB> --mapping <FILE>` | 标准化为 bank_transactions 表 | DB, --mapping |
| `excelcli analyze fund <DB>` | 基础资金分析 | DB |
| `excelcli query <DB> --sql <SQL>` | 执行自定义 SQL 查询 | DB, --sql 或 --file |
| `excelcli transactions <DB>` | 交易流水多维筛选 | DB |
| `excelcli tax map <DB>` | 涉税表自动识别与映射 | DB |
| `excelcli tax normalize <DB> --mapping <FILE>` | 涉税表标准化 | DB, --mapping |
| `excelcli tax analyze <DB>` | 涉税风险概览分析 | DB |
| `excelcli tax invoices <DB>` | 发票明细筛选 | DB |

### 银行流水分析完整步骤（按顺序执行）

**Step 1 — 导入数据**

```bash
excelcli import 交易流水.xlsx --case-id case001 --db case001.db
```

- `<INPUT>`：输入文件路径，支持 .xls / .xlsx / .csv
- `--case-id`：案件编号，用于区分不同案件
- `--db`：SQLite 数据库输出路径
- 可选：`--verify`（导入后校验）、`--encoding gbk`（指定 CSV 编码）
- 如果有多个 Excel 文件，逐个 import 到同一个 `--db` 即可

**Step 2 — 检查表结构**

```bash
excelcli inspect case001.db --json --sample 5
```

- 确认导入的表名、列名、数据样例
- 这一步**必须执行**，因为不同银行导出的列名差异很大，后续分析依赖正确的列名
- 输出 JSON 格式方便程序化处理

**Step 3 — 生成字段映射**

```bash
excelcli map case001.db --out mapping.json
```

- 自动识别原始表列名与标准字段的对应关系
- 输出映射 JSON 文件，供 normalize 使用
- 可选：`--json` 直接在终端查看映射结果

**Step 4 — 数据标准化**

```bash
excelcli normalize case001.db --mapping mapping.json
```

- 根据映射文件，将原始表标准化为 `bank_transactions` 标准表
- 标准化后，所有后续分析都基于 `bank_transactions` 表进行

**Step 5 — 基础资金分析**

```bash
excelcli analyze fund case001.db
```

- 对标准化后的银行流水进行基础资金分析
- 可选：`--json` 输出 JSON 格式

**Step 6 — 交易筛选**

```bash
excelcli transactions case001.db --account "622848123456" --min-amount 10000 --json
excelcli transactions case001.db --counterparty "张三" --start 2024-01-01 --end 2024-06-30 --csv
excelcli transactions case001.db --direction 支出 --keyword "转账" --limit 200
```

筛选参数说明：
- `--account`：按账号筛选
- `--counterparty`：按对手方名称筛选
- `--direction`：按交易方向（如 收/支）
- `--min-amount` / `--max-amount`：金额范围
- `--start` / `--end`：日期范围
- `--keyword`：关键词模糊搜索
- `--limit`：返回行数上限（默认 100）
- `--csv`：CSV 格式输出，方便导入其他工具

**Step 7 — 自定义 SQL 查询**

```bash
excelcli query case001.db --sql "SELECT * FROM bank_transactions LIMIT 10"
excelcli query case001.db --sql "SELECT account, COUNT(*) as cnt, SUM(amount) as total FROM bank_transactions GROUP BY account ORDER BY total DESC" --limit 50 --json
excelcli query case001.db --file analysis.sql --limit 500 --csv
```

- `--sql` 和 `--file` 二选一
- `--limit`：返回行数上限（默认 100）
- 所有分析指标的计算都应通过 `excelcli query` 执行 SQL 完成

### 涉税分析步骤（如数据涉及涉税场景）

```bash
# Step 1: 导入（同上）
excelcli import 发票数据.xlsx --case-id tax001 --db tax001.db

# Step 2: 涉税映射
excelcli tax map tax001.db --out tax_mapping.json

# Step 3: 标准化
excelcli tax normalize tax001.db --mapping tax_mapping.json

# Step 4: 风险分析
excelcli tax analyze tax001.db --top 20

# Step 5: 发票筛选
excelcli tax invoices tax001.db --taxpayer "某某公司" --min-amount 100000 --csv
```

涉税分析统一操作以下标准表：`tax_invoices`、`seller_invoice`、`buyer_invoice`、`tax_registrations` 等。

### bank_transactions 标准表列名（必须掌握）

`normalize` 完成后，原始中文列名会被映射为以下**英文标准列名**。后续所有 `excelcli query --sql` 查询必须使用这些列名，**不要使用原始中文列名**，否则会报 `no such column` 错误。

| 标准列名 | 含义 | 对应原始字段 | 备注 |
| --- | --- | --- | --- |
| `id` | 自增主键 | - | INTEGER PK |
| `case_id` | 案件编号 | _case_id | TEXT |
| `source_file` | 来源文件 | _source_file | TEXT |
| `sheet_name` | 工作表名 | _sheet_name | TEXT |
| `row_no` | 原始行号 | _row_no | INTEGER |
| `bank_name` | 银行名称 | - | TEXT, 可能为空 |
| `account_no` | 交易卡号 | 交易卡号 | TEXT |
| `account_name` | 交易户名 | 交易户名 | TEXT |
| `account_id_no` | 交易证件号 | 交易证件号 | TEXT, 可能为空 |
| `account_type` | 账户类型 | - | TEXT, 可能为空 |
| `txn_time` | 交易时间 | 交易时间 | TEXT, 格式 YYYY-MM-DD HH:MM:SS |
| `direction` | 收付方向 | 收付标志 | TEXT, 值为 "in"/"out" |
| `amount` | 交易金额 | 交易金额 | REAL |
| `balance` | 交易余额 | 交易余额 | REAL |
| `counterparty_account` | 对手账卡号 | 交易对手账卡号 | TEXT, 可能为空字符串 |
| `counterparty_name` | 对手户名 | 对手户名 | TEXT, 可能为空字符串 |
| `counterparty_id_no` | 对手证件号 | 对手证件号 | TEXT, 可能为空 |
| `counterparty_bank` | 对手开户银行 | 对手开户银行 | TEXT, 可能为空 |
| `summary` | 摘要说明 | 摘要说明 | TEXT |
| `channel` | 现金标志/交易渠道 | 现金标志 | TEXT, 常见值 "现金交易"/"其它" |
| `location` | 交易发生地 | 交易发生地 | TEXT, 可能为空 |
| `ip` | IP地址 | IP地址 | TEXT, 可能为空 |
| `mac` | MAC地址 | MAC地址 | TEXT, 可能为空 |
| `currency` | 交易币种 | 交易币种 | TEXT, 可能为空 |
| `txn_serial_no` | 交易流水号 | 交易流水号 | TEXT, 可能为空 |
| `voucher_no` | 凭证号 | 凭证号 | TEXT, 可能为空 |
| `is_success` | 是否成功 | 交易是否成功 | INTEGER, 1=成功 |
| `raw_table` | 原始表名 | - | TEXT |
| `raw_row_id` | 原始行ID | - | INTEGER |
| `dedup_key` | 去重键 | - | TEXT |

**关键易错点**：
- 交易时间列名是 `txn_time`，不是 `transaction_time`
- 收付方向列名是 `direction`，不是 `收付标志`，且值为 `"in"` / `"out"`（不是"进"/"出"）
- 现金标志列名是 `channel`，不是 `cash_flag`
- 余额列名是 `balance`，不是 `transaction_balance`
- 对手账号列名是 `counterparty_account`，空值为空字符串 `""`，不是 NULL
- `amount` 为 REAL 类型，可以直接用于数值计算和聚合
- 无对手信息的交易：`counterparty_account = ""` 且 `counterparty_name = ""`

**normalize 后的必备步骤**：执行 `PRAGMA table_info(bank_transactions)` 确认实际列名，避免 SQL 报错：

```bash
excelcli query <DB> --sql "PRAGMA table_info(bank_transactions)" --json
```

### 完整分析所需的 SQL 查询集（可直接复用）

以下是在 `analyze fund` 基础上，需要通过 `excelcli query` 补充执行的 SQL 查询。**注意：所有查询均使用上方标准列名**。

**1. 年度交易趋势（含对手主体数）**
```sql
SELECT strftime('%Y', txn_time) as year, direction,
       COUNT(*) as cnt, ROUND(SUM(amount),2) as total,
       COUNT(DISTINCT counterparty_account) as counterparties
FROM bank_transactions WHERE amount > 0
GROUP BY year, direction ORDER BY year, direction
```

**2. 现金交易统计**
```sql
SELECT channel, direction, COUNT(*) as cnt, ROUND(SUM(amount),2) as total
FROM bank_transactions WHERE channel IS NOT NULL AND channel != ''
GROUP BY channel, direction ORDER BY channel, direction
```

**3. 余额特征**
```sql
SELECT ROUND(MAX(balance),2) as max_balance,
       ROUND(MIN(balance),2) as min_balance,
       ROUND(AVG(balance),2) as avg_balance
FROM bank_transactions WHERE balance IS NOT NULL
```

**4. 最终余额**
```sql
SELECT ROUND(balance,2) as final_balance FROM bank_transactions ORDER BY id DESC LIMIT 1
```

**5. 同名账户识别（对手户名 = 主体户名）**
```sql
SELECT counterparty_account, direction, COUNT(*) as cnt, ROUND(SUM(amount),2) as total
FROM bank_transactions
WHERE counterparty_name = (SELECT account_name FROM bank_transactions LIMIT 1)
  AND counterparty_account != '' AND counterparty_account != account_no
GROUP BY counterparty_account, direction ORDER BY total DESC
```

**6. 双向交易对手**
```sql
SELECT counterparty_account,
       COUNT(DISTINCT direction) as dir_cnt,
       GROUP_CONCAT(DISTINCT direction) as directions,
       COUNT(*) as cnt, ROUND(SUM(amount),2) as total
FROM bank_transactions
WHERE counterparty_account != '' AND counterparty_account != account_no
GROUP BY counterparty_account HAVING dir_cnt > 1 ORDER BY total DESC
```

**7. 自身转账（对手账号 = 主体账号）**
```sql
SELECT direction, COUNT(*) as cnt, ROUND(SUM(amount),2) as total
FROM bank_transactions WHERE counterparty_account = account_no
GROUP BY direction
```

**8. 交易天数统计**
```sql
SELECT COUNT(DISTINCT date(txn_time)) as trading_days FROM bank_transactions WHERE amount > 0
```

**9. 整万交易统计**
```sql
SELECT COUNT(*) as cnt FROM bank_transactions
WHERE CAST(amount AS INTEGER) = amount AND CAST(amount AS INTEGER) % 10000 = 0 AND amount >= 10000
```

**10. 快进快出天数（同日进出均>5万）**
```sql
SELECT date(txn_time) as d,
       SUM(CASE WHEN direction='in' THEN amount ELSE 0 END) as in_amt,
       SUM(CASE WHEN direction='out' THEN amount ELSE 0 END) as out_amt
FROM bank_transactions WHERE amount > 0
GROUP BY d HAVING in_amt > 50000 AND out_amt > 50000
```

**11. 可疑关键词交易**
```sql
SELECT counterparty_account, counterparty_name, direction, COUNT(*) as cnt, ROUND(SUM(amount),2) as total
FROM bank_transactions
WHERE summary LIKE '%换汇%' OR summary LIKE '%换钱%' OR summary LIKE '%换币%'
   OR summary LIKE '%兑换%' OR summary LIKE '%换美金%'
GROUP BY counterparty_account, counterparty_name, direction
```

**12. 非工作时间交易（22:00-08:00）**
```sql
SELECT COUNT(*) as cnt, ROUND(SUM(amount),2) as total
FROM bank_transactions
WHERE CAST(strftime('%H', txn_time) AS INTEGER) >= 22
   OR CAST(strftime('%H', txn_time) AS INTEGER) < 8
```

**13. 同名多户控制人（同一户名控制多个账户）**
```sql
SELECT counterparty_name, COUNT(DISTINCT counterparty_account) as accounts,
       GROUP_CONCAT(DISTINCT counterparty_account) as account_list
FROM bank_transactions
WHERE counterparty_name IS NOT NULL AND counterparty_name != ''
  AND counterparty_account != '' AND counterparty_account != account_no
GROUP BY counterparty_name HAVING accounts >= 2 ORDER BY accounts DESC
```

**14. 收入/支出端对手账户数**
```sql
SELECT direction, COUNT(DISTINCT CASE WHEN counterparty_account != '' AND counterparty_account != account_no THEN counterparty_account END) as unique_counterparties
FROM bank_transactions GROUP BY direction
```

**15. 对手开户银行分布**
```sql
SELECT counterparty_bank, COUNT(DISTINCT counterparty_account) as accounts,
       COUNT(*) as cnt, ROUND(SUM(amount),2) as total
FROM bank_transactions
WHERE counterparty_bank IS NOT NULL AND counterparty_bank != ''
GROUP BY counterparty_bank ORDER BY accounts DESC
```

### 重要注意事项

1. **所有数据操作必须通过 excelcli 完成**：禁止智能体自行用 Python/pandas 读取 Excel、自建入库流程、或编写 Python 脚本进行任何数据分析。所有分析指标和统计计算均通过 `excelcli query --sql` 执行 SQL 完成，团伙划分等复杂分析也不例外
2. **必须先 inspect 再分析**：不同银行的数据格式差异很大，先 inspect 确认列名和数据格式
3. **必须先 map 再 normalize**：normalize 依赖映射文件，map 生成的 JSON 文件是桥梁
4. **标准化后使用英文标准列名**：`bank_transactions` 表使用上方列名对照表中的英文名，SQL 查询中禁止使用中文列名
5. **normalize 后先查 PRAGMA**：执行 `PRAGMA table_info(bank_transactions)` 确认实际列名，然后再写 SQL 查询，避免反复报错
6. **复杂统计用 query**：`analyze fund` 提供基础分析（overview、月度趋势、金额分布、TOP来源/去向、集中度、部分可疑指标），自定义统计指标用上方 SQL 查询集补充
7. **数据异常排查**：如果分析结果异常，先用 `excelcli inspect` 核对列名和样例数据，再检查映射文件是否正确，必要时调整映射后重新 `normalize`
8. **SQL 查询可并行**：上述15条 SQL 查询相互独立，可以分批并行执行（每批3-4条），提高分析效率

---

## 数据文件说明

典型的资金分析数据包含以下四类文件（文件名可能不同，以实际列名为准）：

### 1. 交易明细（主表）

核心数据文件，每条记录为一笔交易，包含收付双方完整信息。所有分析的主要数据来源。

**标准字段**：交易卡号、交易账号、交易户名、交易证件号、交易时间、交易金额、交易余额、收付标志（进/出）、交易对手账卡号、对手户名、对手证件号、对手开户银行、摘要说明、交易币种、交易网点名称、交易发生地、交易是否成功、传票号、IP地址、MAC地址、对手交易余额、交易流水号、凭证号、交易柜员号、现金标志、备注

### 2. 人员信息（辅助表）

涉案客户的身份和背景信息，用于补充对手身份背景、工作单位等。

**标准字段**：客户名称、证照类型、证照号码、单位地址、单位电话、工作单位、邮箱地址、代办人姓名、代办人证件类型、代办人证件号码、国税纳税号、地税纳税号、法人代表、客户工商执照号码

### 3. 账户信息（辅助表）

涉案账户的开户属性和状态信息，用于报告"账户基本信息"章节。

**标准字段**：账户开户名称、开户人证件号码、交易卡号、交易账号、账号开户时间、账户余额、币种、开户网点代码、开户网点、账户状态、账户类型、账号开户银行

### 4. 子账户信息（辅助表）

涉案账户的子产品/子类别信息（如定期、理财、贷款等），用于展示账户的完整资金结构。

**标准字段**：银行名称、开户账号、子账户账号、余额、可用余额、子账户类别、币种、钞汇标识、账户状态

> **文件匹配原则**：用户提供的文件可能名称不同，但通过列名可以识别文件类型。优先匹配交易明细，再通过"交易卡号/交易户名/证照号码"等关键字段关联辅助表。

## 分析框架

分析分为四个阶段，按顺序执行。每个阶段完成后检查产出是否齐全再进入下一阶段。

### 阶段A：数据探查与交易画像

**目标**：了解账户全貌，建立基础数据认知。

**操作**：使用 `excelcli import` 导入数据，`excelcli inspect` 确认列名和数据格式，`excelcli map` + `normalize` 完成标准化，然后通过 `excelcli analyze fund` 和 `excelcli query --sql` 计算以下指标。

**首先检查账户数量**：读取交易明细后，先统计"交易卡号"列的唯一值数量，确认数据中涉及几个主体账户。如果只有1个账户则为单账户分析；如果有多个账户，需在报告开头列出所有涉案账户，后续分析分别或合并进行（取决于业务需求）。

**数据预处理**：
1. 过滤失败交易：如"交易是否成功"列存在，仅保留值为1（成功）的记录，并在报告中注明过滤了多少笔失败交易
2. 交易时间、金额、方向等字段由 `excelcli normalize` 标准化；如结果异常，先用 `excelcli inspect` 核查字段映射是否正确，检查映射 JSON 文件中对应字段的映射关系
3. 收付方向通常在"收付标志"列，值为"进"/"出"，先确认具体取值
4. 使用 `excelcli inspect <db路径> --json --sample 5` 查看真实列名和样例数据
5. 关联辅助表：通过"交易卡号"或"交易户名"关联账户信息和人员信息，补充账户属性和人员背景

**必须产出的指标**：
- 涉及主体账户数量（单账户/多账户）
- 交易笔数、交易总额、时间跨度（首尾日期和天数）
- 收入/支出分别的笔数和金额
- 收支平衡度（差额/总额）
- 年度交易趋势（按年分组的进出笔数、金额、以及每年进出分别涉及多少个对手主体）
- 金额分布（按 0-1千/1千-5千/5千-1万/1万-5万/5万-10万/10万-50万/50万以上 共7个区间，分别统计进出笔数）
- 现金交易笔数、金额、占比（如数据中有"现金标志"列）
- 余额特征：最高/最低/平均/最终余额（如数据中有"交易余额"列）
- 日均交易笔数
- **交易地理分布**：按"交易发生地"统计交易笔数和金额，识别主要交易地区（如数据中有该字段）
- **对手开户银行分布**：按"对手开户银行"统计对手账户数，识别资金主要流向的银行机构（如数据中有该字段）
- **账户开户信息**：从账户信息表汇总所有涉案账户的开户时间、开户网点、账户类型、账户状态等（如提供了账户信息文件）
- **子账户结构**：从子账户信息表展示涉案账户的子产品分布和资金分布（如提供了子账户信息文件）

### 阶段B：资金来源去向分析

**目标**：识别核心交易对手，刻画资金流向模式。

**必须产出的指标**：
- 收入端涉及多少个不同对手账户
- 支出端涉及多少个不同对手账户
- TOP10资金来源表：对手账户、对手户名、对手证件号（用于关联人员信息）、交易笔数、总金额、平均单笔、特征标注
- TOP10资金去向表：同上
- 资金来源集中度：TOP1 / TOP3 / TOP5 占总收入的百分比
- 资金去向集中度：TOP1 / TOP3 / TOP5 占总支出的百分比
- 同名账户识别：对手户名与主体账户户名相同的账户列表及合计金额
- 双向交易对手：既有收入又有支出的对手账户
- 自身转账：对手账户号等于主体账户号的交易统计

**分析要点**：
- 对手信息分布在"交易对手账卡号""对手户名""对手证件号""对手开户银行"等列中
- 可通过"对手证件号"关联人员信息表，获取对手的工作单位、单位地址等背景信息，用于报告中的对手身份描述
- 无对手信息的交易（NaN）单独统计，通常是现金存取
- TOP10表中的"特征"列需人工标注，如"同名账户""高频转入""最大流出""双向交易"等

### 阶段C：团伙划分

**目标**：通过设备指纹、地域关联、开户集中度和交易关系识别可能的团伙网络。

**子任务C1 — 基于设备信息的团伙划分**：

IP/MAC地址在交易明细中直接可用，无需单独的对手数据文件。以下所有操作均通过 `excelcli query --sql` 完成，禁止编写 Python 脚本。

**关键原则**：只有在出账方向（direction='out'）上共享同一IP/MAC的账户才视为设备关联。

**步骤1 — 查找出账方向共享同一IP的账户组**：
```sql
SELECT ip, GROUP_CONCAT(DISTINCT account_no) as accounts,
       COUNT(DISTINCT account_no) as account_count,
       COUNT(*) as txn_count, ROUND(SUM(amount),2) as total_amount
FROM bank_transactions
WHERE direction='out' AND ip IS NOT NULL AND ip != ''
GROUP BY ip HAVING account_count >= 2
ORDER BY account_count DESC
```

**步骤2 — 查找出账方向共享同一MAC的账户组**：
```sql
SELECT mac, GROUP_CONCAT(DISTINCT account_no) as accounts,
       COUNT(DISTINCT account_no) as account_count,
       COUNT(*) as txn_count, ROUND(SUM(amount),2) as total_amount
FROM bank_transactions
WHERE direction='out' AND mac IS NOT NULL AND mac != ''
GROUP BY mac HAVING account_count >= 2
ORDER BY account_count DESC
```

**步骤3 — 在报告中归纳团伙**：根据上述 SQL 查询结果，将共享同一 IP 或 MAC 的账户归为同一团伙。如果两个 IP/MAC 查询结果中有重叠账户，手动合并为同一团伙。在报告中列出每个团伙的成员账户、共享设备信息、与主体账户的交易金额汇总

**子任务C2 — 基于对手证件号前6位的地域聚合**：

如交易明细中有"对手证件号"列，通过 `excelcli query --sql` 执行以下查询。身份证前6位代表户籍所在地区划代码，相同前6位意味着来自同一地区，在反洗钱场景中常与老乡团伙、地域性犯罪组织相关。

```sql
SELECT SUBSTR(counterparty_id_no, 1, 6) as region_code,
       COUNT(DISTINCT counterparty_account) as account_count,
       GROUP_CONCAT(DISTINCT counterparty_account) as accounts,
       COUNT(*) as txn_count, ROUND(SUM(amount),2) as total_amount
FROM bank_transactions
WHERE counterparty_id_no IS NOT NULL AND counterparty_id_no != ''
  AND counterparty_account != '' AND counterparty_account != account_no
GROUP BY region_code HAVING account_count >= 2
ORDER BY account_count DESC
```

也可结合人员信息表中的"工作单位""单位地址"进行交叉验证，在报告中归纳地域团伙结论。

**子任务C3 — 基于对手开户银行的聚类分析**：

如交易明细中有"对手开户银行"列，通过 `excelcli query --sql` 执行以下查询。集中在同一银行/网点开立的多个对手账户可能提示有组织的开户行为。

```sql
SELECT counterparty_bank,
       COUNT(DISTINCT counterparty_account) as account_count,
       GROUP_CONCAT(DISTINCT counterparty_account) as accounts,
       COUNT(*) as txn_count, ROUND(SUM(amount),2) as total_amount
FROM bank_transactions
WHERE counterparty_bank IS NOT NULL AND counterparty_bank != ''
  AND counterparty_account != '' AND counterparty_account != account_no
GROUP BY counterparty_bank HAVING account_count >= 3
ORDER BY account_count DESC
```

**子任务C4 — 基于交易关系和身份特征的团伙拓展**：

通过 `excelcli query --sql` 完成以下分析，无需编写代码：

1. **同名账户关联**：不同账户号但户名相同，视为同一人控制（使用前文 SQL 查询 #13 "同名多户控制人"）
2. **稳定资金关系**：高频次、大金额的固定交易对手，通过以下 SQL 查询：
```sql
SELECT counterparty_account, counterparty_name, direction,
       COUNT(*) as txn_count, ROUND(SUM(amount),2) as total,
       ROUND(AVG(amount),2) as avg_amount
FROM bank_transactions
WHERE counterparty_account != '' AND counterparty_account != account_no
GROUP BY counterparty_account, counterparty_name, direction
HAVING txn_count >= 10 AND total >= 100000
ORDER BY txn_count DESC
```
3. **交易关键词关联**：摘要中含"换汇""换钱"等关键词的对手账户（使用前文 SQL 查询 #11 "可疑关键词交易"）
4. **关联人员信息**：通过证件号匹配人员信息表，分析对手的工作单位、单位地址是否集中

**数据缺失处理**：
- 如果交易明细中没有IP/MAC列：跳过C1，仅做C2-C4
- 如果没有对手证件号列：跳过C2，在报告中注明"因数据中缺少对手证件号信息，未进行地域关联分析"
- 不要因为部分数据缺失而放弃整个团伙分析章节

### 阶段D：可疑特征量化与研判

**目标**：对前三阶段的发现进行量化评估，形成结论。

计算以下可疑指标（详见下方"可疑指标阈值"），逐项给出数值和判定结论，作为报告"可疑点分析"章节的依据。完成后根据指标组合匹配"经营模式识别规则"，推断可能的犯罪类型。

---

## 可疑指标与阈值

以下是核心可疑指标的计算方法和判定阈值。分析时逐项计算，每项给出具体数值和判定结论。

| 序号 | 指标名称 | 计算方法 | 正常范围 | 异常阈值 | 高度异常 |
| --- | --- | --- | --- | --- | --- |
| 1 | 收支平衡度 | abs(收入-支出) / (收入+支出) | > 20% | < 5% | < 1% |
| 2 | 现金交易占比 | 现金交易笔数 / 总交易笔数 | < 20% | > 50% | > 70% |
| 3 | 对手集中度(来源) | TOP3收入金额 / 总收入 | < 30% | > 40% | > 60% |
| 4 | 对手集中度(去向) | TOP3支出金额 / 总支出 | < 30% | > 40% | > 60% |
| 5 | 同名账户数 | 对手户名=主体户名的不同账户数 | 0 | >= 2 | >= 5 |
| 6 | 整额交易占比 | 整万金额交易笔数 / 总笔数 | < 15% | > 30% | > 45% |
| 7 | 快进快出天数占比 | 同日进出均>5万的天数 / 总交易天数 | < 5% | > 10% | > 20% |
| 8 | 资金周转率 | 交易总额 / 平均余额 | < 20 | > 50 | > 100 |
| 9 | 可疑关键词交易 | 摘要含"换汇/换钱/换币/兑换/换美金"的笔数 | 0 | >= 1 | >= 10 |

补充指标（如数据支持，建议也计算）：
- 自身转账笔数和金额占比（存在且金额占比 > 5% 视为异常）
- 30分钟内进出配对数
- 非工作时间（22:00-08:00）交易占比
- **地域集中度**：出账对手中，证件号前6位相同的人数占总对手数的比例（> 30% 视为异常）
- **对手开户银行集中度**：TOP1开户银行的对手账户数 / 总对手账户数（> 40% 视为异常）

---

## 经营模式识别规则

根据可疑指标的组合，匹配以下犯罪模式。可能同时符合多种模式，逐一判断。

**地下钱庄 / 非法换汇**：
- 收支高度平衡（平衡度 < 5%）
- 现金交易占比高（> 50%）
- 对手集中度高
- 快进快出明显
- 存在"换汇/换钱"关键词
- 整额交易占比高

**非法资金归集转移**：
- 收支高度平衡
- 收入端小额多笔、支出端大额少笔（"小进大出"）
- 整额交易占比高
- 同名多账户控制

**电信诈骗资金分发**：
- 单向大额进入 + 快速分散小额出
- 账户活跃期短（通常数周到数月）
- 余额快速清零

**有组织团伙犯罪**：
- 多账户共享IP/MAC设备
- 同名多户控制
- 多层资金中转
- 对手证件号前6位地域高度集中
- 对手开户银行集中度高

匹配到模式后，在报告中基于阶段 A-D 的已有分析结果（无需额外编程或计算），按"四层模型"归纳描述经营模式：
1. **资金归集层**（收款阶段）：根据阶段 B 的 TOP10 来源表，描述上游资金如何归集
2. **资金中转层**（结算阶段）：根据阶段 A 的收支平衡度和快进快出指标，描述核心账户的整合与分配行为
3. **资金分配层**（分配阶段）：根据阶段 B 的 TOP10 去向表，描述向下游分发的路径
4. **变现层**（付款阶段）：根据阶段 A 的现金交易统计，描述现金取款或购汇行为

---

## 常见问题处理

分析过程中遇到问题时参考以下指引，不要卡住。

### 数据列名不匹配

不同系统导出的交易数据列名差异很大。先打印列名再适配：

使用 `excelcli inspect <DB> --json --sample 5` 查看真实字段和样例数据；使用 `excelcli transactions`、`excelcli tax invoices` 或 `excelcli query --sql` 完成筛选和统计。

常见列名对照（与本项目标准字段的映射）：

| 标准字段 | 可能的其他列名 |
| --- | --- |
| 交易卡号 | 卡号、账号、账户号码、主账号 |
| 交易账号 | 账号、内部账号、账户号 |
| 交易户名 | 户名、账户名称、客户姓名 |
| 交易证件号 | 证件号码、身份证号、身份证号码、证件号 |
| 交易时间 | 交易日期、记账时间、发生日期 |
| 交易金额 | 金额、发生额、交易发生金额 |
| 收付标志 | 借贷标志、交易方向、借贷方向 |
| 交易余额 | 余额、账户余额、当前余额 |
| 交易对手账卡号 | 对手账号、对方账户、对手卡号 |
| 对手户名 | 对方户名、对方姓名、对手名称 |
| 对手证件号 | 对手身份证号、对方证件号码 |
| 对手开户银行 | 对手银行、对方开户行、对手开户网点 |
| 交易发生地 | 交易地点、发生地、地区 |
| 交易网点名称 | 交易网点、网点名称、机构名称 |
| 交易是否成功 | 成功标志、交易状态、处理状态 |
| 现金标志 | 交易渠道、转账方式 |
| 摘要说明 | 摘要、交易摘要、备注、用途 |
| IP地址 | IP、登录IP、操作IP |
| MAC地址 | MAC、设备MAC、物理地址 |

如果找不到对应列，跳过依赖该列的分析项，在报告中注明"因数据中缺少XX字段，该项未分析"。

### Excel文件过大或读取失败

使用 `excelcli import` 时添加 `--verify` 参数校验数据完整性；导入后用 `excelcli inspect <DB>` 查看表结构和数据量。

### 分析结果异常

排查步骤：
1. 先用 `excelcli inspect <DB> --json --sample 5` 核对列名和样例数据
2. 检查映射 JSON 文件（如 mapping.json）中各字段映射是否正确
3. 必要时修改映射 JSON 后重新执行 `excelcli normalize <DB> --mapping mapping.json`

常见问题：
- 金额、时间、空值等解析问题：检查映射 JSON 中对应字段的映射关系，必要时修改映射后重新标准化
- 标准化后字段缺失：说明映射 JSON 中未映射该字段，补充映射后重新 normalize

### 分析结果不符合预期

- **收支严重不平衡**（如只有进没有出）：检查收付标志列的取值是否正确识别，可能是"借/贷"而非"进/出"
- **对手全部为NaN**：可能现金交易没有对手信息，属正常现象，在报告中说明
- **团伙划分结果只有1个大团伙**：检查是否有某个IP/MAC被大量账户共享（如公共WiFi），考虑过滤掉关联账户数超过50的IP
- **年度统计金额与总额对不上**：检查是否有交易金额为0的记录或异常值
- **交易笔数与预期不符**：检查是否已过滤"交易是否成功" != 1 的失败交易

### 数据量过少（<100笔）

- 年度趋势改为按季度或月度统计
- 金额分布合并为3个区间（小额/中额/大额）
- 团伙分析简化为仅做同名账户和交易关系分析
- 快进快出改为逐笔检查而非统计天数

---

## 报告结构

撰写正式报告时，优先读取并遵循同目录的 [report-template.md](report-template.md)。


| 章节 | 核心内容 | 数据来源 |
| --- | --- | --- |
| 一、基本情况 | 账户信息表(含子账户) + 交易概况表 + 交易结构特点 | 阶段A + 辅助表 |
| 二、资金交易情况 | 总体概况表 + 年度趋势表 + 金额分布表 + 地理分布表 | 阶段A |
| 三、主要资金来源分析 | 集中度 + TOP10来源表 + 逐户分析 | 阶段B |
| 四、主要资金去向分析 | 集中度 + TOP10去向表 + 流向模式 + 开户银行分布 | 阶段B |
| 五、团伙划分分析 | IP/MAC团伙(出方向) + 证件号地域聚合 + 开户银行聚类 + 交易关系拓展 + 组织架构 | 阶段C |
| 六、可疑点分析 | 逐项列出每个可疑特征及数据支撑 | 阶段D |
| 七、经营模式与资金穿透 | 四层模型 + 资金流入/流出路径 + 涉案规模 | 全部阶段 |
| 八、下一步工作建议 | 立即措施 + 深度调查 + 扩线调查 | 全部阶段 |
| 九、结论 | 证据汇总 + 涉案规模 + 处置建议 | 全部阶段 |
