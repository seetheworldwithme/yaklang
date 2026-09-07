# 参考代码片段

按分析维度组织的 Python 代码片段。**不是可以直接运行的完整脚本**，而是供模型根据实际数据格式组合和调整的参考。

使用前请先确认：
1. 实际的列名（用 `df.columns.tolist()` 检查）
2. 收付标志的具体取值（用 `df['收付标志'].unique()` 检查）
3. 数据类型（用 `df.dtypes` 检查，金额是否为数值型、时间是否需要转换）

---

## 1. 数据读取与探查

```python
import pandas as pd
import numpy as np
from collections import defaultdict

# --- 识别并读取数据文件 ---
# 通过列名自动识别文件类型，也可手动指定文件路径
import glob

files = glob.glob('*.xlsx') + glob.glob('*.xls')
file_map = {}  # type -> filepath

for f in files:
    df_tmp = pd.read_excel(f, nrows=1)
    cols = set(df_tmp.columns.tolist())

    if '交易时间' in cols and '收付标志' in cols:
        file_map['交易明细'] = f
    elif '客户名称' in cols and '证照号码' in cols:
        file_map['人员信息'] = f
    elif '账户开户名称' in cols and '开户网点' in cols:
        file_map['账户信息'] = f
    elif '子账户账号' in cols and '子账户类别' in cols:
        file_map['子账户信息'] = f

print("识别到的数据文件:", file_map)

# --- 读取交易明细（主表） ---
df = pd.read_excel(file_map.get('交易明细', '交易明细.xlsx'))
print("=== 交易明细 ===")
print(f"形状: {df.shape}")
print(f"列名: {df.columns.tolist()}")
print(df.dtypes)
print(df.head(3))

# --- 读取辅助表 ---
df_person = None
df_account = None
df_subaccount = None

if '人员信息' in file_map:
    df_person = pd.read_excel(file_map['人员信息'])
    print(f"\n=== 人员信息 ===")
    print(f"形状: {df_person.shape}")
    print(f"列名: {df_person.columns.tolist()}")

if '账户信息' in file_map:
    df_account = pd.read_excel(file_map['账户信息'])
    print(f"\n=== 账户信息 ===")
    print(f"形状: {df_account.shape}")
    print(f"列名: {df_account.columns.tolist()}")

if '子账户信息' in file_map:
    df_subaccount = pd.read_excel(file_map['子账户信息'])
    print(f"\n=== 子账户信息 ===")
    print(f"形状: {df_subaccount.shape}")
    print(f"列名: {df_subaccount.columns.tolist()}")
```

### 检查主体账户数量

```python
# 确认数据中涉及几个主体账户
if COL['self_account'] in df.columns:
    accounts = df[COL['self_account']].unique()
    print(f"数据中涉及 {len(accounts)} 个主体账户: {accounts.tolist()}")
    if len(accounts) == 1:
        print("→ 单账户分析模式")
    else:
        print("→ 多账户分析模式，需在报告中列出所有涉案账户")
```

### 列名适配模板

```python
# 根据实际列名修改以下映射（以下为典型字段，按实际数据调整）
COL = {
    'time': '交易时间',           # 交易发生时间
    'amount': '交易金额',         # 交易金额（正数）
    'direction': '收付标志',      # 收入/支出方向（"进"/"出"）
    'balance': '交易余额',        # 交易后余额
    'opp_account': '交易对手账卡号',  # 对手账户号
    'opp_name': '对手户名',       # 对手户名
    'opp_id': '对手证件号',       # 对手证件号/身份证号
    'opp_bank': '对手开户银行',   # 对手开户银行
    'opp_balance': '对手交易余额', # 对手交易后余额（可选）
    'cash_flag': '现金标志',      # 现金/转账标志
    'memo': '摘要说明',           # 交易摘要/备注
    'self_account': '交易卡号',   # 本方交易卡号
    'self_name': '交易户名',      # 本方户名
    'self_id': '交易证件号',      # 本方证件号
    'ip': 'IP地址',              # IP地址
    'mac': 'MAC地址',            # MAC地址
    'location': '交易发生地',     # 交易发生地点
    'branch': '交易网点名称',     # 交易网点
    'success': '交易是否成功',    # 交易是否成功（1=成功）
    'currency': '交易币种',       # 币种
    'txn_serial': '交易流水号',   # 交易流水号
    'voucher': '凭证号',          # 凭证号
    'teller': '交易柜员号',       # 柜员号
}

# 收付方向的取值映射（根据实际数据调整）
DIR_IN = '进'    # 或 '收', '贷', 'C'
DIR_OUT = '出'   # 或 '付', '借', 'D'

# --- 数据预处理 ---
df = df.copy()
df[COL['time']] = pd.to_datetime(df[COL['time']])
df[COL['amount']] = pd.to_numeric(df[COL['amount']], errors='coerce').abs()

# 过滤失败交易（兼容不同银行的取值格式）
if COL['success'] in df.columns:
    # 先查看实际取值
    success_values = df[COL['success']].unique()
    print(f"交易是否成功列取值: {success_values}")

    # 定义失败标识（各种银行的失败取值）
    fail_values = {0, '0', '失败', '交易失败', 'FAIL', 'F', 'N', '否', False}
    success_values_set = set(str(v).strip().upper() for v in success_values)

    # 判断该列是否用数值型表示（如0/1）
    is_numeric = df[COL['success']].dtype in ['int64', 'float64', 'int32']

    if is_numeric:
        # 数值型：0=失败，其余保留
        total_before = len(df)
        df = df[df[COL['success']] != 0]
        filtered = total_before - len(df)
    else:
        # 字符串/混合型：仅过滤明确为失败的值
        total_before = len(df)
        fail_mask = df[COL['success']].astype(str).str.strip().str.upper().isin(
            [str(v) for v in fail_values]
        )
        df = df[~fail_mask]
        filtered = total_before - len(df)

    # 安全检查：如果过滤掉了超过50%的数据，说明取值识别可能有误，保留全部
    if filtered > total_before * 0.5:
        print(f"警告: 过滤掉了{filtered}/{total_before}笔(>{filtered/total_before*100:.0f}%)，"
              f"过滤条件可能有误，恢复全部数据")
        df = pd.read_excel(file_map.get('交易明细', '交易明细.xlsx'))
        df[COL['time']] = pd.to_datetime(df[COL['time']])
        df[COL['amount']] = pd.to_numeric(df[COL['amount']], errors='coerce').abs()
        filtered = 0

    print(f"过滤失败交易: {filtered}笔, 剩余{len(df)}笔")

df_in = df[df[COL['direction']] == DIR_IN]
df_out = df[df[COL['direction']] == DIR_OUT]

print(f"收入: {len(df_in)}笔, 支出: {len(df_out)}笔")
```

### 辅助表关联

```python
# 构建对手证件号 → 人员信息的映射（用于报告中补充对手背景）
person_lookup = {}
if df_person is not None:
    person_col_id = '证照号码' if '证照号码' in df_person.columns else None
    if person_col_id:
        for _, row in df_person.iterrows():
            person_lookup[row[person_col_id]] = row.to_dict()
        print(f"人员信息已加载: {len(person_lookup)}条")

# 构建涉案账户信息汇总（用于报告"账户基本信息"章节）
# 当前余额从交易明细最后一笔的交易余额取值，不从账户信息表取
if df_account is not None:
    print("\n=== 涉案账户信息 ===")
    for _, row in df_account.iterrows():
        name = row.get('账户开户名称', '未知')
        card = row.get('交易卡号', '未知')
        open_date = row.get('账号开户时间', '未知')
        branch = row.get('开户网点', '未知')
        acct_type = row.get('账户类型', '未知')
        status = row.get('账户状态', '未知')
        bank = row.get('账号开户银行', '未知')

        last_balance = '未知'
        if COL['balance'] in df.columns:
            acct_txns = df[df[COL['self_account']] == card].sort_values(COL['time'])
            if len(acct_txns) > 0:
                last_balance = pd.to_numeric(acct_txns[COL['balance']].iloc[-1], errors='coerce')
                last_balance = f"{last_balance:,.2f}" if pd.notna(last_balance) else '未知'

        print(f"  {name}({card}): 开户时间={open_date}, 开户行={bank}, 网点={branch}, "
              f"类型={acct_type}, 状态={status}, 当前余额={last_balance}")

# 子账户信息汇总
if df_subaccount is not None:
    print(f"\n=== 子账户信息 ===")
    sub_summary = df_subaccount.groupby('子账户类别').agg(
        count=('子账户账号', 'count'),
        total_balance=('余额', 'sum')
    ).sort_values('total_balance', ascending=False)
    for cat, row in sub_summary.iterrows():
        print(f"  {cat}: {row['count']}个, 合计余额{row['total_balance']:,.2f}")
```

---

## 2. 基础统计（阶段A）

### 交易概况

```python
total_count = len(df)
total_amount = df[COL['amount']].sum()

in_count = len(df_in)
in_amount = df_in[COL['amount']].sum()
out_count = len(df_out)
out_amount = df_out[COL['amount']].sum()

date_min = df[COL['time']].min()
date_max = df[COL['time']].max()
date_span = (date_max - date_min).days

balance_ratio = abs(in_amount - out_amount) / (in_amount + out_amount) if (in_amount + out_amount) > 0 else 0

# 对手户名去重个数（反映实际交易涉及的自然人/法人数量）
in_opp_names = df_in[COL['opp_name']].dropna().nunique()
out_opp_names = df_out[COL['opp_name']].dropna().nunique()

print(f"交易笔数: {total_count}, 总金额: {total_amount:,.2f}")
print(f"收入: {in_count}笔 / {in_amount:,.2f}元, 平均单笔: {in_amount/in_count:,.2f}元" if in_count > 0 else "收入: 0笔")
print(f"支出: {out_count}笔 / {out_amount:,.2f}元, 平均单笔: {out_amount/out_count:,.2f}元" if out_count > 0 else "支出: 0笔")
print(f"收入端涉及对手户名: {in_opp_names}个")
print(f"支出端涉及对手户名: {out_opp_names}个")
print(f"时间跨度: {date_min.date()} ~ {date_max.date()}, 共{date_span}天")
print(f"收支平衡度: {balance_ratio:.4f} ({balance_ratio*100:.2f}%)")
print(f"日均交易: {total_count / max(date_span, 1):.1f}笔")
```

### 年度趋势（含对手主体数）

```python
df['year'] = df[COL['time']].dt.year

# 基础年度统计
yearly = df.groupby(['year', COL['direction']]).agg(
    count=(COL['amount'], 'count'),
    total=(COL['amount'], 'sum')
).unstack(fill_value=0)

# 每年进出分别涉及多少个对手主体
yearly_opp = df.dropna(subset=[COL['opp_account']]).groupby(
    ['year', COL['direction']]
)[COL['opp_account']].nunique().unstack(fill_value=0)
yearly_opp.columns = [f'对手主体数_{c}' for c in yearly_opp.columns]

print("\n=== 年度交易趋势 ===")
print(yearly)
print("\n=== 年度对手主体数 ===")
print(yearly_opp)
```

### 金额分布

```python
bins = [0, 1000, 5000, 10000, 50000, 100000, 500000, float('inf')]
labels = ['0-1千', '1千-5千', '5千-1万', '1万-5万', '5万-10万', '10万-50万', '50万以上']

df['amount_bin'] = pd.cut(df[COL['amount']], bins=bins, labels=labels, right=True)

amount_dist = df.groupby([COL['direction'], 'amount_bin'], observed=True).size().unstack(fill_value=0)
print("\n=== 金额分布 ===")
print(amount_dist)
```

### 交易地理分布

```python
if COL['location'] in df.columns:
    location_stats = df.groupby(COL['location']).agg(
        count=(COL['amount'], 'count'),
        total=(COL['amount'], 'sum')
    ).sort_values('total', ascending=False)

    print("\n=== 交易地理分布 ===")
    for loc, row in location_stats.head(15).iterrows():
        print(f"  {loc}: {row['count']}笔, {row['total']:,.2f}元 ({row['total']/total_amount*100:.1f}%)")

    # 地理集中度：TOP5地区占总交易金额的比例
    top5_geo_ratio = location_stats['total'].head(5).sum() / total_amount * 100
    print(f"  TOP5地区集中度: {top5_geo_ratio:.1f}%")
```

### 对手开户银行分布

```python
if COL['opp_bank'] in df.columns:
    bank_dist = df.dropna(subset=[COL['opp_bank']]).groupby(COL['opp_bank']).agg(
        account_count=(COL['opp_account'], 'nunique'),
        txn_count=(COL['amount'], 'count'),
        total=(COL['amount'], 'sum')
    ).sort_values('account_count', ascending=False)

    print("\n=== 对手开户银行分布 ===")
    for bank, row in bank_dist.head(15).iterrows():
        print(f"  {bank}: {row['account_count']}个对手账户, {row['txn_count']}笔, {row['total']:,.2f}元")

    # 开户银行集中度
    total_opp_accounts = df[COL['opp_account']].dropna().nunique()
    top1_bank_accounts = bank_dist['account_count'].iloc[0] if len(bank_dist) > 0 else 0
    bank_conc = top1_bank_accounts / total_opp_accounts * 100 if total_opp_accounts > 0 else 0
    print(f"  TOP1开户银行集中度: {bank_conc:.1f}%")
```

### 现金交易统计

```python
if COL['cash_flag'] in df.columns:
    print(f"现金标志取值: {df[COL['cash_flag']].unique()}")

    cash_mask = df[COL['cash_flag']].astype(str).str.contains('现')
    cash_count = cash_mask.sum()
    cash_amount = df.loc[cash_mask, COL['amount']].sum()

    print(f"现金交易: {cash_count}笔 ({cash_count/total_count*100:.1f}%), "
          f"金额: {cash_amount:,.2f} ({cash_amount/total_amount*100:.1f}%)")
```

### 余额特征

```python
if COL['balance'] in df.columns:
    balances = pd.to_numeric(df[COL['balance']], errors='coerce').dropna()
    if len(balances) > 0:
        print(f"\n=== 余额特征 ===")
        print(f"最高余额: {balances.max():,.2f}")
        print(f"最低余额: {balances.min():,.2f}")
        print(f"平均余额: {balances.mean():,.2f}")
        sorted_df = df.sort_values(COL['time'])
        final_balance = pd.to_numeric(sorted_df[COL['balance']].iloc[-1], errors='coerce')
        print(f"最终余额: {final_balance:,.2f}")
```

---

## 3. 对手分析（阶段B）

### 收入/支出涉及账户数

```python
in_accounts = df_in[COL['opp_account']].dropna().nunique()
out_accounts = df_out[COL['opp_account']].dropna().nunique()
print(f"收入端涉及 {in_accounts} 个对手账户")
print(f"支出端涉及 {out_accounts} 个对手账户")
```

### TOP10 资金来源/去向

