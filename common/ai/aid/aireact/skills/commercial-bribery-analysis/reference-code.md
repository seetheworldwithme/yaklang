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
COL = {
    'time': '交易时间',
    'amount': '交易金额',
    'direction': '收付标志',
    'balance': '交易余额',
    'opp_account': '交易对手账卡号',
    'opp_name': '对手户名',
    'opp_id': '对手证件号',
    'opp_bank': '对手开户银行',
    'opp_balance': '对手交易余额',
    'cash_flag': '现金标志',
    'memo': '摘要说明',
    'self_account': '交易卡号',
    'self_name': '交易户名',
    'self_id': '交易证件号',
    'ip': 'IP地址',
    'mac': 'MAC地址',
    'location': '交易发生地',
    'branch': '交易网点名称',
    'success': '交易是否成功',
    'currency': '交易币种',
    'txn_serial': '交易流水号',
    'voucher': '凭证号',
    'teller': '交易柜员号',
}

DIR_IN = '进'
DIR_OUT = '出'

# --- 数据预处理 ---
df = df.copy()
df[COL['time']] = pd.to_datetime(df[COL['time']])
df[COL['amount']] = pd.to_numeric(df[COL['amount']], errors='coerce').abs()

if COL['success'] in df.columns:
    total_before = len(df)
    df = df[df[COL['success']] == 1]
    print(f"过滤失败交易: {total_before - len(df)}笔, 剩余{len(df)}笔")

df_in = df[df[COL['direction']] == DIR_IN]
df_out = df[df[COL['direction']] == DIR_OUT]

print(f"收入: {len(df_in)}笔, 支出: {len(df_out)}笔")
```

### 辅助表关联

```python
person_lookup = {}
if df_person is not None:
    person_col_id = '证照号码' if '证照号码' in df_person.columns else None
    if person_col_id:
        for _, row in df_person.iterrows():
            person_lookup[row[person_col_id]] = row.to_dict()
        print(f"人员信息已加载: {len(person_lookup)}条")

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
        print(f"  {name}({card}): 开户时间={open_date}, 开户行={bank}, 网点={branch}, 类型={acct_type}, 状态={status}")

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

# 商业贿赂场景关注收支比例而非平衡度
in_ratio = in_amount / total_amount * 100 if total_amount > 0 else 0
out_ratio = out_amount / total_amount * 100 if total_amount > 0 else 0
in_out_multiple = in_amount / out_amount if out_amount > 0 else float('inf')

print(f"交易笔数: {total_count}, 总金额: {total_amount:,.2f}")
print(f"收入: {in_count}笔 / {in_amount:,.2f}元 ({in_ratio:.1f}%)")
print(f"支出: {out_count}笔 / {out_amount:,.2f}元 ({out_ratio:.1f}%)")
print(f"收入为支出的 {in_out_multiple:.1f} 倍")
print(f"时间跨度: {date_min.date()} ~ {date_max.date()}, 共{date_span}天")
print(f"日均交易: {total_count / max(date_span, 1):.1f}笔")
```

### 年度趋势（含对手主体数）

```python
df['year'] = df[COL['time']].dt.year

yearly = df.groupby(['year', COL['direction']]).agg(
    count=(COL['amount'], 'count'),
    total=(COL['amount'], 'sum')
).unstack(fill_value=0)

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

### 交易渠道分析