```python
def top_counterparties(df_subset, direction_label, n=10):
    """统计TOP-N对手账户，包含证件号用于关联人员信息"""
    grouped = df_subset.groupby([COL['opp_account'], COL['opp_name']]).agg(
        count=(COL['amount'], 'count'),
        total=(COL['amount'], 'sum'),
        avg=(COL['amount'], 'mean'),
        first_time=(COL['time'], 'min'),
        last_time=(COL['time'], 'max')
    ).sort_values('total', ascending=False)

    # 获取对手证件号
    opp_ids = df_subset.dropna(subset=[COL['opp_account'], COL.get('opp_id', '')]).drop_duplicates(
        subset=[COL['opp_account']]
    ).set_index(COL['opp_account'])[COL['opp_id']].to_dict() if COL.get('opp_id') and COL['opp_id'] in df_subset.columns else {}

    print(f"\n=== TOP{n} {direction_label} ===")
    for i, ((acct, name), row) in enumerate(grouped.head(n).iterrows(), 1):
        opp_id_info = opp_ids.get(acct, '未知')
        print(f"{i}. {name}({acct}): {row['count']}笔, "
              f"合计{row['total']:,.2f}, 均笔{row['avg']:,.2f}, 证件: {opp_id_info}")

    return grouped

in_top = top_counterparties(df_in, '资金来源')
out_top = top_counterparties(df_out, '资金去向')
```

### 对手背景信息补充

```python
# 对TOP对手通过证件号关联人员信息表
def get_person_info(opp_id, lookup):
    """通过证件号查询人员背景信息"""
    if not opp_id or opp_id == '未知' or not lookup:
        return None
    # 尝试精确匹配或前缀匹配（脱敏数据可能截断了部分号码）
    for key, info in lookup.items():
        if opp_id.startswith(key[:6]) or key.startswith(opp_id[:6]):
            return info
    return None

# 查看TOP对手的工作单位信息
if person_lookup:
    print("\n=== TOP对手人员背景 ===")
    for direction, df_top, top_data in [('来源', df_in, in_top), ('去向', df_out, out_top)]:
        print(f"\n--- {direction}端TOP10对手 ---")
        for i, ((acct, name), row) in enumerate(top_data.head(10).iterrows(), 1):
            opp_id = df_top[df_top[COL['opp_account']] == acct][COL.get('opp_id', '')].dropna().unique()
            opp_id = opp_id[0] if len(opp_id) > 0 else None
            info = get_person_info(opp_id, person_lookup)
            if info:
                work = info.get('工作单位', '未知')
                unit_addr = info.get('单位地址', '未知')
                legal = info.get('法人代表', '未知')
                print(f"  {i}. {name}: 工作单位={work}, 地址={unit_addr}, 法人={legal}")
            else:
                print(f"  {i}. {name}: 人员信息未匹配")
```

### 集中度计算

```python
def concentration(grouped_total, overall_total):
    """计算TOP1/3/5集中度"""
    cumsum = grouped_total.cumsum()
    result = {}
    for n in [1, 3, 5]:
        if len(cumsum) >= n:
            result[f'TOP{n}'] = cumsum.iloc[n-1] / overall_total * 100
        else:
            result[f'TOP{n}'] = cumsum.iloc[-1] / overall_total * 100
    return result

in_conc = concentration(in_top['total'], in_amount)
out_conc = concentration(out_top['total'], out_amount)

print(f"\n来源集中度: TOP1={in_conc['TOP1']:.1f}%, TOP3={in_conc['TOP3']:.1f}%, TOP5={in_conc['TOP5']:.1f}%")
print(f"去向集中度: TOP1={out_conc['TOP1']:.1f}%, TOP3={out_conc['TOP3']:.1f}%, TOP5={out_conc['TOP5']:.1f}%")
```

### 同名账户与双向交易

```python
# 主体户名（从数据中获取）
SELF_NAME = df[COL['self_name']].mode()[0] if COL['self_name'] in df.columns else '主体户名'

# 同名账户
same_name = df[df[COL['opp_name']] == SELF_NAME]
if len(same_name) > 0:
    same_name_accounts = same_name[COL['opp_account']].unique()
    print(f"\n同名账户: {len(same_name_accounts)}个, 交易{len(same_name)}笔, "
          f"金额{same_name[COL['amount']].sum():,.2f}")
    for acct in same_name_accounts:
        sub = same_name[same_name[COL['opp_account']] == acct]
        print(f"  - {acct}: {len(sub)}笔, {sub[COL['amount']].sum():,.2f}")

# 双向交易对手
in_set = set(df_in[COL['opp_account']].dropna().unique())
out_set = set(df_out[COL['opp_account']].dropna().unique())
bidirectional = in_set & out_set
print(f"\n双向交易对手: {len(bidirectional)}个")
for acct in sorted(bidirectional):
    name = df[df[COL['opp_account']] == acct][COL['opp_name']].iloc[0]
    in_amt = df_in[df_in[COL['opp_account']] == acct][COL['amount']].sum()
    out_amt = df_out[df_out[COL['opp_account']] == acct][COL['amount']].sum()
    print(f"  {name}({acct}): 进{in_amt:,.2f}, 出{out_amt:,.2f}")

# 自身转账（对手账户号包含主体账户号的记录）
SELF_ACCOUNT = df[COL['self_account']].mode()[0] if COL['self_account'] in df.columns else None
if SELF_ACCOUNT:
    self_transfer = df[df[COL['opp_account']] == SELF_ACCOUNT]
    if len(self_transfer) > 0:
        print(f"\n自身转账: {len(self_transfer)}笔, {self_transfer[COL['amount']].sum():,.2f}")
```

---

## 3.5 对手资金画像分析

```python
def analyze_counterparty_profile(df, opp_account, col_self_account, col_opp_account,
                                   col_opp_name, col_amount, col_direction, col_time,
                                   col_balance, dir_in, dir_out):
    """
    对指定对手账户生成资金画像：交易时间/金额/频率/方向等多维分析。
    如该对手在数据中也作为交易卡号（主体），则做完整的双向分析。
    """
    # 该对手作为"对手"出现的所有交易（与主体的交互）
    as_opp = df[df[col_opp_account] == opp_account]
    opp_name = as_opp[col_opp_name].iloc[0] if len(as_opp) > 0 else '未知'

    # 该对手作为"交易卡号"出现的所有交易（对手自身的交易记录）
    as_self = df[df[col_self_account] == opp_account]
    has_own_records = len(as_self) > 0

    print(f"\n=== 对手资金画像: {opp_name} ({opp_account}) ===")
    print(f"与主体交易记录: {len(as_opp)}笔")
    print(f"对手自身交易记录: {len(as_self)}笔 ({'有' if has_own_records else '无'})")

    # --- 与主体的交易分析 ---
    in_txns = as_opp[as_opp[col_direction] == dir_in]
    out_txns = as_opp[as_opp[col_direction] == dir_out]

    in_count, in_amount = len(in_txns), in_txns[col_amount].sum()
    out_count, out_amount = len(out_txns), out_txns[col_amount].sum()

    print(f"\n与主体交易概况:")
    print(f"  收入(主体←对手): {in_count}笔, {in_amount:,.2f}元")
    print(f"  支出(主体→对手): {out_count}笔, {out_amount:,.2f}元")

    direction = '双向' if (in_count > 0 and out_count > 0) else ('进' if in_count > 0 else '出')
    print(f"  交易方向: {direction}")

    if len(as_opp) > 0:
        print(f"  首次交易: {as_opp[col_time].min()}")
        print(f"  末次交易: {as_opp[col_time].max()}")
        print(f"  平均单笔: {as_opp[col_amount].mean():,.2f}元")
        print(f"  最大单笔: {as_opp[col_amount].max():,.2f}元")
        print(f"  最小单笔: {as_opp[col_amount].min():,.2f}元")

        # 时段分布
        hours = as_opp[col_time].dt.hour
        night_ratio = ((hours >= 22) | (hours < 8)).sum() / len(as_opp)
        peak_hour = hours.mode()[0] if len(hours) > 0 else '未知'
        print(f"  夜间交易占比: {night_ratio*100:.1f}%, 交易高峰时段: {peak_hour}时")

        # 涉及的主体账户
        involved_accounts = as_opp[col_self_account].unique()
        print(f"  涉及主体账户: {len(involved_accounts)}个 → {involved_accounts.tolist()}")

    # --- 对手自身交易分析（如有数据）---
    if has_own_records:
        own_in = as_self[as_self[col_direction] == dir_in]
        own_out = as_self[as_self[col_direction] == dir_out]
        own_in_amount = own_in[col_amount].sum()
        own_out_amount = own_out[col_amount].sum()
        own_total = own_in_amount + own_out_amount

        print(f"\n对手自身交易概况:")
        print(f"  总笔数: {len(as_self)}, 总金额: {own_total:,.2f}元")
        print(f"  收入: {len(own_in)}笔/{own_in_amount:,.2f}元, 支出: {len(own_out)}笔/{own_out_amount:,.2f}元")

        if own_total > 0:
            own_ratio = own_in_amount / own_total
            print(f"  交易比值: {own_ratio:.4f} → ", end='')
            if own_ratio >= 0.8:
                print("资金来源账户")
            elif own_ratio >= 0.6:
                print("偏来源型账户")
            elif own_ratio >= 0.4:
                print("过渡/中转账户")
            elif own_ratio >= 0.2:
                print("偏去向型账户")
            else:
                print("资金去向账户")

            own_balance_ratio = abs(own_in_amount - own_out_amount) / own_total
            print(f"  收支平衡度: {own_balance_ratio*100:.2f}%")

        # 对手的TOP对手
        own_opp_top = as_self.groupby(col_opp_account)[col_amount].agg(['sum', 'count']).sort_values('sum', ascending=False)
        print(f"\n对手的TOP5交易对手:")
        for acct, row in own_opp_top.head(5).iterrows():
            name = df[df[col_opp_account] == acct][col_opp_name].iloc[0] if len(df[df[col_opp_account] == acct]) > 0 else '未知'
            print(f"    {name}({acct}): {row['count']}笔, {row['sum']:,.2f}元")

    return {
        'opp_account': opp_account,
        'opp_name': opp_name,
        'with_subject': {'in_count': in_count, 'in_amount': in_amount,
                         'out_count': out_count, 'out_amount': out_amount, 'direction': direction},
        'has_own_records': has_own_records,
    }

# 对TOP5来源和去向的对手分别生成资金画像
# for acct in top_sources[:5] + top_targets[:5]:
#     analyze_counterparty_profile(
#         df, acct, COL['self_account'], COL['opp_account'],
#         COL['opp_name'], COL['amount'], COL['direction'],
#         COL['time'], COL['balance'], DIR_IN, DIR_OUT
#     )
```

---

## 4. 团伙划分（阶段C）

### Union-Find 算法

```python
class UnionFind:
    """并查集，用于将共享IP/MAC的账户聚类为团伙"""

    def __init__(self):
        self.parent = {}
        self.rank = {}

    def find(self, x):
        if x not in self.parent:
            self.parent[x] = x
            self.rank[x] = 0
        if self.parent[x] != x:
            self.parent[x] = self.find(self.parent[x])
        return self.parent[x]

    def union(self, x, y):
        rx, ry = self.find(x), self.find(y)
        if rx == ry:
            return
        if self.rank[rx] < self.rank[ry]:
            rx, ry = ry, rx
        self.parent[ry] = rx
        if self.rank[rx] == self.rank[ry]:
            self.rank[rx] += 1

    def get_groups(self):
        groups = defaultdict(set)
        for x in self.parent:
            groups[self.find(x)].add(x)
        return dict(groups)
```

### 基于出账记录的 IP/MAC 聚类

```python
# 直接从交易明细中进行团伙分析（无需单独的对手数据文件）

# 1. 获取主体账户的出账对手列表
out_counterparties = set(df_out[COL['opp_account']].dropna().unique())
print(f"出账对手总数: {len(out_counterparties)}")

# 2. 从交易明细中筛选出账对手的"出"方向记录
#    注意：这里的"出"方向是指这些对手作为出账方（即交易卡号=对手账户）的记录
#    但如果数据中只有主体账户的交易明细，则我们用主体账户的出账记录来分析
#    关键：我们要找出哪些出账对手在"出"方向上使用了相同的IP/MAC

# 方法一：如果交易明细中包含对手账户的交易（多账户明细），
# 可以直接筛选交易卡号在出账对手列表中的记录
out_cp_records = df[df[COL['self_account']].isin(out_counterparties)]
# 进一步筛选出方向
out_cp_out = out_cp_records[out_cp_records[COL['direction']] == DIR_OUT].copy()
print(f"出账对手的出方向交易记录数: {len(out_cp_out)}")

# 方法二：如果交易明细只有主体账户的记录（无法看到对手的行为），
# 则用主体账户出账时的对手信息进行关联分析
# 此时IP/MAC可能代表的是交易发生的设备信息

# 以下以方法一为示例（多账户明细模式）：
has_ip = COL.get('ip') and COL['ip'] in out_cp_out.columns
has_mac = COL.get('mac') and COL['mac'] in out_cp_out.columns
print(f"IP列存在: {has_ip}, MAC列存在: {has_mac}")

# 3. 统计每个IP/MAC在出账方向上关联的交易卡号数量
if has_ip:
    ip_out = out_cp_out.dropna(subset=[COL['ip']])
    ip_account_count = ip_out.groupby(COL['ip'])[COL['self_account']].nunique()
    shared_ips = ip_account_count[ip_account_count >= 2]
    print(f"\n共享IP（出方向，关联>=2个卡号）: {len(shared_ips)}个")
    for ip, cnt in shared_ips.sort_values(ascending=False).head(20).items():
        accounts = ip_out[ip_out[COL['ip']] == ip][COL['self_account']].unique()
        print(f"  {ip}: 涉及{cnt}个主体卡号 → {accounts.tolist()[:10]}")

if has_mac:
    mac_out = out_cp_out.dropna(subset=[COL['mac']])
    mac_account_count = mac_out.groupby(COL['mac'])[COL['self_account']].nunique()
    shared_macs = mac_account_count[mac_account_count >= 2]
    print(f"\n共享MAC（出方向，关联>=2个卡号）: {len(shared_macs)}个")
    for mac, cnt in shared_macs.sort_values(ascending=False).head(20).items():
        accounts = mac_out[mac_out[COL['mac']] == mac][COL['self_account']].unique()
        print(f"  {mac}: 涉及{cnt}个主体卡号 → {accounts.tolist()[:10]}")

# 4. Union-Find 聚类
if has_ip or has_mac:
    ip_data = out_cp_out.dropna(subset=[COL['ip']]) if has_ip else pd.DataFrame()
    mac_data = out_cp_out.dropna(subset=[COL['mac']]) if has_mac else pd.DataFrame()

    accounts_with_device = set()
    if len(ip_data) > 0:
        accounts_with_device |= set(ip_data[COL['self_account']].unique())
    if len(mac_data) > 0:
        accounts_with_device |= set(mac_data[COL['self_account']].unique())

    print(f"\n有设备信息的出账对手: {len(accounts_with_device)}个")

    uf = UnionFind()

    if has_ip and len(ip_data) > 0:
        for ip, group in ip_data.groupby(COL['ip']):
            accounts = group[COL['self_account']].unique().tolist()
            for i in range(1, len(accounts)):
                uf.union(accounts[0], accounts[i])

    if has_mac and len(mac_data) > 0:
        for mac, group in mac_data.groupby(COL['mac']):
            accounts = group[COL['self_account']].unique().tolist()
            for i in range(1, len(accounts)):
                uf.union(accounts[0], accounts[i])

    for acct in accounts_with_device:
        uf.find(acct)

    gangs = uf.get_groups()
    gangs = {k: v for k, v in gangs.items() if len(v) >= 2}

    print(f"\n识别出 {len(gangs)} 个团伙（>=2人）:")
    for i, (root, members) in enumerate(sorted(gangs.items(), key=lambda x: -len(x[1])), 1):
        print(f"\n--- 团伙{i} ({len(members)}人) ---")
        for m in sorted(members):
            txn = df_out[df_out[COL['opp_account']] == m]
            amt = txn[COL['amount']].sum() if len(txn) > 0 else 0
            cnt = len(txn)
            print(f"  {m}: 与主体交易{cnt}笔, {amt:,.2f}元")
```

### 基于对手证件号前6位的地域聚合

```python
if COL.get('opp_id') and COL['opp_id'] in df.columns:
    df_id = df.dropna(subset=[COL['opp_account'], COL['opp_id']])[
        [COL['opp_account'], COL['opp_name'], COL['opp_id']]
    ].drop_duplicates(subset=[COL['opp_account']])

    # 提取前6位（户籍地区划代码）
    df_id['id_prefix6'] = df_id[COL['opp_id']].astype(str).str[:6]
    # 过滤无效值（非6位数字开头）
    df_id = df_id[df_id['id_prefix6'].str.match(r'^\d{6}', na=False)]

    # 按前6位分组，统计每组有多少个不同账户
    region_groups = df_id.groupby('id_prefix6').agg(
        accounts=(COL['opp_account'], lambda x: set(x)),
        count=(COL['opp_account'], 'count'),
        names=(COL['opp_name'], list)
    )
    region_gangs = region_groups[region_groups['accounts'].apply(len) >= 2]

    total_opp_with_id = df_id[COL['opp_account']].nunique()

    print(f"\n=== 对手证件号地域聚合 ===")
    print(f"有证件号的对手: {total_opp_with_id}个")
    print(f"同地域组（>=2人）: {len(region_gangs)}组")

    for prefix, row in region_gangs.iterrows():
        accounts = row['accounts']
        print(f"\n  地区代码 {prefix}: {len(accounts)}个对手账户")
        for acct in sorted(accounts):
            name_rows = df[df[COL['opp_account']] == acct]
            name = name_rows[COL['opp_name']].iloc[0] if len(name_rows) > 0 else '未知'
            txn = df_out[df_out[COL['opp_account']] == acct]
            print(f"    {name}({acct}): 与主体交易{len(txn)}笔, {txn[COL['amount']].sum():,.2f}元")

    # 地域集中度
    if len(region_gangs) > 0:
        max_region = region_gangs['accounts'].apply(len).max()
        region_conc = max_region / total_opp_with_id * 100
        print(f"\n最大地域组占比: {max_region}/{total_opp_with_id} = {region_conc:.1f}%")
```

### 基于对手开户银行的聚类

```python
if COL.get('opp_bank') and COL['opp_bank'] in df.columns:
    bank_accounts = df.dropna(subset=[COL['opp_account'], COL['opp_bank']]).drop_duplicates(
        subset=[COL['opp_account']]
    )

    bank_groups = bank_accounts.groupby(COL['opp_bank']).agg(
        account_count=(COL['opp_account'], 'count'),
        accounts=(COL['opp_account'], list)
    ).sort_values('account_count', ascending=False)

    # 找出开户在同一银行的对手（>=2个账户）
    bank_gangs = bank_groups[bank_groups['account_count'] >= 2]

    print(f"\n=== 对手开户银行聚类 ===")
    print(f"有开户银行信息的对手: {bank_accounts[COL['opp_account']].nunique()}个")
    print(f"同银行组（>=2人）: {len(bank_gangs)}组")

    for bank, row in bank_gangs.head(10).iterrows():
        accounts = row['accounts']
        total_amt = sum(
            df_out[df_out[COL['opp_account']] == acct][COL['amount']].sum()
            for acct in accounts
        )
        print(f"  {bank}: {row['account_count']}个对手账户, 与主体交易合计{total_amt:,.2f}元")
```

### 基于交易关系的团伙拓展

```python
# 同名账户拓展
opp_names = df[COL['opp_name']].dropna()
name_counts = opp_names.value_counts()
multi_account_names = name_counts[name_counts >= 2].index.tolist()

print("\n=== 同名多户 ===")
for name in multi_account_names:
    accounts = df[df[COL['opp_name']] == name][COL['opp_account']].unique()
    total = df[df[COL['opp_name']] == name][COL['amount']].sum()
    print(f"{name}: {len(accounts)}个账户, 合计{total:,.2f}元")

# 高频交易对手（潜在稳定关系）
freq_threshold = 10  # 交易次数阈值，可调整
opp_freq = df.groupby(COL['opp_account']).size()
high_freq = opp_freq[opp_freq >= freq_threshold]
if len(high_freq) > 0:
    print(f"\n=== 高频对手（>={freq_threshold}笔）===")
    for acct, cnt in high_freq.sort_values(ascending=False).items():
        name = df[df[COL['opp_account']] == acct][COL['opp_name']].iloc[0] if len(df[df[COL['opp_account']] == acct]) > 0 else '未知'
        print(f"  {name}({acct}): {cnt}笔")
```

---

### 对手与涉案主体的团伙关系分析

```python
def analyze_counterparty_gang_relations(df, col_self_account, col_opp_account, col_opp_name,
                                          col_amount, col_direction, col_time,
                                          col_ip, col_mac, col_opp_id,
                                          dir_in, dir_out, subject_accounts=None):
    """
    分析交易对手与涉案主体之间的团伙关联关系。
    subject_accounts: 涉案主体账户列表，如为None则自动从数据中获取所有交易卡号。
    """
    if subject_accounts is None:
        subject_accounts = df[col_self_account].unique().tolist()

    # 统计每个对手与主体的关联特征
    opp_stats = df.dropna(subset=[col_opp_account]).groupby(col_opp_account).agg(
        opp_name=(col_opp_name, 'first'),
        txn_count=(col_amount, 'count'),
        total_amount=(col_amount, 'sum'),
        first_time=(col_time, 'min'),
        last_time=(col_time, 'max'),
    )

    # 交易方向
    for opp_acct in opp_stats.index:
        opp_txns = df[df[col_opp_account] == opp_acct]
        has_in = (opp_txns[col_direction] == dir_in).any()
        has_out = (opp_txns[col_direction] == dir_out).any()
        opp_stats.loc[opp_acct, 'direction'] = '双向' if (has_in and has_out) else ('进' if has_in else '出')

    total_amount = df[col_amount].sum()
    opp_stats['amount_ratio'] = opp_stats['total_amount'] / total_amount

    # 设备共享检测
    opp_stats['shared_device'] = False
    if col_ip in df.columns:
        subject_ips = set()
        for acct in subject_accounts:
            acct_ips = df[df[col_self_account] == acct][col_ip].dropna().unique()
            subject_ips.update(acct_ips)

        for opp_acct in opp_stats.index:
            opp_as_self = df[df[col_self_account] == opp_acct]
            if len(opp_as_self) > 0:
                opp_ips = set(opp_as_self[col_ip].dropna().unique())
                if opp_ips & subject_ips:
                    opp_stats.loc[opp_acct, 'shared_device'] = True

    # 同地域检测
    opp_stats['same_region'] = False
    if col_opp_id in df.columns:
        subject_ids = set()
        for acct in subject_accounts:
            acct_ids = df[df[col_self_account] == acct].get('交易证件号', pd.Series()).dropna().unique()
            subject_ids.update([str(i)[:6] for i in acct_ids if len(str(i)) >= 6])

        for opp_acct in opp_stats.index:
            opp_id_vals = df[df[col_opp_account] == opp_acct][col_opp_id].dropna().unique()
            opp_prefixes = {str(i)[:6] for i in opp_id_vals if len(str(i)) >= 6}
            if opp_prefixes & subject_ids:
                opp_stats.loc[opp_acct, 'same_region'] = True

    # 关联强度分级
    def classify_relation(row):
        score = 0
        if row['direction'] == '双向':
            score += 1
        if row['shared_device']:
            score += 1
        if row['same_region']:
            score += 1
        if row['amount_ratio'] >= 0.05:
            score += 1
        if row['txn_count'] >= 20:
            score += 1

        if score >= 2:
            return '核心成员'
        elif row['amount_ratio'] >= 0.05 or row['txn_count'] >= 20:
            return '紧密关联'
        elif row['txn_count'] >= 3:
            return '一般关联'
        else:
            return '外围成员'

    opp_stats['relation_level'] = opp_stats.apply(classify_relation, axis=1)

    # 输出
    print(f"\n=== 对手团伙关系分析 ===")
    for level in ['核心成员', '紧密关联', '一般关联', '外围成员']:
        level_df = opp_stats[opp_stats['relation_level'] == level]
        print(f"\n{level}: {len(level_df)}个对手")
        for acct, row in level_df.sort_values('total_amount', ascending=False).head(10).iterrows():
            print(f"  {row['opp_name']}({acct}): {row['txn_count']}笔, "
                  f"{row['total_amount']:,.2f}元({row['amount_ratio']*100:.1f}%), "
                  f"方向={row['direction']}, 设备共享={row['shared_device']}, "
                  f"同地域={row['same_region']}")

    return opp_stats

# opp_relations = analyze_counterparty_gang_relations(
#     df, COL['self_account'], COL['opp_account'], COL['opp_name'],
#     COL['amount'], COL['direction'], COL['time'],
#     COL['ip'], COL['mac'], COL['opp_id'],
#     DIR_IN, DIR_OUT
# )
```

### 对手间横向关联分析

```python
def analyze_inter_counterparty_relations(df, col_self_account, col_opp_account, col_opp_name,
                                            col_amount, col_direction, col_time,
                                            col_ip, col_opp_id, dir_out):
    """
    分析主要对手之间的横向关联（不经过涉案主体的关联）。
    """
    # 获取有自身交易记录的对手
    all_self_accounts = set(df[col_self_account].unique())
    all_opp_accounts = set(df[col_opp_account].dropna().unique())
    opp_with_records = all_self_accounts & all_opp_accounts

    relations = []

    if len(opp_with_records) < 2:
        print("交易明细中作为主体出现的对手不足2个，无法分析横向关联")
        return pd.DataFrame()

    opp_list = sorted(opp_with_records)

    for i in range(len(opp_list)):
        for j in range(i + 1, len(opp_list)):
            a, b = opp_list[i], opp_list[j]
            reasons = []

            # 1. 相互交易
            mutual = df[
                ((df[col_self_account] == a) & (df[col_opp_account] == b)) |
                ((df[col_self_account] == b) & (df[col_opp_account] == a))
            ]
            if len(mutual) > 0:
                reasons.append(f'相互交易{len(mutual)}笔/{mutual[col_amount].sum():,.0f}元')

            # 2. IP共享
            if col_ip in df.columns:
                ips_a = set(df[df[col_self_account] == a][col_ip].dropna().unique())
                ips_b = set(df[df[col_self_account] == b][col_ip].dropna().unique())
                common_ips = ips_a & ips_b
                if common_ips:
                    reasons.append(f'共享IP {len(common_ips)}个')

            # 3. 同地域
            if col_opp_id in df.columns:
                ids_a = df[df[col_opp_account] == a][col_opp_id].dropna().unique()
                ids_b = df[df[col_opp_account] == b][col_opp_id].dropna().unique()
                prefix_a = {str(i)[:6] for i in ids_a if len(str(i)) >= 6}
                prefix_b = {str(i)[:6] for i in ids_b if len(str(i)) >= 6}
                if prefix_a & prefix_b:
                    reasons.append('同地域')

            # 4. 同时段操作
            times_a = set(df[df[col_self_account] == a][col_time].dt.date.unique())
            times_b = set(df[df[col_self_account] == b][col_time].dt.date.unique())
            common_dates = times_a & times_b
            if len(common_dates) >= 5:
                reasons.append(f'同日操作{len(common_dates)}天')

            if reasons:
                name_a = df[df[col_opp_account] == a][col_opp_name].iloc[0] if len(df[df[col_opp_account] == a]) > 0 else a
                name_b = df[df[col_opp_account] == b][col_opp_name].iloc[0] if len(df[df[col_opp_account] == b]) > 0 else b
                relations.append({
                    'opp_a': a, 'name_a': name_a,
                    'opp_b': b, 'name_b': name_b,
                    'relation_types': '; '.join(reasons),
                })

    result = pd.DataFrame(relations)
    print(f"\n=== 对手间横向关联 ===")
    if len(result) == 0:
        print("未发现对手之间的横向关联")
    else:
        print(f"发现 {len(result)} 对横向关联:")
        for _, r in result.iterrows():
            print(f"  {r['name_a']} <-> {r['name_b']}: {r['relation_types']}")

    return result

# inter_relations = analyze_inter_counterparty_relations(
#     df, COL['self_account'], COL['opp_account'], COL['opp_name'],
#     COL['amount'], COL['direction'], COL['time'],
#     COL['ip'], COL['opp_id'], DIR_OUT
# )
```

---

## 5. 可疑特征量化（阶段D）

### 核心指标自动计算

```python
results = {}

# 1. 收支平衡度
results['收支平衡度'] = abs(in_amount - out_amount) / (in_amount + out_amount) if (in_amount + out_amount) > 0 else None

# 2. 现金交易占比
if COL.get('cash_flag') and COL['cash_flag'] in df.columns:
    cash_mask = df[COL['cash_flag']].astype(str).str.contains('现')
    results['现金交易占比'] = cash_mask.sum() / total_count
else:
    results['现金交易占比'] = None

# 3. 对手集中度(来源)
if len(in_top) >= 3:
    results['来源集中度TOP3'] = in_top['total'].head(3).sum() / in_amount
else:
    results['来源集中度TOP3'] = in_top['total'].sum() / in_amount if in_amount > 0 else None

# 4. 对手集中度(去向)
if len(out_top) >= 3:
    results['去向集中度TOP3'] = out_top['total'].head(3).sum() / out_amount
else:
    results['去向集中度TOP3'] = out_top['total'].sum() / out_amount if out_amount > 0 else None

# 5. 同名账户数
results['同名账户数'] = df[df[COL['opp_name']] == SELF_NAME][COL['opp_account']].nunique()

# 6. 整额交易占比（整万）
round_10k = df[df[COL['amount']] % 10000 == 0]
results['整万交易占比'] = len(round_10k) / total_count if total_count > 0 else 0
# 整千占比
round_1k = df[df[COL['amount']] % 1000 == 0]
results['整千交易占比'] = len(round_1k) / total_count if total_count > 0 else 0

# 7. 快进快出天数占比
df['date'] = df[COL['time']].dt.date
daily_in = df_in.groupby(df_in[COL['time']].dt.date)[COL['amount']].sum()
daily_out = df_out.groupby(df_out[COL['time']].dt.date)[COL['amount']].sum()
daily = pd.DataFrame({'in': daily_in, 'out': daily_out}).fillna(0)
quick_days = ((daily['in'] > 50000) & (daily['out'] > 50000)).sum()
total_days = df['date'].nunique()
results['快进快出天数占比'] = quick_days / total_days if total_days > 0 else 0

# 8. 资金周转率
if COL['balance'] in df.columns:
    avg_balance = pd.to_numeric(df[COL['balance']], errors='coerce').mean()
    results['资金周转率'] = total_amount / avg_balance if avg_balance > 0 else None
else:
    results['资金周转率'] = None

# 9. 可疑关键词
if COL.get('memo') and COL['memo'] in df.columns:
    keywords = ['换汇', '换钱', '换币', '兑换', '换美金', '换美元', '外汇']
    pattern = '|'.join(keywords)
    keyword_mask = df[COL['memo']].astype(str).str.contains(pattern, na=False)
    results['可疑关键词笔数'] = keyword_mask.sum()
else:
    results['可疑关键词笔数'] = None

# 10. 地域集中度（补充指标）
if COL.get('opp_id') and COL['opp_id'] in df.columns:
    opp_ids = df.dropna(subset=[COL['opp_account'], COL['opp_id']]).drop_duplicates(subset=[COL['opp_account']])
    opp_ids['prefix6'] = opp_ids[COL['opp_id']].astype(str).str[:6]
    valid_ids = opp_ids[opp_ids['prefix6'].str.match(r'^\d{6}', na=False)]
    if len(valid_ids) > 0:
        max_region_count = valid_ids['prefix6'].value_counts().max()
        results['地域集中度'] = max_region_count / len(valid_ids)
    else:
        results['地域集中度'] = None
else:
    results['地域集中度'] = None

# 11. 对手开户银行集中度（补充指标）
if COL.get('opp_bank') and COL['opp_bank'] in df.columns:
    opp_banks = df.dropna(subset=[COL['opp_account'], COL['opp_bank']]).drop_duplicates(subset=[COL['opp_account']])
    if len(opp_banks) > 0:
        top1_bank = opp_banks[COL['opp_bank']].value_counts().max()
        results['开户银行集中度'] = top1_bank / len(opp_banks)
    else:
        results['开户银行集中度'] = None
else:
    results['开户银行集中度'] = None
```