```python
# 通过现金标志、摘要说明或交易网点名称推断交易渠道
# 实际渠道判断逻辑需根据数据中的取值调整

def classify_channel(row):
    """根据交易记录推断交易渠道"""
    cash_flag = str(row.get(COL['cash_flag'], ''))
    memo = str(row.get(COL['memo'], ''))
    branch = str(row.get(COL['branch'], ''))

    if '现' in cash_flag or 'ATM' in cash_flag.upper():
        if 'ATM' in cash_flag.upper() or 'ATM' in memo.upper() or 'ATM' in branch.upper():
            return 'ATM'
        return '柜台现金'
    elif '网银' in cash_flag or '网银' in memo or '网上' in memo:
        return '网上银行'
    elif 'POS' in cash_flag.upper() or 'POS' in memo.upper():
        return 'POS'
    elif '支付宝' in memo or '微信' in memo or '第三方' in cash_flag:
        return '第三方支付'
    elif '手机' in cash_flag or '手机' in memo:
        return '手机银行'
    elif '电话' in cash_flag or '电话' in memo:
        return '电话银行'
    else:
        return '转账/其他'

if COL['cash_flag'] in df.columns or COL['memo'] in df.columns:
    df['channel'] = df.apply(classify_channel, axis=1)
    channel_stats = df.groupby('channel').agg(
        count=(COL['amount'], 'count'),
        total=(COL['amount'], 'sum')
    ).sort_values('total', ascending=False)

    print("\n=== 交易渠道分布 ===")
    for ch, row in channel_stats.iterrows():
        print(f"  {ch}: {row['count']}笔 ({row['count']/total_count*100:.1f}%), "
              f"{row['total']:,.2f}元 ({row['total']/total_amount*100:.1f}%)")

    # 按收入方向的渠道分布（关注行贿资金的流入渠道）
    in_channel = df_in.copy()
    in_channel['channel'] = df.loc[df_in.index, 'channel'] if 'channel' in df.columns else '未知'
    in_ch_stats = in_channel.groupby('channel')[COL['amount']].agg(['count', 'sum']).sort_values('sum', ascending=False)
    print("\n=== 收入端渠道分布 ===")
    print(in_ch_stats)
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
def get_person_info(opp_id, lookup):
    """通过证件号查询人员背景信息"""
    if not opp_id or opp_id == '未知' or not lookup:
        return None
    for key, info in lookup.items():
        if str(opp_id) == str(key) or str(opp_id).startswith(str(key)[:6]):
            return info
    return None

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
                business_id = info.get('客户工商执照号码', '未知')
                print(f"  {i}. {name}: 工作单位={work}, 地址={unit_addr}, 法人={legal}, 工商执照={business_id}")
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
SELF_NAME = df[COL['self_name']].mode()[0] if COL['self_name'] in df.columns else '主体户名'

same_name = df[df[COL['opp_name']] == SELF_NAME]
if len(same_name) > 0:
    same_name_accounts = same_name[COL['opp_account']].unique()
    print(f"\n同名账户: {len(same_name_accounts)}个, 交易{len(same_name)}笔, "
          f"金额{same_name[COL['amount']].sum():,.2f}")
    for acct in same_name_accounts:
        sub = same_name[same_name[COL['opp_account']] == acct]
        print(f"  - {acct}: {len(sub)}笔, {sub[COL['amount']].sum():,.2f}")

in_set = set(df_in[COL['opp_account']].dropna().unique())
out_set = set(df_out[COL['opp_account']].dropna().unique())
bidirectional = in_set & out_set
print(f"\n双向交易对手: {len(bidirectional)}个")
for acct in sorted(bidirectional):
    name = df[df[COL['opp_account']] == acct][COL['opp_name']].iloc[0]
    in_amt = df_in[df_in[COL['opp_account']] == acct][COL['amount']].sum()
    out_amt = df_out[df_out[COL['opp_account']] == acct][COL['amount']].sum()
    print(f"  {name}({acct}): 进{in_amt:,.2f}, 出{out_amt:,.2f}")

SELF_ACCOUNT = df[COL['self_account']].mode()[0] if COL['self_account'] in df.columns else None
if SELF_ACCOUNT:
    self_transfer = df[df[COL['opp_account']] == SELF_ACCOUNT]
    if len(self_transfer) > 0:
        print(f"\n自身转账: {len(self_transfer)}笔, {self_transfer[COL['amount']].sum():,.2f}")
```

### 贿赂伪装摘要检测

```python
bribery_keywords = [
    '咨询费', '技术服务费', '顾问费', '佣金', '提成', '奖金',
    '好处费', '感谢费', '辛苦费', '中介费', '介绍费',
    '茶水费', '车马费', '劳务费', '信息费', '协调费'
]

if COL.get('memo') and COL['memo'] in df.columns:
    pattern = '|'.join(bribery_keywords)
    bribery_memo = df[df[COL['memo']].astype(str).str.contains(pattern, na=False)]

    print(f"\n=== 贿赂伪装摘要检测 ===")
    print(f"命中笔数: {len(bribery_memo)}, 合计金额: {bribery_memo[COL['amount']].sum():,.2f}")

    if len(bribery_memo) > 0:
        # 按关键词分组统计
        for kw in bribery_keywords:
            hits = bribery_memo[bribery_memo[COL['memo']].astype(str).str.contains(kw, na=False)]
            if len(hits) > 0:
                print(f"  '{kw}': {len(hits)}笔, {hits[COL['amount']].sum():,.2f}元")

        # 涉及的对手分布
        bribery_opps = bribery_memo.groupby(COL['opp_name']).agg(
            count=(COL['amount'], 'count'),
            total=(COL['amount'], 'sum')
        ).sort_values('total', ascending=False)
        print(f"\n  涉及 {len(bribery_opps)} 个对手:")
        for name, row in bribery_opps.head(10).iterrows():
            print(f"    {name}: {row['count']}笔, {row['total']:,.2f}元")

        # 区分收入端和支出端
        bribery_in = bribery_memo[bribery_memo[COL['direction']] == DIR_IN]
        bribery_out = bribery_memo[bribery_memo[COL['direction']] == DIR_OUT]
        print(f"\n  收入端: {len(bribery_in)}笔, {bribery_in[COL['amount']].sum():,.2f}元")
        print(f"  支出端: {len(bribery_out)}笔, {bribery_out[COL['amount']].sum():,.2f}元")
```

### 企业-个人关系识别