### 阈值判定与输出

```python
thresholds = {
    '收支平衡度':        {'normal': 0.20, 'abnormal': 0.05, 'high': 0.01, 'direction': 'lower'},
    '现金交易占比':      {'normal': 0.20, 'abnormal': 0.50, 'high': 0.70, 'direction': 'higher'},
    '来源集中度TOP3':    {'normal': 0.30, 'abnormal': 0.40, 'high': 0.60, 'direction': 'higher'},
    '去向集中度TOP3':    {'normal': 0.30, 'abnormal': 0.40, 'high': 0.60, 'direction': 'higher'},
    '同名账户数':        {'normal': 0,    'abnormal': 2,    'high': 5,    'direction': 'higher'},
    '整万交易占比':      {'normal': 0.15, 'abnormal': 0.30, 'high': 0.45, 'direction': 'higher'},
    '快进快出天数占比':  {'normal': 0.05, 'abnormal': 0.10, 'high': 0.20, 'direction': 'higher'},
    '资金周转率':        {'normal': 20,   'abnormal': 50,   'high': 100,  'direction': 'higher'},
    '可疑关键词笔数':    {'normal': 0,    'abnormal': 1,    'high': 10,   'direction': 'higher'},
    '地域集中度':        {'normal': 0.10, 'abnormal': 0.20, 'high': 0.30, 'direction': 'higher'},
    '开户银行集中度':    {'normal': 0.20, 'abnormal': 0.30, 'high': 0.40, 'direction': 'higher'},
}

print("\n=== 可疑指标判定结果 ===")
for name, value in results.items():
    if value is None:
        print(f"  {name}: 无法计算（缺少数据）")
        continue

    th = thresholds.get(name)
    if th is None:
        print(f"  {name}: {value}")
        continue

    if th['direction'] == 'lower':
        if value < th['high']:
            level = '高度异常'
        elif value < th['abnormal']:
            level = '异常'
        else:
            level = '正常'
    else:
        if value >= th['high']:
            level = '高度异常'
        elif value >= th['abnormal']:
            level = '异常'
        else:
            level = '正常'

    if isinstance(value, float) and value < 1:
        display = f"{value*100:.2f}%"
    else:
        display = f"{value}"

    marker = '高度异常' if level == '高度异常' else ('异常' if level == '异常' else '正常')
    print(f"  [{marker}] {name}: {display} → {level}")
```

---

## 6. 快进快出检测（30分钟窗口）

```python
def find_quick_pairs(df, col_time, col_amount, col_direction, dir_in, dir_out, window_minutes=30):
    """
    检测30分钟内的快进快出配对。
    返回配对列表 [(进账时间, 进账金额, 出账时间, 出账金额, 间隔分钟), ...]
    """
    df_sorted = df.sort_values(col_time).copy()
    ins = df_sorted[df_sorted[col_direction] == dir_in][[col_time, col_amount]].values
    outs = df_sorted[df_sorted[col_direction] == dir_out][[col_time, col_amount]].values

    pairs = []
    used_outs = set()

    for in_time, in_amt in ins:
        if in_amt < 50000:  # 只关注大额
            continue
        for j, (out_time, out_amt) in enumerate(outs):
            if j in used_outs or out_amt < 50000:
                continue
            diff = (out_time - in_time)
            if hasattr(diff, 'total_seconds'):
                diff_minutes = diff.total_seconds() / 60
            else:
                diff_minutes = float(diff) / 60e9  # numpy timedelta

            if 0 < diff_minutes <= window_minutes:
                pairs.append((in_time, float(in_amt), out_time, float(out_amt), diff_minutes))
                used_outs.add(j)
                break

    return pairs

pairs = find_quick_pairs(
    df, COL['time'], COL['amount'], COL['direction'], DIR_IN, DIR_OUT, window_minutes=30
)

print(f"\n=== 30分钟快进快出配对: {len(pairs)}对 ===")
for in_t, in_a, out_t, out_a, mins in pairs[:20]:
    print(f"  进: {in_t} ({in_a:,.0f}) → 出: {out_t} ({out_a:,.0f}), 间隔{mins:.0f}分钟")
```

---

## 7. 资金穿透分析

```python
def fund_penetration(df_main, target_accounts, col_self, col_opp, col_amount, col_direction, dir_in, dir_out, depth=1):
    """
    从交易明细中进行资金穿透分析。
    target_accounts: 待穿透的账户列表
    对每个账户，统计其作为交易卡号时的上/下游资金关系。
    """
    result = {}
    for acct in target_accounts:
        # 作为交易卡号的记录
        acct_data = df_main[df_main[col_self] == acct]
        if len(acct_data) == 0:
            # 也检查交易账号
            acct_data = df_main[df_main.get('交易账号', col_self)] if '交易账号' in df_main.columns else pd.DataFrame()
            if len(acct_data) == 0:
                continue

        # 上游（给 acct 转入的来源）
        upstream = acct_data[acct_data[col_direction] == dir_in].groupby(col_opp)[col_amount].agg(['sum', 'count']).sort_values('sum', ascending=False)
        # 下游（acct 转出的去向）
        downstream = acct_data[acct_data[col_direction] == dir_out].groupby(col_opp)[col_amount].agg(['sum', 'count']).sort_values('sum', ascending=False)

        result[acct] = {
            'upstream_top5': upstream.head(5),
            'downstream_top5': downstream.head(5),
            'total_in': acct_data[acct_data[col_direction] == dir_in][col_amount].sum(),
            'total_out': acct_data[acct_data[col_direction] == dir_out][col_amount].sum(),
        }

    return result

# 对TOP来源和去向的账户做穿透
top_sources = in_top.head(5).index.get_level_values(0).tolist()
top_targets = out_top.head(5).index.get_level_values(0).tolist()

penetration = fund_penetration(
    df,
    top_sources + top_targets,
    COL['self_account'],
    COL['opp_account'],
    COL['amount'],
    COL['direction'],
    DIR_IN, DIR_OUT
)

for acct, info in penetration.items():
    print(f"\n--- {acct} ---")
    print(f"总收入: {info['total_in']:,.2f}, 总支出: {info['total_out']:,.2f}")
    if len(info['upstream_top5']) > 0:
        print("  TOP5上游:")
        print(info['upstream_top5'].to_string())
    if len(info['downstream_top5']) > 0:
        print("  TOP5下游:")
        print(info['downstream_top5'].to_string())
```

---

## 8. 报告生成辅助

### Markdown 格式化表格

```python
def to_md_table(headers, rows):
    """将数据转为 Markdown 表格字符串"""
    lines = []
    lines.append('| ' + ' | '.join(str(h) for h in headers) + ' |')
    lines.append('| ' + ' | '.join('---' for _ in headers) + ' |')
    for row in rows:
        lines.append('| ' + ' | '.join(str(v) for v in row) + ' |')
    return '\n'.join(lines)

# 使用示例
headers = ['序号', '对手账户', '对手户名', '交易笔数', '合计金额', '平均单笔']
rows = []
for i, ((acct, name), row) in enumerate(in_top.head(10).iterrows(), 1):
    rows.append([i, acct, name, row['count'], f"{row['total']:,.2f}", f"{row['avg']:,.2f}"])

table_str = to_md_table(headers, rows)
print(table_str)
```

### Word 报告生成（使用 python-docx）

```python
from docx import Document
from docx.shared import Pt, Cm, RGBColor
from docx.enum.text import WD_PARAGRAPH_ALIGNMENT

def create_report():
    doc = Document()

    # 标题
    title = doc.add_heading('资金分析报告', level=0)
    title.alignment = WD_PARAGRAPH_ALIGNMENT.CENTER

    # 正文段落
    doc.add_heading('一、基本情况', level=1)
    doc.add_heading('（一）账户基本信息', level=2)
    doc.add_paragraph('此处填写账户基本信息...')

    # 表格
    table = doc.add_table(rows=1, cols=4)
    table.style = 'Table Grid'
    headers = table.rows[0].cells
    headers[0].text = '项目'
    headers[1].text = '笔数'
    headers[2].text = '金额'
    headers[3].text = '占比'

    # 添加数据行
    row = table.add_row().cells
    row[0].text = '收入'
    row[1].text = str(in_count)
    row[2].text = f'{in_amount:,.2f}'
    row[3].text = f'{in_amount/total_amount*100:.1f}%'

    doc.save('资金分析报告.docx')
    print("报告已生成: 资金分析报告.docx")

# create_report()  # 取消注释执行
```

### f-string 防错规范（重要）

生成 Word 报告时，所有包含变量引用的字符串**必须使用 f-string**（在引号前加 `f` 前缀），否则 `{变量名}` 会作为字面量原样输出到文档中。

**错误写法**（变量不会被求值，直接输出 `{r["in_accounts"]}` 这样的文字）：
```python
doc.add_paragraph('该账户收入{in_count}笔，支出{out_count}笔')  # 缺少 f 前缀！
doc.add_paragraph('上游{r["in_accounts"]}个对手')                # 缺少 f 前缀！
```

**正确写法**（变量会被正确求值并替换为真实数据）：
```python
doc.add_paragraph(f'该账户收入{in_count}笔，支出{out_count}笔')
doc.add_paragraph(f'上游{r["in_accounts"]}个对手')
```

**检查方法**：生成报告后，打开 docx 文件搜索 `{` 字符。如果文档中出现 `{r[`、`{in_count}` 等原始占位符文本，说明有字符串遗漏了 `f` 前缀，需要逐一排查修复。

---

## 9. 账户分类（交易比值法） `[来源：技战法3"三取四辨"]`

```python
def classify_accounts_by_ratio(df, col_opp_account, col_opp_name, col_amount, col_direction, dir_in, dir_out):
    """
    基于交易比值（收入/(收入+支出)）对对手账户进行分类。
    返回DataFrame，包含每个对手账户的比值和分类。
    """
    # 按对手账户分别统计进出金额
    opp_in = df[df[col_direction] == dir_in].groupby(col_opp_account)[col_amount].sum()
    opp_out = df[df[col_direction] == dir_out].groupby(col_opp_account)[col_amount].sum()

    # 合并
    opp_stats = pd.DataFrame({
        'in_amount': opp_in,
        'out_amount': opp_out
    }).fillna(0)

    opp_stats['total'] = opp_stats['in_amount'] + opp_stats['out_amount']
    opp_stats['ratio'] = opp_stats['in_amount'] / opp_stats['total']
    opp_stats['ratio'] = opp_stats['ratio'].fillna(0)

    # 获取对手户名
    name_map = df.dropna(subset=[col_opp_account]).drop_duplicates(
        subset=[col_opp_account]
    ).set_index(col_opp_account)[col_opp_name].to_dict()
    opp_stats['name'] = opp_stats.index.map(name_map)

    # 分类
    def classify(ratio):
        if ratio >= 0.8:
            return '资金来源账户'
        elif ratio >= 0.6:
            return '偏来源型账户'
        elif ratio >= 0.4:
            return '过渡/中转账户'
        elif ratio >= 0.2:
            return '偏去向型账户'
        else:
            return '资金去向账户'

    opp_stats['category'] = opp_stats['ratio'].apply(classify)

    # 特殊标记
    opp_stats['special'] = ''
    opp_stats.loc[opp_stats['ratio'] == 1.0, 'special'] = '虚开通道账户'
    opp_stats.loc[opp_stats['ratio'] == 0.0, 'special'] = '钱庄指定账户'
    opp_stats.loc[opp_stats['ratio'] >= 0.99, 'special'] = opp_stats.loc[
        opp_stats['ratio'] >= 0.99, 'special'
    ].replace('', '高度可疑单向流入')

    return opp_stats.sort_values('total', ascending=False)

opp_classification = classify_accounts_by_ratio(
    df, COL['opp_account'], COL['opp_name'],
    COL['amount'], COL['direction'], DIR_IN, DIR_OUT
)

# 分类汇总
print("\n=== 对手账户分类汇总 ===")
summary = opp_classification.groupby('category').agg(
    count=('total', 'count'),
    total_amount=('total', 'sum')
)
for cat, row in summary.iterrows():
    print(f"  {cat}: {row['count']}个账户, 合计交易{row['total_amount']:,.2f}元")

# 特殊账户
specials = opp_classification[opp_classification['special'] != '']
if len(specials) > 0:
    print(f"\n特殊标记账户: {len(specials)}个")
    for acct, row in specials.head(20).iterrows():
        print(f"  {row['name']}({acct}): 比值={row['ratio']:.4f}, {row['special']}")
```

### 财务账户识别 `[来源：技战法3"三取四辨"]`

```python
def identify_financial_accounts(df, opp_stats, col_opp_account, col_balance, col_time):
    """
    识别疑似团伙财务账户：比值0.6~0.9 且 余额超10万次数≥3。
    需要交易余额字段。
    """
    if col_balance not in df.columns:
        print("无余额字段，跳过财务账户识别")
        return pd.DataFrame()

    candidates = opp_stats[(opp_stats['ratio'] >= 0.6) & (opp_stats['ratio'] <= 0.9)]
    financial_accounts = []

    for acct in candidates.index:
        acct_txns = df[df[col_opp_account] == acct]
        if len(acct_txns) == 0:
            continue
        balances = pd.to_numeric(acct_txns[col_balance], errors='coerce').dropna()
        high_balance_count = (balances > 100000).sum()
        if high_balance_count >= 3:
            financial_accounts.append({
                'account': acct,
                'name': candidates.loc[acct, 'name'],
                'ratio': candidates.loc[acct, 'ratio'],
                'high_balance_count': high_balance_count,
                'total_amount': candidates.loc[acct, 'total']
            })

    result = pd.DataFrame(financial_accounts)
    if len(result) > 0:
        print(f"\n=== 疑似财务账户: {len(result)}个 ===")
        for _, row in result.iterrows():
            print(f"  {row['name']}({row['account']}): 比值={row['ratio']:.2f}, "
                  f"余额>10万次数={row['high_balance_count']}, 交易额={row['total_amount']:,.2f}")
    return result

# financial = identify_financial_accounts(
#     df, opp_classification, COL['opp_account'], COL['balance'], COL['time']
# )
```