```python
# 识别对手中的企业账户（对手户名含"公司/有限/集团/厂"等关键词）
enterprise_keywords = ['公司', '有限', '集团', '厂', '企业', '商行', '商贸', '贸易',
                       '科技', '投资', '咨询', '服务', '实业', '股份']
enterprise_pattern = '|'.join(enterprise_keywords)

enterprise_in = df_in[df_in[COL['opp_name']].astype(str).str.contains(enterprise_pattern, na=False)]
enterprise_in_amount = enterprise_in[COL['amount']].sum()
enterprise_in_ratio = enterprise_in_amount / in_amount * 100 if in_amount > 0 else 0

print(f"\n=== 企业转个人分析 ===")
print(f"来自企业账户的收入: {len(enterprise_in)}笔, {enterprise_in_amount:,.2f}元, 占总收入{enterprise_in_ratio:.1f}%")

if len(enterprise_in) > 0:
    ent_top = enterprise_in.groupby(COL['opp_name'])[COL['amount']].agg(['count', 'sum']).sort_values('sum', ascending=False)
    print("\n  TOP企业来源:")
    for name, row in ent_top.head(10).iterrows():
        print(f"    {name}: {row['count']}笔, {row['sum']:,.2f}元")

# 通过人员信息表识别对手的企业背景和业务关联
if df_person is not None and '工作单位' in df_person.columns:
    opp_units = {}
    for _, row in df_person.iterrows():
        name = row.get('客户名称', '')
        unit = row.get('工作单位', '')
        if name and unit and pd.notna(unit):
            opp_units[name] = unit

    print(f"\n=== 对手工作单位分布 ===")
    in_opp_names = df_in[COL['opp_name']].dropna().unique()
    matched_units = {}
    for name in in_opp_names:
        if name in opp_units:
            unit = opp_units[name]
            if unit not in matched_units:
                matched_units[unit] = []
            matched_units[unit].append(name)

    for unit, members in sorted(matched_units.items(), key=lambda x: -len(x[1])):
        amt = df_in[df_in[COL['opp_name']].isin(members)][COL['amount']].sum()
        print(f"  {unit}: {len(members)}人, 合计转入{amt:,.2f}元")
        for m in members:
            m_amt = df_in[df_in[COL['opp_name']] == m][COL['amount']].sum()
            print(f"    - {m}: {m_amt:,.2f}元")
```

---

## 4. 关联人网络分析（阶段C）

### Union-Find 算法

```python
class UnionFind:
    """并查集，用于将共享特征的账户聚类为关联组"""

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

### 基于出账记录的 IP/MAC 聚类（C1）

```python
out_counterparties = set(df_out[COL['opp_account']].dropna().unique())
print(f"出账对手总数: {len(out_counterparties)}")

out_cp_records = df[df[COL['self_account']].isin(out_counterparties)]
out_cp_out = out_cp_records[out_cp_records[COL['direction']] == DIR_OUT].copy()
print(f"出账对手的出方向交易记录数: {len(out_cp_out)}")

has_ip = COL.get('ip') and COL['ip'] in out_cp_out.columns
has_mac = COL.get('mac') and COL['mac'] in out_cp_out.columns
print(f"IP列存在: {has_ip}, MAC列存在: {has_mac}")

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

    print(f"\n识别出 {len(gangs)} 个关联组（>=2人）:")
    for i, (root, members) in enumerate(sorted(gangs.items(), key=lambda x: -len(x[1])), 1):
        print(f"\n--- 关联组{i} ({len(members)}人) ---")
        for m in sorted(members):
            txn = df_out[df_out[COL['opp_account']] == m]
            amt = txn[COL['amount']].sum() if len(txn) > 0 else 0
            cnt = len(txn)
            print(f"  {m}: 与主体交易{cnt}笔, {amt:,.2f}元")
```

### 基于对手证件号前6位的地域聚合（C2）

```python
if COL.get('opp_id') and COL['opp_id'] in df.columns:
    df_id = df.dropna(subset=[COL['opp_account'], COL['opp_id']])[
        [COL['opp_account'], COL['opp_name'], COL['opp_id']]
    ].drop_duplicates(subset=[COL['opp_account']])

    df_id['id_prefix6'] = df_id[COL['opp_id']].astype(str).str[:6]
    df_id = df_id[df_id['id_prefix6'].str.match(r'^\d{6}', na=False)]

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
            txn = df[df[COL['opp_account']] == acct]
            print(f"    {name}({acct}): 与主体交易{len(txn)}笔, {txn[COL['amount']].sum():,.2f}元")

    if len(region_gangs) > 0:
        max_region = region_gangs['accounts'].apply(len).max()
        region_conc = max_region / total_opp_with_id * 100
        print(f"\n最大地域组占比: {max_region}/{total_opp_with_id} = {region_conc:.1f}%")