---

## 10. 对手重合度计算 `[来源：技战法2"星链"、技战法3"三取四辨"]`

```python
def calculate_counterparty_overlap(df, col_self_account, col_opp_account, col_direction, dir_out):
    """
    计算不同主体账户之间的出账对手Jaccard重合度。
    需要数据中包含多个主体账户的交易明细。
    返回：重合度矩阵和高重合度账户对列表。
    """
    accounts = df[col_self_account].unique()
    if len(accounts) < 2:
        print("数据中只有1个主体账户，无法计算对手重合度")
        return None, []

    # 每个主体账户的出账对手集合
    out_df = df[df[col_direction] == dir_out]
    account_opps = {}
    for acct in accounts:
        opps = set(out_df[out_df[col_self_account] == acct][col_opp_account].dropna().unique())
        if len(opps) > 0:
            account_opps[acct] = opps

    # 两两计算Jaccard重合度
    overlap_pairs = []
    acct_list = list(account_opps.keys())
    for i in range(len(acct_list)):
        for j in range(i + 1, len(acct_list)):
            a, b = acct_list[i], acct_list[j]
            set_a, set_b = account_opps[a], account_opps[b]
            intersection = len(set_a & set_b)
            union = len(set_a | set_b)
            jaccard = intersection / union if union > 0 else 0

            overlap_pairs.append({
                'account_a': a,
                'account_b': b,
                'opps_a': len(set_a),
                'opps_b': len(set_b),
                'intersection': intersection,
                'union': union,
                'jaccard': jaccard,
            })

    overlap_df = pd.DataFrame(overlap_pairs).sort_values('jaccard', ascending=False)

    # 输出高重合度结果
    print(f"\n=== 对手重合度分析（{len(acct_list)}个主体账户）===")
    high_overlap = overlap_df[overlap_df['jaccard'] >= 0.20]
    if len(high_overlap) > 0:
        print(f"重合度≥20%的账户对: {len(high_overlap)}对")
        for _, row in high_overlap.iterrows():
            level = '高度疑似钱庄团伙' if row['jaccard'] >= 0.30 else '可能同一团伙'
            print(f"  {row['account_a']} <-> {row['account_b']}: "
                  f"重合度={row['jaccard']*100:.1f}% ({row['intersection']}/{row['union']}), {level}")
    else:
        print("未发现重合度≥20%的账户对")

    return overlap_df, high_overlap

# overlap_df, high_overlap = calculate_counterparty_overlap(
#     df, COL['self_account'], COL['opp_account'], COL['direction'], DIR_OUT
# )

# 基于高重合度的账户对用Union-Find归入团伙
# if high_overlap is not None and len(high_overlap) > 0:
#     uf_overlap = UnionFind()
#     for _, row in high_overlap.iterrows():
#         uf_overlap.union(row['account_a'], row['account_b'])
#     overlap_gangs = uf_overlap.get_groups()
#     for root, members in overlap_gangs.items():
#         if len(members) >= 2:
#             print(f"  重合度团伙: {members}")
```

### 共同对手交易信息统计

```python
def detail_common_counterparties(df, account_a, account_b, col_self_account, col_opp_account,
                                   col_opp_name, col_amount, col_direction, dir_in, dir_out):
    """
    对高重合度的账户对，列出共同对手的详细交易统计表。
    """
    df_a = df[df[col_self_account] == account_a]
    df_b = df[df[col_self_account] == account_b]

    opps_a = set(df_a[col_opp_account].dropna().unique())
    opps_b = set(df_b[col_opp_account].dropna().unique())
    common_opps = opps_a & opps_b

    if not common_opps:
        print("无共同对手")
        return pd.DataFrame()

    rows = []
    for opp in common_opps:
        opp_name = df[df[col_opp_account] == opp][col_opp_name].iloc[0] if len(df[df[col_opp_account] == opp]) > 0 else '未知'

        # 与账户A的交易
        a_txns = df_a[df_a[col_opp_account] == opp]
        a_count = len(a_txns)
        a_amount = a_txns[col_amount].sum()
        a_in = (a_txns[col_direction] == dir_in).any()
        a_out = (a_txns[col_direction] == dir_out).any()
        a_dir = '双向' if (a_in and a_out) else ('进' if a_in else '出')

        # 与账户B的交易
        b_txns = df_b[df_b[col_opp_account] == opp]
        b_count = len(b_txns)
        b_amount = b_txns[col_amount].sum()
        b_in = (b_txns[col_direction] == dir_in).any()
        b_out = (b_txns[col_direction] == dir_out).any()
        b_dir = '双向' if (b_in and b_out) else ('进' if b_in else '出')

        rows.append({
            'opp_name': opp_name, 'opp_account': opp,
            'a_count': a_count, 'a_amount': a_amount, 'a_dir': a_dir,
            'b_count': b_count, 'b_amount': b_amount, 'b_dir': b_dir,
            'total_amount': a_amount + b_amount,
        })

    result = pd.DataFrame(rows).sort_values('total_amount', ascending=False)

    print(f"\n=== 共同对手交易统计 ({account_a} vs {account_b}) ===")
    print(f"共同对手数: {len(result)}")
    for _, r in result.head(20).iterrows():
        print(f"  {r['opp_name']}({r['opp_account']}): "
              f"与A {r['a_count']}笔/{r['a_amount']:,.2f}元({r['a_dir']}), "
              f"与B {r['b_count']}笔/{r['b_amount']:,.2f}元({r['b_dir']}), "
              f"合计{r['total_amount']:,.2f}元")

    return result

# common_detail = detail_common_counterparties(
#     df, account_a, account_b, COL['self_account'], COL['opp_account'],
#     COL['opp_name'], COL['amount'], COL['direction'], DIR_IN, DIR_OUT
# )
```

---

## 11. 账户休眠-激活检测 `[来源：技战法1"四步"、技战法8"比判分锁"]`

```python
def detect_dormant_activation(df, col_self_account, col_time, dormant_days=30, active_threshold=5):
    """
    检测账户的休眠-激活模式。
    dormant_days: 连续无交易天数阈值（默认30天）
    active_threshold: 激活后日均交易笔数阈值（默认5笔/天）
    返回每个账户的休眠-激活周期列表。
    """
    results = {}
    accounts = df[col_self_account].unique()

    for acct in accounts:
        acct_df = df[df[col_self_account] == acct].sort_values(col_time)
        if len(acct_df) < 2:
            continue

        # 按日统计交易笔数
        daily_counts = acct_df.groupby(acct_df[col_time].dt.date).size()
        date_range = pd.date_range(daily_counts.index.min(), daily_counts.index.max(), freq='D')
        daily_full = pd.Series(0, index=date_range.date)
        for d, c in daily_counts.items():
            daily_full[d] = c

        # 检测连续无交易的天段（休眠期）
        cycles = []
        dormant_start = None
        dormant_streak = 0

        for i, (date, count) in enumerate(daily_full.items()):
            if count == 0:
                if dormant_start is None:
                    dormant_start = date
                dormant_streak += 1
            else:
                if dormant_streak >= dormant_days:
                    # 休眠结束，检查激活期（后续7天的日均交易）
                    activate_start = date
                    next_7_days = daily_full.iloc[i:i+7]
                    avg_daily = next_7_days.mean() if len(next_7_days) > 0 else 0

                    if avg_daily >= active_threshold:
                        cycles.append({
                            'dormant_start': dormant_start,
                            'dormant_end': date,
                            'dormant_days': dormant_streak,
                            'activate_start': activate_start,
                            'activate_avg_daily': avg_daily,
                        })
                dormant_start = None
                dormant_streak = 0

        if cycles:
            results[acct] = cycles

    # 输出
    print(f"\n=== 休眠-激活检测 ===")
    if not results:
        print("未发现明显的休眠-激活模式")
    for acct, cycles in results.items():
        print(f"\n账户 {acct}: {len(cycles)}个休眠-激活周期")
        for c in cycles:
            print(f"  休眠: {c['dormant_start']} ~ {c['dormant_end']} ({c['dormant_days']}天)")
            print(f"  激活: {c['activate_start']} 开始, 日均{c['activate_avg_daily']:.1f}笔")

    return results

# dormant_results = detect_dormant_activation(df, COL['self_account'], COL['time'])
```

---

## 12. 小额试探交易检测 `[来源：技战法1"四步"、技战法8"比判分锁"]`

```python
def detect_probe_transactions(df, col_time, col_amount, col_direction, col_opp_account,
                               dir_in, small_threshold=20, large_threshold=10000,
                               window_hours=24):
    """
    检测小额试探交易：进账方向中≤small_threshold的交易后，
    window_hours内是否有同一对手（或任何对手）的≥large_threshold大额交易。
    """
    df_in_sorted = df[df[col_direction] == dir_in].sort_values(col_time).copy()
    small_txns = df_in_sorted[df_in_sorted[col_amount] <= small_threshold]
    large_txns = df_in_sorted[df_in_sorted[col_amount] >= large_threshold]

    pairs = []
    for _, small in small_txns.iterrows():
        small_time = small[col_time]
        small_opp = small[col_opp_account]
        window_end = small_time + pd.Timedelta(hours=window_hours)

        # 查找窗口内的大额交易
        following_large = large_txns[
            (large_txns[col_time] > small_time) &
            (large_txns[col_time] <= window_end)
        ]

        if len(following_large) > 0:
            first_large = following_large.iloc[0]
            interval_hours = (first_large[col_time] - small_time).total_seconds() / 3600
            pairs.append({
                'probe_time': small_time,
                'probe_amount': small[col_amount],
                'probe_opp': small_opp,
                'large_time': first_large[col_time],
                'large_amount': first_large[col_amount],
                'large_opp': first_large[col_opp_account],
                'interval_hours': interval_hours,
                'same_opp': small_opp == first_large[col_opp_account],
            })

    print(f"\n=== 小额试探交易检测 ===")
    print(f"小额(≤{small_threshold}元)交易总数: {len(small_txns)}")
    print(f"其中后续有大额(≥{large_threshold}元)交易的配对: {len(pairs)}对")

    for p in pairs[:20]:
        same_mark = '[同对手]' if p['same_opp'] else '[不同对手]'
        print(f"  试探: {p['probe_time']} {p['probe_amount']:.0f}元({p['probe_opp']}) "
              f"→ 大额: {p['large_time']} {p['large_amount']:,.0f}元({p['large_opp']}) "
              f"间隔{p['interval_hours']:.1f}h {same_mark}")

    return pairs

# probe_pairs = detect_probe_transactions(
#     df, COL['time'], COL['amount'], COL['direction'],
#     COL['opp_account'], DIR_IN
# )
```

---

## 13. 规避监控金额检测 `[来源：技战法1"四步"]`

```python
def detect_threshold_avoidance(df, col_amount, col_opp_name, report_threshold=50000,
                                lower_bound=45000, upper_bound=49999):
    """
    检测规避大额交易报告阈值的交易。
    默认检测45000~49999元区间的交易占比。
    """
    total = len(df)
    avoidance_mask = (df[col_amount] >= lower_bound) & (df[col_amount] <= upper_bound)
    avoidance_count = avoidance_mask.sum()
    avoidance_amount = df.loc[avoidance_mask, col_amount].sum()

    ratio = avoidance_count / total if total > 0 else 0

    print(f"\n=== 规避监控金额检测 ===")
    print(f"大额报告阈值: {report_threshold:,}元")
    print(f"{lower_bound:,}~{upper_bound:,}元区间交易: {avoidance_count}笔, "
          f"金额{avoidance_amount:,.2f}元")
    print(f"占总交易笔数: {ratio*100:.2f}%")

    if ratio > 0.05:
        print("→ 高度异常: 大量交易集中在监控阈值下方")
    elif ratio > 0.03:
        print("→ 异常: 较多交易接近监控阈值")
    elif ratio > 0.01:
        print("→ 需关注: 部分交易接近监控阈值")
    else:
        print("→ 正常")

    # 列出这些交易的时间和对手分布
    if avoidance_count > 0 and col_opp_name in df.columns:
        avoidance_df = df[avoidance_mask]
        print(f"\n规避金额交易的对手分布:")
        opp_dist = avoidance_df.groupby(col_opp_name).size().sort_values(ascending=False)
        for name, cnt in opp_dist.head(10).items():
            print(f"  {name}: {cnt}笔")

    return {'count': avoidance_count, 'amount': avoidance_amount, 'ratio': ratio}

# avoidance = detect_threshold_avoidance(df, COL['amount'], COL['opp_name'])
```

---

## 14. 通用摘要识别 `[来源：技战法8"比判分锁"]`

```python
def analyze_generic_memos(df, col_memo, col_amount, col_direction, dir_in, dir_out):
    """
    分析摘要说明字段中通用词汇的使用情况。
    通用词汇表明交易可能使用虚假摘要掩饰资金性质。
    """
    if col_memo not in df.columns:
        print("无摘要字段，跳过通用摘要分析")
        return None

    generic_keywords = ['货款', '往来款', '采购款', '投资', '服务费', '汇款', '购货',
                        '贸易款', '借款', '还款', '咨询费', '工程款', '材料款']
    pattern = '|'.join(generic_keywords)

    memo_filled = df[col_memo].fillna('')
    generic_mask = memo_filled.astype(str).str.contains(pattern, na=False)
    empty_mask = memo_filled.astype(str).str.strip() == ''

    total = len(df)
    generic_count = generic_mask.sum()
    empty_count = empty_mask.sum()
    generic_amount = df.loc[generic_mask, col_amount].sum()

    print(f"\n=== 通用摘要分析 ===")
    print(f"通用摘要交易: {generic_count}笔 ({generic_count/total*100:.1f}%), "
          f"金额{generic_amount:,.2f}元")
    print(f"空白摘要交易: {empty_count}笔 ({empty_count/total*100:.1f}%)")
    print(f"合计(通用+空白): {generic_count+empty_count}笔 "
          f"({(generic_count+empty_count)/total*100:.1f}%)")

    # 各关键词出现频次
    print(f"\n通用关键词出现频次:")
    for kw in generic_keywords:
        kw_count = memo_filled.astype(str).str.contains(kw, na=False).sum()
        if kw_count > 0:
            print(f"  {kw}: {kw_count}笔")

    # 按进出方向分别统计
    for dir_label, dir_val in [('收入', dir_in), ('支出', dir_out)]:
        dir_df = df[df[col_direction] == dir_val]
        dir_generic = dir_df[col_memo].fillna('').astype(str).str.contains(pattern, na=False).sum()
        print(f"  {dir_label}端通用摘要: {dir_generic}/{len(dir_df)} "
              f"({dir_generic/len(dir_df)*100:.1f}%)" if len(dir_df) > 0 else f"  {dir_label}端无数据")

    return {
        'generic_count': generic_count,
        'generic_ratio': generic_count / total if total > 0 else 0,
        'empty_count': empty_count,
        'generic_amount': generic_amount,
    }

# memo_analysis = analyze_generic_memos(
#     df, COL['memo'], COL['amount'], COL['direction'], DIR_IN, DIR_OUT
# )
```

---

## 15. 资金回流检测 `[来源：技战法7"比穿联汇"]`

```python
def detect_fund_loop(df, col_self_account, col_opp_account, col_amount,
                      col_direction, col_time, dir_in, dir_out, max_depth=3):
    """
    检测资金回流（A→B→C→A的环形资金流）。
    需要数据中包含多个账户的交易明细。
    max_depth: 最大追踪层数
    """
    accounts = set(df[col_self_account].unique())
    loops = []

    # 构建有向图：从每个账户的出账对手关系
    out_graph = {}
    for acct in accounts:
        acct_out = df[(df[col_self_account] == acct) & (df[col_direction] == dir_out)]
        targets = acct_out[col_opp_account].dropna().unique()
        out_graph[acct] = set(targets) & accounts  # 只考虑数据内的账户

    # DFS检测环
    for start in accounts:
        visited = {start}
        stack = [(start, [start])]

        while stack:
            current, path = stack.pop()
            if current not in out_graph:
                continue

            for next_acct in out_graph[current]:
                if next_acct == start and len(path) >= 2:
                    # 发现回流
                    loop_path = path + [start]

                    # 计算回流金额
                    total_flow = 0
                    for k in range(len(loop_path) - 1):
                        a, b = loop_path[k], loop_path[k+1]
                        flow = df[(df[col_self_account] == a) &
                                  (df[col_opp_account] == b) &
                                  (df[col_direction] == dir_out)][col_amount].sum()
                        total_flow += flow

                    loops.append({
                        'path': ' → '.join(str(p) for p in loop_path),
                        'depth': len(path),
                        'total_flow': total_flow,
                    })
                elif next_acct not in visited and len(path) < max_depth:
                    visited.add(next_acct)
                    stack.append((next_acct, path + [next_acct]))

    print(f"\n=== 资金回流检测 ===")
    if not loops:
        print("未发现资金回流路径")
    else:
        print(f"发现 {len(loops)} 条回流路径:")
        for loop in sorted(loops, key=lambda x: -x['total_flow'])[:20]:
            print(f"  路径: {loop['path']}")
            print(f"  层数: {loop['depth']}, 涉及金额: {loop['total_flow']:,.2f}元")

    return loops

# loops = detect_fund_loop(
#     df, COL['self_account'], COL['opp_account'],
#     COL['amount'], COL['direction'], COL['time'], DIR_IN, DIR_OUT
# )
```

---

## 16. 多维评分模型 `[来源：技战法2"星链"]`

```python
def multi_dimension_scoring(df, results, col_opp_name, col_opp_id, col_time,
                             col_amount, col_memo, col_ip, col_mac, col_balance,
                             col_self_account, col_direction, dir_in, dir_out):
    """
    对可疑账户进行13维度综合评分。
    results: 前面计算的可疑指标字典
    返回总分和风险等级。
    """
    scores = {}
    total_dims = 0

    # 1. 收支平衡度 (0~3分)
    val = results.get('收支平衡度')
    if val is not None:
        total_dims += 1
        if val < 0.01:
            scores['收支平衡度'] = 3
        elif val < 0.05:
            scores['收支平衡度'] = 2
        elif val < 0.20:
            scores['收支平衡度'] = 1
        else:
            scores['收支平衡度'] = 0

    # 2. 日终余额趋零 (0~3分)
    if col_balance in df.columns:
        total_dims += 1
        df_sorted = df.sort_values(col_time)
        daily_last_balance = df_sorted.groupby(df_sorted[col_time].dt.date)[col_balance].last()
        daily_last_balance = pd.to_numeric(daily_last_balance, errors='coerce').dropna()
        if len(daily_last_balance) > 0:
            zero_ratio = (daily_last_balance < 1000).sum() / len(daily_last_balance)
            if zero_ratio > 0.70:
                scores['日终余额趋零'] = 3
            elif zero_ratio > 0.50:
                scores['日终余额趋零'] = 2
            elif zero_ratio > 0.30:
                scores['日终余额趋零'] = 1
            else:
                scores['日终余额趋零'] = 0
            results['日终余额趋零占比'] = zero_ratio

    # 3. 快进快出频率 (0~3分)
    val = results.get('快进快出天数占比')
    if val is not None:
        total_dims += 1
        if val > 0.20:
            scores['快进快出频率'] = 3
        elif val > 0.10:
            scores['快进快出频率'] = 2
        elif val > 0.05:
            scores['快进快出频率'] = 1
        else:
            scores['快进快出频率'] = 0

    # 4. 资金周转率 (0~3分)
    val = results.get('资金周转率')
    if val is not None:
        total_dims += 1
        if val > 100:
            scores['资金周转率'] = 3
        elif val > 50:
            scores['资金周转率'] = 2
        elif val > 20:
            scores['资金周转率'] = 1
        else:
            scores['资金周转率'] = 0

    # 5. 整额交易占比 (0~3分)
    val = results.get('整万交易占比')
    if val is not None:
        total_dims += 1
        if val > 0.45:
            scores['整额交易占比'] = 3
        elif val > 0.30:
            scores['整额交易占比'] = 2
        elif val > 0.15:
            scores['整额交易占比'] = 1
        else:
            scores['整额交易占比'] = 0

    # 6. 规避监控金额 (0~3分)
    total_dims += 1
    avoidance_mask = (df[col_amount] >= 45000) & (df[col_amount] <= 49999)
    avoidance_ratio = avoidance_mask.sum() / len(df) if len(df) > 0 else 0
    if avoidance_ratio > 0.05:
        scores['规避监控金额'] = 3
    elif avoidance_ratio > 0.03:
        scores['规避监控金额'] = 2
    elif avoidance_ratio > 0.01:
        scores['规避监控金额'] = 1
    else:
        scores['规避监控金额'] = 0

    # 7. 地域集中度 (0~3分)
    val = results.get('地域集中度')
    if val is not None:
        total_dims += 1
        if val > 0.50:
            scores['地域集中度'] = 3
        elif val > 0.30:
            scores['地域集中度'] = 2
        elif val > 0.15:
            scores['地域集中度'] = 1
        else:
            scores['地域集中度'] = 0

    # 8. 同姓氏对手占比 (0~2分)
    if col_opp_name in df.columns:
        total_dims += 1
        opp_names_valid = df[col_opp_name].dropna()
        if len(opp_names_valid) > 0:
            surnames = opp_names_valid.astype(str).str[0]
            top_surname_ratio = surnames.value_counts().iloc[0] / len(surnames) if len(surnames) > 0 else 0
            if top_surname_ratio > 0.40:
                scores['同姓氏对手占比'] = 2
            elif top_surname_ratio > 0.25:
                scores['同姓氏对手占比'] = 1
            else:
                scores['同姓氏对手占比'] = 0

    # 9. 对手年龄分布 (0~2分)
    if col_opp_id in df.columns:
        total_dims += 1
        opp_ids_valid = df[col_opp_id].dropna().astype(str)
        birth_years = opp_ids_valid.str[6:10]
        birth_years = pd.to_numeric(birth_years, errors='coerce').dropna()
        if len(birth_years) > 0:
            current_year = pd.Timestamp.now().year
            ages = current_year - birth_years
            young_ratio = ((ages >= 18) & (ages <= 25)).sum() / len(ages)
            if young_ratio > 0.50:
                scores['对手年龄分布'] = 2
            elif young_ratio > 0.30:
                scores['对手年龄分布'] = 1
            else:
                scores['对手年龄分布'] = 0

    # 10. 可疑关键词 (0~3分)
    val = results.get('可疑关键词笔数')
    if val is not None:
        total_dims += 1
        if val >= 10:
            scores['可疑关键词'] = 3
        elif val >= 1:
            scores['可疑关键词'] = 2
        else:
            scores['可疑关键词'] = 0

    # 11. 通用摘要占比 (0~2分)
    if col_memo in df.columns:
        total_dims += 1
        generic_kw = ['货款', '往来款', '采购款', '投资', '服务费', '汇款']
        generic_pattern = '|'.join(generic_kw)
        generic_ratio = df[col_memo].fillna('').astype(str).str.contains(
            generic_pattern, na=False).sum() / len(df)
        if generic_ratio > 0.80:
            scores['通用摘要占比'] = 2
        elif generic_ratio > 0.60:
            scores['通用摘要占比'] = 1
        else:
            scores['通用摘要占比'] = 0

    # 12. 共享设备(IP/MAC) (0~3分)
    total_dims += 1
    shared_device_score = 0
    if col_ip in df.columns:
        ip_counts = df.groupby(col_ip)[col_self_account].nunique()
        max_shared = ip_counts.max() if len(ip_counts) > 0 else 0
        if max_shared >= 3:
            shared_device_score = max(shared_device_score, 3)
        elif max_shared >= 2:
            shared_device_score = max(shared_device_score, 2)
    if col_mac in df.columns:
        mac_counts = df.groupby(col_mac)[col_self_account].nunique()
        max_shared_mac = mac_counts.max() if len(mac_counts) > 0 else 0
        if max_shared_mac >= 3:
            shared_device_score = max(shared_device_score, 3)
        elif max_shared_mac >= 2:
            shared_device_score = max(shared_device_score, 2)
    scores['共享设备'] = shared_device_score

    # 13. 休眠-激活模式 (0~3分) — 简化版检测
    total_dims += 1
    daily_counts = df.groupby(df[col_time].dt.date).size()
    date_range = pd.date_range(daily_counts.index.min(), daily_counts.index.max(), freq='D')
    daily_full = pd.Series(0, index=date_range.date)
    for d, c in daily_counts.items():
        daily_full[d] = c

    dormant_count = 0
    streak = 0
    for count in daily_full.values:
        if count == 0:
            streak += 1
        else:
            if streak >= 30:
                dormant_count += 1
            streak = 0

    if dormant_count >= 3:
        scores['休眠-激活模式'] = 3
    elif dormant_count >= 1:
        scores['休眠-激活模式'] = 2
    else:
        scores['休眠-激活模式'] = 0

    # 汇总
    raw_total = sum(scores.values())
    # 等比例调整（缺失维度补偿）
    adjusted_total = raw_total / total_dims * 13 if total_dims > 0 else 0

    if adjusted_total >= 25:
        risk_level = '一级（极高风险）'
    elif adjusted_total >= 18:
        risk_level = '二级（高风险）'
    elif adjusted_total >= 10:
        risk_level = '三级（中风险）'
    else:
        risk_level = '四级（低风险）'

    print(f"\n=== 多维评分结果 ===")
    for dim, score in scores.items():
        print(f"  {dim}: {score}分")
    print(f"\n原始总分: {raw_total} (参与维度: {total_dims}/13)")
    print(f"调整总分: {adjusted_total:.1f}")
    print(f"风险等级: {risk_level}")

    return {
        'scores': scores,
        'raw_total': raw_total,
        'total_dims': total_dims,
        'adjusted_total': adjusted_total,
        'risk_level': risk_level,
    }

# scoring = multi_dimension_scoring(
#     df, results,
#     COL['opp_name'], COL['opp_id'], COL['time'],
#     COL['amount'], COL['memo'], COL['ip'], COL['mac'],
#     COL['balance'], COL['self_account'], COL['direction'],
#     DIR_IN, DIR_OUT
# )
```

---

## 17. 反冻结试探模式检测 `[来源：技战法8"比判分锁"]`

```python
def detect_anti_freeze_probe(df, col_time, col_amount, col_direction,
                               col_opp_account, col_opp_name, dir_in,
                               small_max=20, large_min=10000, window_hours=24):
    """
    检测反冻结试探模式：小额转入（≤small_max）后window_hours内
    跟随同一对手或同一团伙的大额转入（≥large_min）。
    """
    df_in = df[df[col_direction] == dir_in].sort_values(col_time).copy()
    small_txns = df_in[df_in[col_amount] <= small_max]
    large_txns = df_in[df_in[col_amount] >= large_min]

    pairs = []
    for _, small in small_txns.iterrows():
        small_time = small[col_time]
        small_opp = small[col_opp_account]
        window_end = small_time + pd.Timedelta(hours=window_hours)

        # 查找窗口内同一对手的大额交易
        same_opp_large = large_txns[
            (large_txns[col_time] > small_time) &
            (large_txns[col_time] <= window_end) &
            (large_txns[col_opp_account] == small_opp)
        ]

        if len(same_opp_large) > 0:
            first_large = same_opp_large.iloc[0]
            pairs.append({
                'probe_time': small_time,
                'probe_amount': small[col_amount],
                'probe_opp': small_opp,
                'probe_opp_name': small.get(col_opp_name, '未知'),
                'large_time': first_large[col_time],
                'large_amount': first_large[col_amount],
                'interval_hours': (first_large[col_time] - small_time).total_seconds() / 3600,
            })

    # 也检查不同对手但紧邻的模式
    for _, small in small_txns.iterrows():
        small_time = small[col_time]
        # 查找紧随的大额（5分钟内）
        immediate_large = large_txns[
            (large_txns[col_time] > small_time) &
            (large_txns[col_time] <= small_time + pd.Timedelta(minutes=5))
        ]
        if len(immediate_large) > 0 and small[col_opp_account] != immediate_large.iloc[0][col_opp_account]:
            first_large = immediate_large.iloc[0]
            already_found = any(
                p['probe_time'] == small_time and p['probe_opp'] == small[col_opp_account]
                for p in pairs
            )
            if not already_found:
                pairs.append({
                    'probe_time': small_time,
                    'probe_amount': small[col_amount],
                    'probe_opp': small[col_opp_account],
                    'probe_opp_name': small.get(col_opp_name, '未知'),
                    'large_time': first_large[col_time],
                    'large_amount': first_large[col_amount],
                    'interval_hours': (first_large[col_time] - small_time).total_seconds() / 3600,
                })

    print(f"\n=== 反冻结试探检测 ===")
    print(f"发现 {len(pairs)} 个试探-跟随配对")

    # 提取涉及的试探方（标记为团伙账户）
    probe_accounts = set()
    for p in pairs:
        probe_accounts.add(p['probe_opp'])
        print(f"  试探: {p['probe_time']} {p['probe_amount']:.0f}元 ← {p['probe_opp_name']}")
        print(f"  大额: {p['large_time']} {p['large_amount']:,.0f}元, 间隔{p['interval_hours']:.1f}h")

    if probe_accounts:
        print(f"\n涉及的试探方账户（标记为团伙账户）: {len(probe_accounts)}个")
        for a in probe_accounts:
            print(f"  {a}")

    return pairs, probe_accounts

# freeze_pairs, freeze_accounts = detect_anti_freeze_probe(
#     df, COL['time'], COL['amount'], COL['direction'],
#     COL['opp_account'], COL['opp_name'], DIR_IN
# )
```