```

### 企业-个人关联网络构建（C3）

```python
if df_person is not None:
    print("\n=== 企业-个人关联网络 ===")

    # --- 工作单位碰撞 ---
    if '工作单位' in df_person.columns:
        unit_groups = df_person.dropna(subset=['工作单位']).groupby('工作单位')['客户名称'].apply(list)
        shared_units = unit_groups[unit_groups.apply(len) >= 2]
        print(f"\n--- 工作单位碰撞 ---")
        print(f"同单位组（>=2人）: {len(shared_units)}组")
        for unit, members in shared_units.items():
            total_amt = sum(
                df[df[COL['opp_name']] == m][COL['amount']].sum()
                for m in members
            )
            print(f"  {unit}: {len(members)}人, 合计交易{total_amt:,.2f}元")
            for m in members:
                m_amt = df[df[COL['opp_name']] == m][COL['amount']].sum()
                if m_amt > 0:
                    print(f"    - {m}: {m_amt:,.2f}元")

    # --- 法人代表碰撞 ---
    if '法人代表' in df_person.columns:
        legal_groups = df_person.dropna(subset=['法人代表']).groupby('法人代表')['客户名称'].apply(list)
        shared_legal = legal_groups[legal_groups.apply(len) >= 2]
        print(f"\n--- 法人代表碰撞 ---")
        print(f"同法人组（>=2人）: {len(shared_legal)}组")
        for legal, members in shared_legal.items():
            print(f"  法人代表 {legal}: 关联 {len(members)} 个客户 → {members}")

        # 法人代表与交易对手户名交叉比对
        legal_person_map = df_person.dropna(subset=['法人代表']).set_index('客户名称')['法人代表'].to_dict()
        opp_names_set = set(df[COL['opp_name']].dropna().unique())
        legal_in_opps = set(legal_person_map.values()) & opp_names_set
        if legal_in_opps:
            print(f"\n  法人代表同时为交易对手: {legal_in_opps}")

    # --- 单位地址碰撞 ---
    if '单位地址' in df_person.columns:
        addr_groups = df_person.dropna(subset=['单位地址']).groupby('单位地址')['客户名称'].apply(list)
        shared_addr = addr_groups[addr_groups.apply(len) >= 2]
        if len(shared_addr) > 0:
            print(f"\n--- 单位地址碰撞 ---")
            print(f"同地址组（>=2人）: {len(shared_addr)}组")
            for addr, members in shared_addr.items():
                print(f"  {addr}: {members}")

    # --- 工商执照号码关联 ---
    if '客户工商执照号码' in df_person.columns:
        biz_groups = df_person.dropna(subset=['客户工商执照号码']).groupby('客户工商执照号码')['客户名称'].apply(list)
        shared_biz = biz_groups[biz_groups.apply(len) >= 2]
        if len(shared_biz) > 0:
            print(f"\n--- 工商执照号码关联 ---")
            for biz_id, members in shared_biz.items():
                print(f"  工商执照 {biz_id}: {members}")

    # --- 邮箱碰撞 ---
    if '邮箱地址' in df_person.columns:
        email_groups = df_person.dropna(subset=['邮箱地址']).groupby('邮箱地址')['客户名称'].apply(list)
        shared_email = email_groups[email_groups.apply(len) >= 2]
        if len(shared_email) > 0:
            print(f"\n--- 邮箱碰撞 ---")
            for email, members in shared_email.items():
                print(f"  {email}: {members}")
```

### 代办人碰撞（C4）

```python
if df_person is not None and '代办人姓名' in df_person.columns:
    print("\n=== 代办人关联分析 ===")
    agent_groups = df_person.dropna(subset=['代办人姓名']).groupby('代办人姓名')['客户名称'].apply(list)
    shared_agents = agent_groups[agent_groups.apply(len) >= 2]

    print(f"存在代办人信息的客户: {df_person['代办人姓名'].notna().sum()}个")
    print(f"同一代办人代办>=2个账户: {len(shared_agents)}组")

    for agent, clients in shared_agents.items():
        # 检查代办人本人是否也是交易对手
        agent_is_opp = agent in set(df[COL['opp_name']].dropna().unique())
        agent_flag = " [代办人本人也是交易对手!]" if agent_is_opp else ""

        total_amt = sum(
            df[df[COL['opp_name']] == c][COL['amount']].sum()
            for c in clients
        )
        print(f"\n  代办人 {agent}{agent_flag}: 代办 {len(clients)} 个客户")
        for c in clients:
            c_amt = df[df[COL['opp_name']] == c][COL['amount']].sum()
            print(f"    - {c}: 与主体交易 {c_amt:,.2f}元")

    # 代办人证件号碰撞
    if '代办人证件号码' in df_person.columns:
        agent_id_groups = df_person.dropna(subset=['代办人证件号码']).groupby('代办人证件号码')['客户名称'].apply(list)
        shared_agent_ids = agent_id_groups[agent_id_groups.apply(len) >= 2]
        if len(shared_agent_ids) > 0:
            print(f"\n  同一代办人证件号关联>=2个客户: {len(shared_agent_ids)}组")
            for aid, clients in shared_agent_ids.items():
                print(f"    证件号 {aid}: {clients}")
```

### 亲属关系推断（C5）

```python
SELF_ID = df[COL['self_id']].dropna().iloc[0] if COL['self_id'] in df.columns and df[COL['self_id']].notna().any() else None