---

## 18. 伞状分散结构检测 `[来源：技战法7"比穿联汇"]`

```python
def detect_umbrella_scatter(df, col_self_account, col_opp_account, col_amount,
                             col_direction, col_time, dir_out,
                             min_targets=5, window_hours=24, cv_threshold=0.3):
    """
    检测伞状分散转出：一个账户在短时间内向多个对手分散转出。
    min_targets: 最少转出目标数
    window_hours: 时间窗口（小时）
    cv_threshold: 金额变异系数阈值（标准差/均值），越小越均匀
    """
    df_out = df[df[col_direction] == dir_out].sort_values(col_time)
    accounts = df_out[col_self_account].unique()

    scatter_events = []
    for acct in accounts:
        acct_out = df_out[df_out[col_self_account] == acct]
        if len(acct_out) < min_targets:
            continue

        # 滑动窗口检测
        for i, (_, row) in enumerate(acct_out.iterrows()):
            window_start = row[col_time]
            window_end = window_start + pd.Timedelta(hours=window_hours)

            window_txns = acct_out[
                (acct_out[col_time] >= window_start) &
                (acct_out[col_time] <= window_end)
            ]
            unique_targets = window_txns[col_opp_account].nunique()

            if unique_targets >= min_targets:
                amounts = window_txns[col_amount].values
                cv = amounts.std() / amounts.mean() if amounts.mean() > 0 else float('inf')

                scatter_events.append({
                    'account': acct,
                    'date': window_start.date(),
                    'target_count': unique_targets,
                    'txn_count': len(window_txns),
                    'total_amount': amounts.sum(),
                    'avg_amount': amounts.mean(),
                    'cv': cv,
                    'is_uniform': cv < cv_threshold,
                    'targets': window_txns[col_opp_account].unique().tolist()[:10],
                })
                break  # 避免同一窗口重复计数

    # 去重（同一账户同一天只保留一条）
    seen = set()
    unique_events = []
    for e in scatter_events:
        key = (e['account'], e['date'])
        if key not in seen:
            seen.add(key)
            unique_events.append(e)

    print(f"\n=== 伞状分散结构检测 ===")
    if not unique_events:
        print(f"未发现{window_hours}h内向≥{min_targets}个对手分散转出的模式")
    else:
        print(f"发现 {len(unique_events)} 次伞状分散事件:")
        for e in unique_events:
            uniform_mark = '[金额均匀]' if e['is_uniform'] else '[金额不均]'
            print(f"  {e['account']} @ {e['date']}: → {e['target_count']}个对手, "
                  f"{e['txn_count']}笔, 合计{e['total_amount']:,.0f}元, "
                  f"均笔{e['avg_amount']:,.0f}元, CV={e['cv']:.2f} {uniform_mark}")

    return unique_events

# scatter = detect_umbrella_scatter(
#     df, COL['self_account'], COL['opp_account'],
#     COL['amount'], COL['direction'], COL['time'], DIR_OUT
# )
```

---

## 19. 日终余额趋零分析 `[来源：技战法1"四步"、技战法3"三取四辨"]`

```python
def analyze_daily_ending_balance(df, col_time, col_balance, zero_threshold=1000):
    """
    分析日终余额趋零特征。
    过渡账户的典型特征：日终余额接近零。
    """
    if col_balance not in df.columns:
        print("无余额字段，跳过日终余额分析")
        return None

    df_sorted = df.sort_values(col_time)
    df_sorted['_balance_num'] = pd.to_numeric(df_sorted[col_balance], errors='coerce')
    daily_last = df_sorted.groupby(df_sorted[col_time].dt.date)['_balance_num'].last().dropna()

    if len(daily_last) == 0:
        print("无有效余额数据")
        return None

    zero_days = (daily_last < zero_threshold).sum()
    total_days = len(daily_last)
    zero_ratio = zero_days / total_days

    print(f"\n=== 日终余额分析 ===")
    print(f"有交易的总天数: {total_days}")
    print(f"日终余额<{zero_threshold}元的天数: {zero_days} ({zero_ratio*100:.1f}%)")
    print(f"日终余额统计: 均值={daily_last.mean():,.2f}, "
          f"中位数={daily_last.median():,.2f}, 最大={daily_last.max():,.2f}")

    if zero_ratio > 0.70:
        print("→ 高度异常: 日终余额长期趋零，典型过渡账户特征")
    elif zero_ratio > 0.50:
        print("→ 异常: 日终余额频繁趋零")
    elif zero_ratio > 0.30:
        print("→ 需关注: 日终余额较多趋零")
    else:
        print("→ 正常")

    # 日平均余额（用于识别财务账户 — 技战法3标准：<5万为过渡，>10万为财务）
    avg_daily_balance = daily_last.mean()
    print(f"\n日均余额: {avg_daily_balance:,.2f}元")
    if avg_daily_balance < 50000:
        print("→ 日均余额<5万: 符合过渡账户特征")
    elif avg_daily_balance > 100000:
        print("→ 日均余额>10万: 可能为财务/沉淀账户")

    return {
        'zero_days': zero_days,
        'total_days': total_days,
        'zero_ratio': zero_ratio,
        'avg_daily_balance': avg_daily_balance,
    }

# balance_analysis = analyze_daily_ending_balance(df, COL['time'], COL['balance'])
```

---

## 20. 汇率匹配检测（"智汇"模型） `[来源：技战法9"智汇猎手"]`

```python
def detect_exchange_rate_matching(df, col_time, col_amount, col_opp_account, col_opp_name,
                                   tolerance=100):
    """
    检测交易金额是否与外币汇率匹配。
    对金额个位非零或含小数的交易，按当日汇率折算常见外币，
    若结果趋近整百/整千/整万（偏差≤tolerance），标注币种标签。
    """
    # 常见外币汇率示例（实际应接入央行汇率API或导入汇率表）
    # 格式：{币种: 大致汇率范围}，以人民币/1外币为单位
    sample_rates = {
        'USD': 7.2,   # 美元
        'EUR': 7.8,   # 欧元
        'GBP': 9.1,   # 英镑
        'HKD': 0.92,  # 港币
        'JPY': 0.048, # 日元
        'AUD': 4.7,   # 澳元
        'TWD': 0.22,  # 台币
        'KRW': 0.0053,# 韩元
        'SGD': 5.3,   # 新加坡元
        'CAD': 5.2,   # 加元
    }

    df_work = df.copy()
    amounts = df_work[col_amount].values

    # 筛选金额个位非零或含小数的交易
    has_decimal = (amounts != amounts.astype(int)) | (amounts % 10 != 0)
    df_candidates = df_work[has_decimal].copy()

    if len(df_candidates) == 0:
        print("无符合汇率特征的交易（金额均为整十数）")
        return pd.DataFrame()

    print(f"候选交易（金额个位非零或含小数）: {len(df_candidates)}笔")

    # 逐币种检测
    tags = []
    for _, row in df_candidates.iterrows():
        amount = row[col_amount]
        for currency, rate in sample_rates.items():
            foreign_amount = amount / rate
            # 检查是否趋近整百、整千、整万
            for unit in [100, 1000, 10000]:
                remainder = foreign_amount % unit
                if remainder <= tolerance or (unit - remainder) <= tolerance:
                    tags.append({
                        'time': row[col_time],
                        'amount': amount,
                        'opp_account': row.get(col_opp_account),
                        'opp_name': row.get(col_opp_name),
                        'currency': currency,
                        'foreign_amount': round(foreign_amount, 2),
                        'nearest_round': round(foreign_amount / unit) * unit,
                        'deviation': min(remainder, unit - remainder),
                    })
                    break  # 每个币种只取最近的匹配

    tag_df = pd.DataFrame(tags)
    if len(tag_df) == 0:
        print("未发现汇率匹配交易")
        return tag_df

    # 按账户统计各币种标签
    print(f"\n=== 汇率匹配结果: {len(tag_df)}条标签 ===")
    currency_summary = tag_df.groupby('currency').agg(
        tag_count=('amount', 'count'),
        opp_count=('opp_account', 'nunique'),
        date_count=('time', lambda x: x.dt.date.nunique()),
    )

    for currency, row in currency_summary.iterrows():
        suspicious = (row['tag_count'] > 10 and row['opp_count'] > 5 and row['date_count'] > 5)
        mark = ' *** 高度可疑 ***' if suspicious else ''
        print(f"  {currency}: {row['tag_count']}条标签, "
              f"涉及{row['opp_count']}个对手, {row['date_count']}个日期{mark}")

    return tag_df

# rate_tags = detect_exchange_rate_matching(
#     df, COL['time'], COL['amount'], COL['opp_account'], COL['opp_name']
# )
```

---

## 21. 一次性交易对手占比 `[来源：技战法9"智汇猎手"]`

```python
def analyze_one_time_counterparties(df, col_opp_account, col_opp_name, col_amount):
    """
    统计仅交易1-2次的对手占总对手数的比例。
    高占比提示典型钱庄特征（大量一次性客户）。
    """
    opp_freq = df.dropna(subset=[col_opp_account]).groupby(col_opp_account).agg(
        txn_count=(col_amount, 'count'),
        total_amount=(col_amount, 'sum'),
        name=(col_opp_name, 'first')
    )

    total_opp = len(opp_freq)
    one_time = len(opp_freq[opp_freq['txn_count'] <= 2])
    ratio = one_time / total_opp if total_opp > 0 else 0

    print(f"\n=== 一次性交易对手分析 ===")
    print(f"总对手数: {total_opp}")
    print(f"交易1-2次的对手: {one_time}个 ({ratio*100:.1f}%)")

    if ratio > 0.70:
        print("→ 高度异常: 典型钱庄特征，大量一次性客户")
    elif ratio > 0.50:
        print("→ 异常: 一次性对手占比偏高")
    else:
        print("→ 正常")

    # 按交易频次分布
    freq_dist = opp_freq['txn_count'].value_counts().sort_index()
    print(f"\n对手交易频次分布:")
    for freq, count in freq_dist.head(10).items():
        print(f"  交易{freq}次: {count}个对手")

    return {'one_time_count': one_time, 'total_opp': total_opp, 'ratio': ratio}

# one_time = analyze_one_time_counterparties(df, COL['opp_account'], COL['opp_name'], COL['amount'])
```

---

## 22. 非工作时间交易分析 `[来源：技战法9"智汇猎手"]`

```python
def analyze_night_transactions(df, col_time, col_amount, col_direction, dir_in, dir_out,
                                 night_start=22, night_end=8):
    """
    统计非工作时间（夜间）交易占比。
    夜间活跃为网络赌博典型特征，也可能提示跨时区换汇。
    """
    hours = df[col_time].dt.hour
    night_mask = (hours >= night_start) | (hours < night_end)

    total = len(df)
    night_count = night_mask.sum()
    night_ratio = night_count / total if total > 0 else 0
    night_amount = df.loc[night_mask, col_amount].sum()

    print(f"\n=== 非工作时间交易分析 ({night_start}:00-{night_end}:00) ===")
    print(f"夜间交易: {night_count}笔 ({night_ratio*100:.1f}%), 金额{night_amount:,.2f}元")

    if night_ratio > 0.30:
        print("→ 高度异常: 大量夜间交易，疑似涉赌或跨时区换汇")
    elif night_ratio > 0.20:
        print("→ 异常: 夜间交易占比偏高")
    elif night_ratio > 0.10:
        print("→ 需关注: 存在一定比例的夜间交易")
    else:
        print("→ 正常")

    # 按小时分布
    hourly = df.groupby(hours).agg(
        count=(col_amount, 'count'),
        total=(col_amount, 'sum')
    )
    print(f"\n每小时交易分布:")
    for h, row in hourly.iterrows():
        bar = '█' * int(row['count'] / hourly['count'].max() * 30)
        print(f"  {h:02d}时: {row['count']:>5}笔 {row['total']:>15,.0f}元 {bar}")

    # 按方向分别统计
    for label, dir_val in [('收入', dir_in), ('支出', dir_out)]:
        dir_df = df[df[col_direction] == dir_val]
        dir_night = dir_df[dir_df[col_time].dt.hour.isin(
            list(range(night_start, 24)) + list(range(0, night_end))
        )]
        dir_ratio = len(dir_night) / len(dir_df) * 100 if len(dir_df) > 0 else 0
        print(f"  {label}端夜间占比: {len(dir_night)}/{len(dir_df)} ({dir_ratio:.1f}%)")

    return {'night_count': night_count, 'night_ratio': night_ratio, 'night_amount': night_amount}

# night = analyze_night_transactions(df, COL['time'], COL['amount'], COL['direction'], DIR_IN, DIR_OUT)
```

---

## 23. IP网段分析 `[来源：技战法10"热力图"]`

```python
def analyze_ip_segments(df, col_ip, col_self_account, col_time, col_direction, dir_out):
    """
    基于IP网段进行团伙关联分析。
    区分企业专线（固定IP）和家庭宽带（动态IP），按网段聚合。
    """
    if col_ip not in df.columns:
        print("无IP地址字段，跳过IP网段分析")
        return None

    df_out = df[df[col_direction] == dir_out].dropna(subset=[col_ip]).copy()
    if len(df_out) == 0:
        print("出账方向无IP数据")
        return None

    # 提取IP前三段作为网段
    df_out['ip_segment'] = df_out[col_ip].astype(str).str.rsplit('.', n=1).str[0]

    # 按网段聚合
    segment_groups = df_out.groupby('ip_segment').agg(
        account_count=(col_self_account, 'nunique'),
        accounts=(col_self_account, lambda x: list(x.unique())),
        txn_count=(col_ip, 'count'),
        ip_count=(col_ip, 'nunique'),
        date_count=(col_time, lambda x: x.dt.date.nunique()),
    ).sort_values('account_count', ascending=False)

    # 筛选关联≥2个账户的网段
    shared_segments = segment_groups[segment_groups['account_count'] >= 2]

    print(f"\n=== IP网段分析 ===")
    print(f"出账记录中有IP的: {len(df_out)}笔")
    print(f"涉及IP网段: {len(segment_groups)}个")
    print(f"关联≥2个账户的网段: {len(shared_segments)}个")

    for segment, row in shared_segments.head(15).iterrows():
        # 判断IP类型：同一网段在多天出现相同IP → 可能为固定IP(企业专线)
        ip_type = '可能为企业专线' if row['date_count'] > 5 and row['ip_count'] <= 3 else '可能为家庭宽带'
        print(f"\n  网段 {segment}.*: {row['account_count']}个账户, "
              f"{row['ip_count']}个IP, 跨{row['date_count']}天 ({ip_type})")
        for acct in row['accounts'][:10]:
            print(f"    - {acct}")

    return shared_segments

# ip_segments = analyze_ip_segments(df, COL['ip'], COL['self_account'], COL['time'], COL['direction'], DIR_OUT)
```

---

## 24. 社群算法团伙划分（Louvain） `[来源：技战法11"剥洋葱"]`