if SELF_ID and COL.get('opp_id') and COL['opp_id'] in df.columns:
    self_prefix6 = str(SELF_ID)[:6]
    self_birth_year = int(str(SELF_ID)[6:10]) if len(str(SELF_ID)) >= 10 and str(SELF_ID)[6:10].isdigit() else None

    print(f"\n=== 亲属关系推断 ===")
    print(f"主体证件号前6位: {self_prefix6}, 出生年份: {self_birth_year}")

    opp_ids = df.dropna(subset=[COL['opp_account'], COL['opp_id']]).drop_duplicates(
        subset=[COL['opp_account']]
    )[[COL['opp_account'], COL['opp_name'], COL['opp_id']]].copy()

    opp_ids['prefix6'] = opp_ids[COL['opp_id']].astype(str).str[:6]
    opp_ids['birth_year'] = pd.to_numeric(opp_ids[COL['opp_id']].astype(str).str[6:10], errors='coerce')

    # 同户籍地（前6位相同）
    same_region = opp_ids[opp_ids['prefix6'] == self_prefix6]
    if len(same_region) > 0:
        print(f"\n与主体同户籍地的对手: {len(same_region)}个")
        for _, row in same_region.iterrows():
            acct = row[COL['opp_account']]
            name = row[COL['opp_name']]
            birth = row['birth_year']
            age_diff = abs(birth - self_birth_year) if self_birth_year and pd.notna(birth) else '未知'
            txn = df[df[COL['opp_account']] == acct]
            txn_amt = txn[COL['amount']].sum()
            relative_flag = " [可能为亲属]" if isinstance(age_diff, (int, float)) and age_diff <= 30 else ""
            print(f"  {name}({acct}): 出生年差{age_diff}, 交易{len(txn)}笔/{txn_amt:,.2f}元{relative_flag}")

# 同事关系（同一工作单位）
if df_person is not None and '工作单位' in df_person.columns and COL['self_name'] in df.columns:
    self_name = df[COL['self_name']].mode()[0]
    self_unit_row = df_person[df_person['客户名称'] == self_name]
    if len(self_unit_row) > 0:
        self_unit = self_unit_row['工作单位'].iloc[0]
        if pd.notna(self_unit):
            colleagues = df_person[(df_person['工作单位'] == self_unit) & (df_person['客户名称'] != self_name)]
            if len(colleagues) > 0:
                print(f"\n=== 同事关系（同工作单位: {self_unit}）===")
                for _, row in colleagues.iterrows():
                    col_name = row['客户名称']
                    col_txn = df[df[COL['opp_name']] == col_name]
                    if len(col_txn) > 0:
                        print(f"  {col_name}: 与主体交易{len(col_txn)}笔, {col_txn[COL['amount']].sum():,.2f}元")
```

### 白手套账户识别（C6）

```python
# 识别可能的白手套/中间人账户
# 特征：进出对手极少，充当纯粹的资金通道

print("\n=== 白手套账户识别 ===")

# 如果有多账户数据，分析每个账户的进出对手数量
all_accounts = df[COL['self_account']].unique()
if len(all_accounts) > 1:
    for acct in all_accounts:
        acct_df = df[df[COL['self_account']] == acct]
        acct_in = acct_df[acct_df[COL['direction']] == DIR_IN]
        acct_out = acct_df[acct_df[COL['direction']] == DIR_OUT]
        in_opps = acct_in[COL['opp_account']].dropna().nunique()
        out_opps = acct_out[COL['opp_account']].dropna().nunique()
        in_amt = acct_in[COL['amount']].sum()
        out_amt = acct_out[COL['amount']].sum()

        # 白手套特征：进出对手都很少（<=3），且进出金额接近
        is_suspect = (in_opps <= 3 and out_opps <= 3 and in_opps > 0 and out_opps > 0)
        if is_suspect:
            balance_check = abs(in_amt - out_amt) / (in_amt + out_amt) if (in_amt + out_amt) > 0 else 1
            name = acct_df[COL['self_name']].iloc[0] if COL['self_name'] in acct_df.columns else '未知'
            print(f"  {name}({acct}): 进{in_opps}个对手/{in_amt:,.2f}元, "
                  f"出{out_opps}个对手/{out_amt:,.2f}元, 收支差{balance_check*100:.1f}%"
                  f" → {'高度可疑（一进一出型通道）' if balance_check < 0.1 else '关注'}")

# 如果是单账户，检查对手中是否存在疑似白手套
# （某对手既从行贿方收钱，又向受贿方关联人转钱）
# 此分析需要对手的交易明细，仅在多账户数据中可实现
```

### 基于交易关系的关联拓展

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
freq_threshold = 5  # 商业贿赂场景适当降低阈值
opp_freq = df.groupby(COL['opp_account']).size()
high_freq = opp_freq[opp_freq >= freq_threshold]
if len(high_freq) > 0:
    print(f"\n=== 高频对手（>={freq_threshold}笔）===")
    for acct, cnt in high_freq.sort_values(ascending=False).items():
        name = df[df[COL['opp_account']] == acct][COL['opp_name']].iloc[0] if len(df[df[COL['opp_account']] == acct]) > 0 else '未知'
        print(f"  {name}({acct}): {cnt}笔")
```

---

## 5. 可疑特征量化（阶段D）

### 核心指标自动计算

```python
results = {}

# 1. 贿赂伪装摘要笔数
if COL.get('memo') and COL['memo'] in df.columns:
    bribery_pattern = '|'.join(bribery_keywords)
    bribery_mask = df[COL['memo']].astype(str).str.contains(bribery_pattern, na=False)
    results['贿赂伪装摘要笔数'] = int(bribery_mask.sum())
else:
    results['贿赂伪装摘要笔数'] = None

# 2. 大额现金存入笔数
if COL.get('cash_flag') and COL['cash_flag'] in df.columns:
    cash_in_large = df[
        (df[COL['cash_flag']].astype(str).str.contains('现')) &
        (df[COL['direction']] == DIR_IN) &
        (df[COL['amount']] >= 50000)
    ]
    results['大额现金存入笔数'] = len(cash_in_large)
else:
    results['大额现金存入笔数'] = None

# 3. 异常大额进账
avg_in = df_in[COL['amount']].mean() if len(df_in) > 0 else 0
if avg_in > 0:
    large_in = df_in[df_in[COL['amount']] > avg_in * 3]
    results['异常大额进账笔数'] = len(large_in)
else:
    results['异常大额进账笔数'] = 0

# 4. 单向资金关系对手占比
all_opps = df[COL['opp_account']].dropna().unique()
in_only = in_set - out_set
out_only = out_set - in_set
unidirectional = len(in_only) + len(out_only)
results['单向资金关系对手占比'] = unidirectional / len(all_opps) if len(all_opps) > 0 else 0

# 5. 整额交易占比（整万）
round_10k = df[df[COL['amount']] % 10000 == 0]
results['整万交易占比'] = len(round_10k) / total_count if total_count > 0 else 0

# 6. 非工作时间交易占比
df['hour'] = df[COL['time']].dt.hour
df['weekday'] = df[COL['time']].dt.weekday  # 0=Mon, 6=Sun
off_hours = df[(df['hour'] >= 22) | (df['hour'] < 8) | (df['weekday'] >= 5)]
results['非工作时间交易占比'] = len(off_hours) / total_count if total_count > 0 else 0

# 7. 企业转个人占比
enterprise_pattern = '|'.join(enterprise_keywords)
ent_in = df_in[df_in[COL['opp_name']].astype(str).str.contains(enterprise_pattern, na=False)]
results['企业转个人占比'] = ent_in[COL['amount']].sum() / in_amount if in_amount > 0 else 0

# 8. 关联企业对手数
if df_person is not None and '工作单位' in df_person.columns:
    # 统计对手中工作单位存在重叠的不同对手个数
    opp_name_to_unit = {}
    for _, row in df_person.iterrows():
        if pd.notna(row.get('工作单位')):
            opp_name_to_unit[row['客户名称']] = row['工作单位']

    # 检查同单位的对手
    unit_to_opps = defaultdict(set)
    for opp_name in df[COL['opp_name']].dropna().unique():
        if opp_name in opp_name_to_unit:
            unit_to_opps[opp_name_to_unit[opp_name]].add(opp_name)

    # 有重叠的对手数 = 属于有>=2人的单位的对手总数
    related_opps = set()
    for unit, members in unit_to_opps.items():
        if len(members) >= 2:
            related_opps |= members
    results['关联企业对手数'] = len(related_opps)

    # 也考虑法人代表碰撞
    if '法人代表' in df_person.columns:
        legal_to_opps = defaultdict(set)
        for _, row in df_person.iterrows():
            if pd.notna(row.get('法人代表')):
                legal_to_opps[row['法人代表']].add(row['客户名称'])
        for legal, members in legal_to_opps.items():
            if len(members) >= 2:
                related_opps |= members
        results['关联企业对手数'] = len(related_opps)

    # 也考虑单位地址碰撞
    if '单位地址' in df_person.columns:
        addr_to_opps = defaultdict(set)
        for _, row in df_person.iterrows():
            if pd.notna(row.get('单位地址')):
                addr_to_opps[row['单位地址']].add(row['客户名称'])
        for addr, members in addr_to_opps.items():
            if len(members) >= 2:
                related_opps |= members
        results['关联企业对手数'] = len(related_opps)
else:
    results['关联企业对手数'] = None

# 9. 代办人重叠账户数
if df_person is not None and '代办人姓名' in df_person.columns:
    agent_groups_count = df_person.dropna(subset=['代办人姓名']).groupby('代办人姓名')['客户名称'].count()
    max_agent_accounts = agent_groups_count.max() if len(agent_groups_count) > 0 else 0
    results['代办人重叠账户数'] = int(max_agent_accounts)
else:
    results['代办人重叠账户数'] = None

# 10. 收入与支出时间差异（大额进账后7天内等额/近似额支出配对数）
df_in_large = df_in[df_in[COL['amount']] >= 50000].sort_values(COL['time'])
df_out_sorted = df_out.sort_values(COL['time'])
quick_transfer_pairs = 0

used_outs = set()
for _, in_row in df_in_large.iterrows():
    in_time = in_row[COL['time']]
    in_amt = in_row[COL['amount']]
    for j, (_, out_row) in enumerate(df_out_sorted.iterrows()):
        if j in used_outs:
            continue
        out_time = out_row[COL['time']]
        out_amt = out_row[COL['amount']]
        diff_days = (out_time - in_time).total_seconds() / 86400
        if 0 <= diff_days <= 7:
            # 近似额判定：差额在10%以内
            if abs(in_amt - out_amt) / in_amt <= 0.1:
                quick_transfer_pairs += 1
                used_outs.add(j)
                break
        elif diff_days > 7:
            break

results['收入支出时间差异配对数'] = quick_transfer_pairs

# 11. 资金来源集中度
if len(in_top) >= 3:
    results['来源集中度TOP3'] = in_top['total'].head(3).sum() / in_amount if in_amount > 0 else 0
else:
    results['来源集中度TOP3'] = in_top['total'].sum() / in_amount if in_amount > 0 else 0

# 12. 同单位对手集中度
if df_person is not None and '工作单位' in df_person.columns:
    opp_with_unit = 0
    max_unit_count = 0
    for unit, members in unit_to_opps.items():
        opp_in_data = set(df[COL['opp_name']].dropna().unique()) & members
        opp_with_unit += len(opp_in_data)
        max_unit_count = max(max_unit_count, len(opp_in_data))

    total_opps = df[COL['opp_name']].dropna().nunique()
    results['同单位对手集中度'] = max_unit_count / total_opps if total_opps > 0 else 0
else:
    results['同单位对手集中度'] = None
```