```python
def community_detection_louvain(df, col_self_account, col_opp_account, col_amount,
                                  col_direction, dir_out):
    """
    使用Louvain社群检测算法对账户网络进行团伙划分。
    需要安装: pip install python-louvain networkx
    """
    try:
        import networkx as nx
        import community as community_louvain
    except ImportError:
        print("需安装 networkx 和 python-louvain: pip install networkx python-louvain")
        return None

    # 构建有向加权图
    df_out = df[df[col_direction] == dir_out].dropna(subset=[col_opp_account])
    edge_data = df_out.groupby([col_self_account, col_opp_account]).agg(
        weight=(col_amount, 'sum'),
        count=(col_amount, 'count')
    ).reset_index()

    G = nx.from_pandas_edgelist(
        edge_data,
        source=col_self_account,
        target=col_opp_account,
        edge_attr=['weight', 'count'],
        create_using=nx.DiGraph()
    )

    print(f"\n=== 资金网络图 ===")
    print(f"节点数: {G.number_of_nodes()}, 边数: {G.number_of_edges()}")

    # 转为无向图用于社群检测
    G_undirected = G.to_undirected()

    # Louvain社群检测
    partition = community_louvain.best_partition(G_undirected, weight='weight')

    # 按社群分组
    communities = {}
    for node, comm_id in partition.items():
        communities.setdefault(comm_id, []).append(node)

    # 按社群大小排序
    sorted_comms = sorted(communities.items(), key=lambda x: -len(x[1]))

    print(f"识别出 {len(sorted_comms)} 个社群:")
    for comm_id, members in sorted_comms[:10]:
        total_weight = sum(
            G[u][v].get('weight', 0)
            for u, v in G.edges()
            if u in members and v in members
        )
        print(f"  社群{comm_id}: {len(members)}个账户, 内部资金流{total_weight:,.2f}元")

    # 计算中介中心度（识别核心团伙）
    betweenness = nx.betweenness_centrality(G, weight='weight')
    top_central = sorted(betweenness.items(), key=lambda x: -x[1])[:10]

    print(f"\n中介中心度TOP10（核心节点）:")
    for node, bc in top_central:
        comm = partition.get(node, '未知')
        print(f"  {node}: 中心度={bc:.4f}, 所属社群={comm}")

    return {
        'partition': partition,
        'communities': communities,
        'betweenness': betweenness,
        'graph': G,
    }

# community_result = community_detection_louvain(
#     df, COL['self_account'], COL['opp_account'],
#     COL['amount'], COL['direction'], DIR_OUT
# )
```

---

## 25. 工具账户 vs 核心账户区分 `[来源：技战法11"剥洋葱"、技战法13"四比对四锁定"]`

```python
def classify_tool_vs_core_accounts(df, col_self_account, col_amount, col_direction,
                                     col_memo, col_balance, dir_in, dir_out):
    """
    区分工具账户和核心账户。
    工具账户：对外开展业务，快进快出无消费；
    核心账户：内部调拨，有生活消费特征。
    """
    accounts = df[col_self_account].unique()
    classifications = []

    # 生活消费关键词
    life_keywords = ['餐饮', '超市', '便利店', '水电', '燃气', '物业', '话费',
                     '医院', '药店', '加油', '停车', '外卖', '美团', '饿了么',
                     '淘宝', '京东', '拼多多', '滴滴', '公交', '地铁']
    # 理财投资关键词
    finance_keywords = ['理财', '基金', '保险', '定期', '活期', '利息', '分红',
                        '股票', '证券']

    for acct in accounts:
        acct_df = df[df[col_self_account] == acct]
        acct_in = acct_df[acct_df[col_direction] == dir_in]
        acct_out = acct_df[acct_df[col_direction] == dir_out]

        in_amount = acct_in[col_amount].sum()
        out_amount = acct_out[col_amount].sum()
        total = in_amount + out_amount

        # 收支比值
        ratio = in_amount / total if total > 0 else 0.5

        # 日终余额趋零检测
        zero_balance = False
        if col_balance in df.columns:
            balances = pd.to_numeric(acct_df[col_balance], errors='coerce').dropna()
            if len(balances) > 0:
                daily_end = acct_df.sort_values(COL['time']).groupby(
                    acct_df[COL['time']].dt.date
                )[col_balance].last()
                daily_end = pd.to_numeric(daily_end, errors='coerce').dropna()
                zero_ratio = (daily_end < 500).sum() / len(daily_end) if len(daily_end) > 0 else 0
                zero_balance = zero_ratio > 0.5

        # 生活消费检测
        has_life_expense = False
        has_finance = False
        if col_memo in df.columns:
            memos = acct_df[col_memo].fillna('').astype(str)
            life_pattern = '|'.join(life_keywords)
            has_life_expense = memos.str.contains(life_pattern, na=False).any()
            finance_pattern = '|'.join(finance_keywords)
            has_finance = memos.str.contains(finance_pattern, na=False).any()

        # 分类判定
        if zero_balance and not has_life_expense and 0.3 <= ratio <= 0.7:
            acct_type = '工具账户'
        elif has_life_expense or has_finance:
            acct_type = '核心账户（个人）'
        elif ratio > 0.8:
            acct_type = '接收账户'
        elif ratio < 0.2:
            acct_type = '发放/取现账户'
        else:
            acct_type = '待定'

        classifications.append({
            'account': acct,
            'type': acct_type,
            'in_amount': in_amount,
            'out_amount': out_amount,
            'ratio': ratio,
            'zero_balance': zero_balance,
            'has_life_expense': has_life_expense,
            'has_finance': has_finance,
        })

    result_df = pd.DataFrame(classifications)
    print(f"\n=== 账户功能分类 ===")
    for _, row in result_df.iterrows():
        print(f"  {row['account']}: {row['type']} (比值={row['ratio']:.2f}, "
              f"余额趋零={row['zero_balance']}, 生活消费={row['has_life_expense']})")

    # 汇总
    type_summary = result_df['type'].value_counts()
    print(f"\n分类汇总:")
    for t, c in type_summary.items():
        print(f"  {t}: {c}个")

    return result_df

# acct_classes = classify_tool_vs_core_accounts(
#     df, COL['self_account'], COL['amount'], COL['direction'],
#     COL['memo'], COL['balance'], DIR_IN, DIR_OUT
# )
```

---

## 26. 交易亲密度团伙关联 `[来源：技战法9"智汇猎手"]`

```python
def detect_intimacy_groups(df, col_self_account, col_opp_account, col_amount,
                             col_direction, dir_in, dir_out,
                             common_opp_threshold=8, mutual_txn_threshold=3,
                             large_amount_threshold=100000):
    """
    通过交易亲密度识别团伙关联：
    1. 共同交易对手≥8次
    2. 首次大额(≥10万)交易后彼此交易≥3次
    """
    accounts = df[col_self_account].unique()
    if len(accounts) < 2:
        print("需要至少2个主体账户才能计算交易亲密度")
        return []

    # 每个账户的交易对手集合
    account_opps = {}
    for acct in accounts:
        acct_df = df[df[col_self_account] == acct]
        opps = acct_df[col_opp_account].dropna().value_counts()
        account_opps[acct] = opps

    pairs = []
    acct_list = list(accounts)
    for i in range(len(acct_list)):
        for j in range(i + 1, len(acct_list)):
            a, b = acct_list[i], acct_list[j]
            opps_a = set(account_opps.get(a, pd.Series()).index)
            opps_b = set(account_opps.get(b, pd.Series()).index)

            # 条件1: 共同对手数
            common_opps = opps_a & opps_b
            common_count = len(common_opps)

            # 条件2: 相互交易
            mutual_txns = df[
                ((df[col_self_account] == a) & (df[col_opp_account] == b)) |
                ((df[col_self_account] == b) & (df[col_opp_account] == a))
            ]
            mutual_count = len(mutual_txns)

            # 检查大额起始条件
            has_large = (mutual_txns[col_amount] >= large_amount_threshold).any() if len(mutual_txns) > 0 else False

            is_related = False
            reason = []
            if common_count >= common_opp_threshold:
                is_related = True
                reason.append(f'共同对手{common_count}个(≥{common_opp_threshold})')
            if has_large and mutual_count >= mutual_txn_threshold:
                is_related = True
                reason.append(f'大额后互转{mutual_count}次(≥{mutual_txn_threshold})')

            if is_related:
                pairs.append({
                    'account_a': a,
                    'account_b': b,
                    'common_opps': common_count,
                    'mutual_txns': mutual_count,
                    'has_large': has_large,
                    'reason': '; '.join(reason),
                })

    print(f"\n=== 交易亲密度团伙关联 ===")
    if not pairs:
        print("未发现满足亲密度条件的账户对")
    else:
        print(f"发现 {len(pairs)} 对关联账户:")
        for p in pairs:
            print(f"  {p['account_a']} <-> {p['account_b']}: {p['reason']}")

    return pairs

# intimacy_pairs = detect_intimacy_groups(
#     df, COL['self_account'], COL['opp_account'],
#     COL['amount'], COL['direction'], DIR_IN, DIR_OUT
# )
```

---

## 27. 交错活跃-沉睡检测 `[来源：技战法9"智汇猎手"]`

```python
def detect_alternating_activity(df, col_self_account, col_time, col_id='交易证件号'):
    """
    检测同一主体名下多个账户交错活跃-沉睡的规避监管模式。
    """
    if col_id not in df.columns:
        print("无证件号字段，跳过交错活跃检测")
        return None

    # 按证件号分组，找出同一人名下的多个账户
    person_accounts = df.groupby(col_id)[col_self_account].unique()
    multi_accounts = person_accounts[person_accounts.apply(len) >= 2]

    if len(multi_accounts) == 0:
        print("未发现同一证件号名下有多个账户")
        return None

    print(f"\n=== 交错活跃-沉睡检测 ===")
    print(f"同一证件号下多账户的主体: {len(multi_accounts)}个")

    alternating_cases = []
    for person_id, accounts in multi_accounts.items():
        # 按月统计各账户交易量
        monthly_activity = {}
        for acct in accounts:
            acct_df = df[df[col_self_account] == acct]
            monthly = acct_df.groupby(acct_df[col_time].dt.to_period('M')).size()
            monthly_activity[acct] = monthly

        # 检测交替模式
        all_months = sorted(set().union(*[set(m.index) for m in monthly_activity.values()]))
        for m in all_months:
            active = [a for a, act in monthly_activity.items() if m in act.index and act[m] > 0]
            inactive = [a for a in accounts if a not in active]

            if len(active) >= 1 and len(inactive) >= 1:
                # 检查下个月是否反转
                next_m = m + 1
                if next_m in [mo for mo in all_months]:
                    next_active = [a for a, act in monthly_activity.items()
                                   if next_m in act.index and act[next_m] > 0]
                    # 之前活跃的变沉睡，之前沉睡的变活跃 → 交替
                    switched = set(active) & set([a for a in accounts if a not in next_active])
                    newly_active = set(inactive) & set(next_active)
                    if switched and newly_active:
                        alternating_cases.append({
                            'person_id': person_id,
                            'month': str(m),
                            'active_then': list(switched),
                            'active_now': list(newly_active),
                        })

    if alternating_cases:
        print(f"发现 {len(alternating_cases)} 次交替活跃模式:")
        for case in alternating_cases[:10]:
            print(f"  {case['person_id']} @ {case['month']}: "
                  f"{case['active_then']} → 沉睡, {case['active_now']} → 激活")
    else:
        print("未发现明显的交替活跃-沉睡模式")

    return alternating_cases

# alternating = detect_alternating_activity(df, COL['self_account'], COL['time'])
```

---

## 28. 敏感摘要关键词检测 `[来源：技战法13"四比对四锁定"、技战法12"平行拓线"]`

```python
def detect_sensitive_keywords(df, col_memo, col_amount, col_direction, dir_in, dir_out):
    """
    检测交易摘要中的敏感关键词，辅助判断犯罪类型。
    """
    if col_memo not in df.columns:
        print("无摘要字段，跳过敏感关键词检测")
        return None

    keyword_groups = {
        '换汇类': ['换汇', '换钱', '换币', '兑换', '换美金', '换美元', '外汇', '汇率'],
        '走私类': ['冻品', '海鲜', '鸡脚', '猪副', '牛副', '牛肉', '进口', '报关', '清关'],
        '赌博类': ['赌', '博彩', '彩金', '下注', '投注', '返水', '代充'],
        '虚拟资产类': ['比特币', 'BTC', 'USDT', '虚拟币', '数字货币', '点卡', '游戏币'],
        '贸易伪装类': ['货款', '往来款', '采购款', '投资款', '服务费', '咨询费', '工程款'],
        '支付类': ['支付', '结算', '手续费', '佣金', '代付', '代收'],
    }

    memos = df[col_memo].fillna('').astype(str)
    results = {}

    print(f"\n=== 敏感摘要关键词检测 ===")
    for group_name, keywords in keyword_groups.items():
        pattern = '|'.join(keywords)
        mask = memos.str.contains(pattern, na=False, case=False)
        count = mask.sum()
        amount = df.loc[mask, col_amount].sum()

        if count > 0:
            results[group_name] = {
                'count': count,
                'amount': amount,
                'ratio': count / len(df) * 100,
                'matched_keywords': []
            }
            print(f"\n  {group_name}: {count}笔 ({count/len(df)*100:.1f}%), 金额{amount:,.2f}元")
            for kw in keywords:
                kw_count = memos.str.contains(kw, na=False).sum()
                if kw_count > 0:
                    results[group_name]['matched_keywords'].append((kw, kw_count))
                    print(f"    '{kw}': {kw_count}笔")

    if not results:
        print("未发现敏感关键词")

    return results

# sensitive = detect_sensitive_keywords(df, COL['memo'], COL['amount'], COL['direction'], DIR_IN, DIR_OUT)
```

---

## 29. 钱庄利润率计算 `[来源：技战法7"比穿联汇"、技战法14"五维五步"]`

```python
def calculate_profit_rate(in_amount, out_amount):
    """
    计算钱庄利润率 = (总进账 - 总出账) / 总进账。
    典型钱庄抽佣比例: 0.5%~3%
    """
    if in_amount <= 0:
        print("无进账数据，无法计算利润率")
        return None

    profit = in_amount - out_amount
    profit_rate = profit / in_amount

    print(f"\n=== 利润率分析 ===")
    print(f"总进账: {in_amount:,.2f}元")
    print(f"总出账: {out_amount:,.2f}元")
    print(f"差额(利润): {profit:,.2f}元")
    print(f"利润率: {profit_rate*100:.4f}%")

    if 0.005 <= profit_rate <= 0.03:
        print(f"→ 利润率落入典型钱庄抽佣区间(0.5%~3%)，佐证钱庄性质")
        print(f"→ 按此估算团伙获利约 {profit:,.2f} 元")
    elif profit_rate < 0.005 and profit_rate >= 0:
        print(f"→ 利润率极低(<0.5%)，可能为纯过渡账户或利润在其他环节实现")
    elif profit_rate > 0.03 and profit_rate < 0.10:
        print(f"→ 利润率偏高(3%~10%)，可能包含其他收入来源")
    elif profit_rate >= 0.10:
        print(f"→ 利润率>10%，不符合典型钱庄特征，需进一步判断业务性质")
    else:
        print(f"→ 负利润率，出账大于进账，可能为资金来源不完整")

    return {
        'profit': profit,
        'profit_rate': profit_rate,
        'in_typical_range': 0.005 <= profit_rate <= 0.03,
    }

# profit = calculate_profit_rate(in_amount, out_amount)
```