### 阈值判定与输出

```python
thresholds = {
    '贿赂伪装摘要笔数':       {'abnormal': 1,    'direction': 'higher'},
    '大额现金存入笔数':       {'abnormal': 1,    'direction': 'higher'},
    '异常大额进账笔数':       {'abnormal': 3,    'direction': 'higher'},
    '单向资金关系对手占比':   {'abnormal': 0.70, 'direction': 'higher'},
    '整万交易占比':           {'abnormal': 0.30, 'direction': 'higher'},
    '非工作时间交易占比':     {'abnormal': 0.15, 'direction': 'higher'},
    '企业转个人占比':         {'abnormal': 0.40, 'direction': 'higher'},
    '关联企业对手数':         {'abnormal': 3,    'direction': 'higher'},
    '代办人重叠账户数':       {'abnormal': 2,    'direction': 'higher'},
    '收入支出时间差异配对数': {'abnormal': 3,    'direction': 'higher'},
    '来源集中度TOP3':         {'abnormal': 0.60, 'direction': 'higher'},
    '同单位对手集中度':       {'abnormal': 0.20, 'direction': 'higher'},
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

    if th['direction'] == 'higher':
        level = '异常' if value >= th['abnormal'] else '正常'
    else:
        level = '异常' if value <= th['abnormal'] else '正常'

    if isinstance(value, float) and 0 < value < 1:
        display = f"{value*100:.2f}%"
    else:
        display = f"{value}"

    marker = '⚠️ 异常' if level == '异常' else '✓ 正常'
    print(f"  [{marker}] {name}: {display}")
```

---

## 6. 大额进账后短期转移检测

```python
def find_transfer_pairs(df, col_time, col_amount, col_direction, col_opp_name,
                        dir_in, dir_out, window_days=7, min_amount=50000):
    """
    检测大额进账后7天内等额或近似额支出的配对。
    返回配对列表 [(进账时间, 进账金额, 进账对手, 出账时间, 出账金额, 出账对手, 间隔天数), ...]
    """
    df_sorted = df.sort_values(col_time).copy()
    ins = df_sorted[df_sorted[col_direction] == dir_in]
    outs = df_sorted[df_sorted[col_direction] == dir_out]

    ins_large = ins[ins[col_amount] >= min_amount]

    pairs = []
    used_outs = set()

    for _, in_row in ins_large.iterrows():
        in_time = in_row[col_time]
        in_amt = in_row[col_amount]
        in_opp = in_row[col_opp_name] if col_opp_name in in_row.index else '未知'

        for idx, out_row in outs.iterrows():
            if idx in used_outs:
                continue
            out_time = out_row[col_time]
            out_amt = out_row[col_amount]
            out_opp = out_row[col_opp_name] if col_opp_name in out_row.index else '未知'

            diff_days = (out_time - in_time).total_seconds() / 86400
            if diff_days < 0:
                continue
            if diff_days > window_days:
                break

            # 近似额：差额在10%以内
            if out_amt >= min_amount and abs(in_amt - out_amt) / in_amt <= 0.1:
                pairs.append((in_time, float(in_amt), in_opp,
                             out_time, float(out_amt), out_opp, diff_days))
                used_outs.add(idx)
                break

    return pairs

pairs = find_transfer_pairs(
    df, COL['time'], COL['amount'], COL['direction'], COL['opp_name'],
    DIR_IN, DIR_OUT, window_days=7, min_amount=50000
)

print(f"\n=== 大额进账后7天内转移配对: {len(pairs)}对 ===")
for in_t, in_a, in_opp, out_t, out_a, out_opp, days in pairs[:20]:
    print(f"  进: {in_t} ({in_a:,.0f}, 来自{in_opp}) → 出: {out_t} ({out_a:,.0f}, 去向{out_opp}), 间隔{days:.1f}天")
```

---

## 7. 大额现金存入检测

```python
if COL.get('cash_flag') and COL['cash_flag'] in df.columns:
    cash_in_large = df[
        (df[COL['cash_flag']].astype(str).str.contains('现')) &
        (df[COL['direction']] == DIR_IN) &
        (df[COL['amount']] >= 50000)
    ].sort_values(COL['time'])

    print(f"\n=== 大额现金存入检测（>=5万元）===")
    print(f"共 {len(cash_in_large)} 笔, 合计 {cash_in_large[COL['amount']].sum():,.2f} 元")

    if len(cash_in_large) > 0:
        # 逐笔展示
        for _, row in cash_in_large.iterrows():
            time_str = row[COL['time']].strftime('%Y-%m-%d %H:%M')
            amt = row[COL['amount']]
            location = row.get(COL['location'], '未知') if COL['location'] in df.columns else '未知'
            branch = row.get(COL['branch'], '未知') if COL['branch'] in df.columns else '未知'
            print(f"  {time_str}: {amt:,.2f}元, 地点={location}, 网点={branch}")

        # 规律性分析
        if len(cash_in_large) >= 3:
            amounts = cash_in_large[COL['amount']].values
            times = cash_in_large[COL['time']].values

            # 金额规律性：是否存在固定金额
            unique_amounts = cash_in_large[COL['amount']].value_counts()
            for amt, cnt in unique_amounts.items():
                if cnt >= 2:
                    print(f"\n  固定金额 {amt:,.2f}元 出现 {cnt} 次")

            # 时间规律性：间隔天数
            time_diffs = pd.Series(times).diff().dt.days.dropna()
            if len(time_diffs) > 0:
                print(f"\n  存入间隔: 最短{time_diffs.min():.0f}天, 最长{time_diffs.max():.0f}天, "
                      f"平均{time_diffs.mean():.1f}天")
```

---

## 8. 行贿路径穿透分析

```python
def bribery_path_penetration(df_main, target_accounts, col_self, col_opp, col_amount, col_direction, dir_in, dir_out):
    """
    从交易明细中进行行贿路径穿透分析。
    target_accounts: 待穿透的账户列表
    """
    result = {}
    for acct in target_accounts:
        acct_data = df_main[df_main[col_self] == acct]
        if len(acct_data) == 0:
            continue

        # 上游（给该账户转入的来源——行贿资金来源）
        upstream = acct_data[acct_data[col_direction] == dir_in].groupby(col_opp)[col_amount].agg(['sum', 'count']).sort_values('sum', ascending=False)
        # 下游（该账户转出的去向——受贿资金消化）
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

penetration = bribery_path_penetration(
    df,
    top_sources + top_targets,
    COL['self_account'],
    COL['opp_account'],
    COL['amount'],
    COL['direction'],
    DIR_IN, DIR_OUT
)

print("\n=== 行贿路径穿透 ===")
for acct, info in penetration.items():
    acct_name = df[df[COL['self_account']] == acct][COL['self_name']].iloc[0] if len(df[df[COL['self_account']] == acct]) > 0 else '未知'
    print(f"\n--- {acct_name}({acct}) ---")
    print(f"总收入: {info['total_in']:,.2f}, 总支出: {info['total_out']:,.2f}")
    if len(info['upstream_top5']) > 0:
        print("  TOP5上游（行贿资金来源）:")
        print(info['upstream_top5'].to_string())
    if len(info['downstream_top5']) > 0:
        print("  TOP5下游（受贿资金去向）:")
        print(info['downstream_top5'].to_string())
```

---

## 9. 报告生成辅助

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

headers = ['序号', '对手账户', '对手户名', '交易笔数', '合计金额', '平均单笔', '特征标注']
rows = []
for i, ((acct, name), row) in enumerate(in_top.head(10).iterrows(), 1):
    rows.append([i, acct, name, row['count'], f"{row['total']:,.2f}", f"{row['avg']:,.2f}", ''])

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

    title = doc.add_heading('商业贿赂犯罪资金分析报告', level=0)
    title.alignment = WD_PARAGRAPH_ALIGNMENT.CENTER

    doc.add_heading('一、基本情况', level=1)
    doc.add_heading('（一）账户基本信息', level=2)
    doc.add_paragraph('此处填写账户基本信息...')

    table = doc.add_table(rows=1, cols=4)
    table.style = 'Table Grid'
    headers = table.rows[0].cells
    headers[0].text = '项目'
    headers[1].text = '笔数'
    headers[2].text = '金额'
    headers[3].text = '占比'

    row = table.add_row().cells
    row[0].text = '收入'
    row[1].text = str(in_count)
    row[2].text = f'{in_amount:,.2f}'
    row[3].text = f'{in_amount/total_amount*100:.1f}%'

    doc.save('商业贿赂犯罪资金分析报告.docx')
    print("报告已生成: 商业贿赂犯罪资金分析报告.docx")

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
