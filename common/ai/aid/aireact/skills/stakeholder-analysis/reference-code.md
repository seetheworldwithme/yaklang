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

files = glob.glob('*.xlsx') + glob.glob('*.xls') + glob.glob('*.csv')
file_map = {}

for f in files:
    try:
        if f.endswith('.csv'):
            df_tmp = pd.read_csv(f, nrows=1, encoding='utf-8')
        else:
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
        elif '会员ID' in cols or ('推荐人' in str(cols) and '层级' in str(cols)):
            file_map['平台后台'] = f
    except Exception as e:
        print(f"读取 {f} 失败: {e}")

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
df_platform = None

if '人员信息' in file_map:
    df_person = pd.read_excel(file_map['人员信息'])
    print(f"\n=== 人员信息 ===\n形状: {df_person.shape}\n列名: {df_person.columns.tolist()}")

if '账户信息' in file_map:
    df_account = pd.read_excel(file_map['账户信息'])
    print(f"\n=== 账户信息 ===\n形状: {df_account.shape}\n列名: {df_account.columns.tolist()}")

if '子账户信息' in file_map:
    df_subaccount = pd.read_excel(file_map['子账户信息'])
    print(f"\n=== 子账户信息 ===\n形状: {df_subaccount.shape}\n列名: {df_subaccount.columns.tolist()}")

if '平台后台' in file_map:
    df_platform = pd.read_excel(file_map['平台后台'])
    print(f"\n=== 平台后台 ===\n形状: {df_platform.shape}\n列名: {df_platform.columns.tolist()}")
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

# --- 交易明细去重 ---
total_before_dedup = len(df)

# 优先使用交易流水号去重
if COL.get('txn_serial') and COL['txn_serial'] in df.columns:
    has_serial = df[COL['txn_serial']].notna() & (df[COL['txn_serial']].astype(str).str.strip() != '')
    df_with_serial = df[has_serial].drop_duplicates(subset=[COL['txn_serial']], keep='first')
    df_without_serial = df[~has_serial]
    # 无流水号的部分用组合字段去重
    dedup_cols_fallback = [COL['time'], COL['amount'], COL['direction']]
    if COL.get('opp_account') and COL['opp_account'] in df.columns:
        dedup_cols_fallback.append(COL['opp_account'])
    if COL.get('self_account') and COL['self_account'] in df.columns:
        dedup_cols_fallback.append(COL['self_account'])
    df_without_serial = df_without_serial.drop_duplicates(subset=dedup_cols_fallback, keep='first')
    df = pd.concat([df_with_serial, df_without_serial], ignore_index=True)
elif COL.get('voucher') and COL['voucher'] in df.columns:
    has_voucher = df[COL['voucher']].notna() & (df[COL['voucher']].astype(str).str.strip() != '')
    df_with_voucher = df[has_voucher].drop_duplicates(subset=[COL['voucher']], keep='first')
    df_without_voucher = df[~has_voucher]
    dedup_cols_fallback = [COL['time'], COL['amount'], COL['direction']]
    if COL.get('opp_account') and COL['opp_account'] in df.columns:
        dedup_cols_fallback.append(COL['opp_account'])
    if COL.get('self_account') and COL['self_account'] in df.columns:
        dedup_cols_fallback.append(COL['self_account'])
    df_without_voucher = df_without_voucher.drop_duplicates(subset=dedup_cols_fallback, keep='first')
    df = pd.concat([df_with_voucher, df_without_voucher], ignore_index=True)
else:
    # 无唯一标识字段，使用组合字段去重
    dedup_cols = [COL['time'], COL['amount'], COL['direction']]
    if COL.get('opp_account') and COL['opp_account'] in df.columns:
        dedup_cols.append(COL['opp_account'])
    if COL.get('self_account') and COL['self_account'] in df.columns:
        dedup_cols.append(COL['self_account'])
    df = df.drop_duplicates(subset=dedup_cols, keep='first')

dedup_removed = total_before_dedup - len(df)
print(f"交易明细去重: 去重前{total_before_dedup}笔, 去重后{len(df)}笔, 移除重复{dedup_removed}笔")

df_in = df[df[COL['direction']] == DIR_IN]
df_out = df[df[COL['direction']] == DIR_OUT]
print(f"收入: {len(df_in)}笔, 支出: {len(df_out)}笔")
```

### 检查主体账户数量与角色初判

```python
if COL['self_account'] in df.columns:
    accounts = df[COL['self_account']].unique()
    print(f"数据中涉及 {len(accounts)} 个主体账户: {accounts.tolist()}")

    for acct in accounts:
        acct_df = df[df[COL['self_account']] == acct]
        acct_in = acct_df[acct_df[COL['direction']] == DIR_IN]
        acct_out = acct_df[acct_df[COL['direction']] == DIR_OUT]
        in_amt = acct_in[COL['amount']].sum()
        out_amt = acct_out[COL['amount']].sum()
        in_opps = acct_in[COL['opp_account']].dropna().nunique()
        out_opps = acct_out[COL['opp_account']].dropna().nunique()
        balance_ratio = abs(in_amt - out_amt) / (in_amt + out_amt) if (in_amt + out_amt) > 0 else 0

        print(f"\n--- 账户 {acct} ---")
        print(f"  收入: {len(acct_in)}笔, {in_amt:,.2f}元, {in_opps}个对手")
        print(f"  支出: {len(acct_out)}笔, {out_amt:,.2f}元, {out_opps}个对手")
        print(f"  收支平衡度: {balance_ratio:.4f}")

        # 角色初判
        if in_opps > 10 and in_amt > out_amt * 1.5:
            print("  → 初判角色: 归集账户（吸金账户）")
        elif out_opps > 10 and out_amt > in_amt * 0.8:
            print("  → 初判角色: 分配账户（提成发放）")
        elif balance_ratio < 0.05:
            print("  → 初判角色: 通道账户（过渡账户）")
        else:
            print("  → 初判角色: 待进一步分析")
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
        print(f"  {name}({card}): 开户时间={open_date}, 开户行={bank}, 类型={acct_type}, 状态={status}")
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

print(f"交易笔数: {total_count}, 总金额: {total_amount:,.2f}")
print(f"收入: {in_count}笔 / {in_amount:,.2f}元")
print(f"支出: {out_count}笔 / {out_amount:,.2f}元")
print(f"时间跨度: {date_min.date()} ~ {date_max.date()}, 共{date_span}天")
print(f"收支平衡度: {balance_ratio:.4f} ({balance_ratio*100:.2f}%)")
print(f"日均交易: {total_count / max(date_span, 1):.1f}笔")
```

### 入门费特征值识别

```python
# 统计收入端各金额出现频次
in_amount_freq = df_in[COL['amount']].value_counts()

# 找出高频出现的整额交易（出现>=5次，且为整百/整千/整万）
feature_amounts = {}
for amt, cnt in in_amount_freq.head(50).items():
    if cnt >= 3:
        if amt % 100 == 0 and amt >= 100:
            feature_amounts[amt] = cnt

print("\n=== 入门费特征值识别 ===")
if feature_amounts:
    for amt, cnt in sorted(feature_amounts.items(), key=lambda x: -x[1]):
        pct = cnt / in_count * 100
        print(f"  {amt:,.0f}元: 出现{cnt}次 ({pct:.1f}%)")

    # 检查倍数关系
    amounts_list = sorted(feature_amounts.keys())
    if len(amounts_list) >= 2:
        base = amounts_list[0]
        multiples = [a / base for a in amounts_list]
        print(f"\n  基准金额: {base:,.0f}元")
        print(f"  倍数关系: {[f'{a:,.0f}={m:.1f}x' for a, m in zip(amounts_list, multiples)]}")
else:
    print("  未发现明显的入门费特征值")

# 支出端特征金额（提成档位）
out_amount_freq = df_out[COL['amount']].value_counts()
feature_out_amounts = {}
for amt, cnt in out_amount_freq.head(50).items():
    if cnt >= 3 and amt % 10 == 0 and amt >= 10:
        feature_out_amounts[amt] = cnt

if feature_out_amounts:
    print("\n=== 提成/返利特征值 ===")
    for amt, cnt in sorted(feature_out_amounts.items(), key=lambda x: -x[1]):
        print(f"  {amt:,.0f}元: 发放{cnt}次")
```

### 年度趋势

```python
df['year'] = df[COL['time']].dt.year

yearly = df.groupby(['year', COL['direction']]).agg(
    count=(COL['amount'], 'count'),
    total=(COL['amount'], 'sum')
).unstack(fill_value=0)

yearly_opp = df.dropna(subset=[COL['opp_account']]).groupby(
    ['year', COL['direction']]
)[COL['opp_account']].nunique().unstack(fill_value=0)

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

---

## 3. 账户角色分类（阶段B）

### 多对一收入分析（归集账户识别）

```python
df['date'] = df[COL['time']].dt.date

# 每天收入端涉及多少个不同对手
daily_in_opps = df_in.groupby([COL['self_account'], df_in[COL['time']].dt.date])[COL['opp_account']].nunique()

# 多对一天数：单日3+个不同对手转入
multi_in_days = daily_in_opps[daily_in_opps >= 3]
total_days = df['date'].nunique()

print("\n=== 多对一收入分析（归集账户识别）===")
for acct in df[COL['self_account']].unique():
    acct_multi = multi_in_days.get(acct, pd.Series())
    if hasattr(acct_multi, '__len__') and len(acct_multi) > 0:
        ratio = len(acct_multi) / total_days * 100
        print(f"  {acct}: 多对一天数={len(acct_multi)} ({ratio:.1f}%), "
              f"最多单日对手数={acct_multi.max()}")
    else:
        print(f"  {acct}: 无明显多对一特征")
```

### 一对多支出分析（分配账户识别）

```python
daily_out_opps = df_out.groupby([COL['self_account'], df_out[COL['time']].dt.date])[COL['opp_account']].nunique()

multi_out_days = daily_out_opps[daily_out_opps >= 3]

print("\n=== 一对多支出分析（分配账户识别）===")
for acct in df[COL['self_account']].unique():
    acct_multi = multi_out_days.get(acct, pd.Series())
    if hasattr(acct_multi, '__len__') and len(acct_multi) > 0:
        ratio = len(acct_multi) / total_days * 100
        print(f"  {acct}: 一对多天数={len(acct_multi)} ({ratio:.1f}%), "
              f"最多单日对手数={acct_multi.max()}")
    else:
        print(f"  {acct}: 无明显一对多特征")
```

### 特征金额交易对手统计

```python
if feature_amounts:
    print("\n=== 特征金额转入对手（疑似入门费缴纳人）===")
    all_feature_amts = list(feature_amounts.keys())

    for amt in all_feature_amts[:5]:
        feat_txns = df_in[df_in[COL['amount']] == amt]
        feat_opps = feat_txns[COL['opp_account']].dropna().unique()
        print(f"\n  金额 {amt:,.0f}元: 共{len(feat_txns)}笔, 涉及{len(feat_opps)}个对手")
        for opp in feat_opps[:10]:
            opp_name = feat_txns[feat_txns[COL['opp_account']] == opp][COL['opp_name']].iloc[0] if len(feat_txns[feat_txns[COL['opp_account']] == opp]) > 0 else '未知'
            opp_cnt = len(feat_txns[feat_txns[COL['opp_account']] == opp])
            print(f"    {opp_name}({opp}): {opp_cnt}笔")
```

### 提成发放规律检测

```python
print("\n=== 提成发放规律检测 ===")

for acct in df[COL['self_account']].unique():
    acct_out = df_out[df_out[COL['self_account']] == acct]
    if len(acct_out) < 10:
        continue

    # 按对手分组，查看是否存在定期发放
    opp_groups = acct_out.groupby(COL['opp_account'])

    regular_payments = []
    for opp, grp in opp_groups:
        if len(grp) < 3:
            continue
        # 检查是否金额相近（标准差/均值 < 0.1）
        amt_mean = grp[COL['amount']].mean()
        amt_std = grp[COL['amount']].std()
        if amt_mean > 0 and (amt_std / amt_mean) < 0.15:
            # 检查时间间隔是否规律
            dates = grp[COL['time']].sort_values()
            intervals = dates.diff().dt.days.dropna()
            if len(intervals) >= 2:
                avg_interval = intervals.mean()
                opp_name = grp[COL['opp_name']].iloc[0] if COL['opp_name'] in grp.columns else '未知'
                regular_payments.append({
                    'account': opp,
                    'name': opp_name,
                    'count': len(grp),
                    'avg_amount': amt_mean,
                    'avg_interval_days': avg_interval,
                    'total': grp[COL['amount']].sum()
                })

    if regular_payments:
        print(f"\n  账户 {acct} 存在定期发放模式:")
        for rp in sorted(regular_payments, key=lambda x: -x['total'])[:10]:
            freq = "月度" if 25 <= rp['avg_interval_days'] <= 35 else \
                   "周度" if 5 <= rp['avg_interval_days'] <= 9 else \
                   f"约{rp['avg_interval_days']:.0f}天"
            print(f"    {rp['name']}({rp['account']}): "
                  f"{rp['count']}次, 均笔{rp['avg_amount']:,.0f}元, "
                  f"频率={freq}, 合计{rp['total']:,.2f}元")
```

---

## 4. 资金来源去向分析（阶段B）

### TOP10 资金来源/去向

```python
def top_counterparties(df_subset, direction_label, n=10):
    grouped = df_subset.groupby([COL['opp_account'], COL['opp_name']]).agg(
        count=(COL['amount'], 'count'),
        total=(COL['amount'], 'sum'),
        avg=(COL['amount'], 'mean'),
        first_time=(COL['time'], 'min'),
        last_time=(COL['time'], 'max')
    ).sort_values('total', ascending=False)

    opp_ids = {}
    if COL.get('opp_id') and COL['opp_id'] in df_subset.columns:
        opp_ids = df_subset.dropna(subset=[COL['opp_account'], COL['opp_id']]).drop_duplicates(
            subset=[COL['opp_account']]
        ).set_index(COL['opp_account'])[COL['opp_id']].to_dict()

    print(f"\n=== TOP{n} {direction_label} ===")
    for i, ((acct, name), row) in enumerate(grouped.head(n).iterrows(), 1):
        opp_id_info = opp_ids.get(acct, '未知')
        print(f"{i}. {name}({acct}): {row['count']}笔, "
              f"合计{row['total']:,.2f}, 均笔{row['avg']:,.2f}, 证件: {opp_id_info}")

    return grouped

in_top = top_counterparties(df_in, '资金来源')
out_top = top_counterparties(df_out, '资金去向')
```

### 集中度计算

```python
def concentration(grouped_total, overall_total):
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

in_set = set(df_in[COL['opp_account']].dropna().unique())
out_set = set(df_out[COL['opp_account']].dropna().unique())
bidirectional = in_set & out_set
print(f"\n双向交易对手: {len(bidirectional)}个")
for acct in sorted(bidirectional)[:20]:
    name = df[df[COL['opp_account']] == acct][COL['opp_name']].iloc[0]
    in_amt = df_in[df_in[COL['opp_account']] == acct][COL['amount']].sum()
    out_amt = df_out[df_out[COL['opp_account']] == acct][COL['amount']].sum()
    print(f"  {name}({acct}): 进{in_amt:,.2f}, 出{out_amt:,.2f}")
```

---

## 5. 层级架构分析（阶段C）

### 基于资金流向的层级推断

```python
def trace_fund_hierarchy(df_main, known_accounts, col_self, col_opp, col_amount, 
                         col_direction, dir_in, dir_out, feature_amounts=None, max_depth=3):
    """
    从已知账户向上迭代追溯，推断资金层级。
    known_accounts: 已知的底层参与人账户列表
    feature_amounts: 特征金额列表（入门费档位）
    返回层级字典: {level: [(account, name, total_received, total_sent, upstream_accounts)]}
    """
    hierarchy = {}
    visited = set()
    current_level_accounts = set(known_accounts)
    
    for depth in range(max_depth):
        if not current_level_accounts:
            break
            
        next_level_accounts = set()
        level_info = []
        
        for acct in current_level_accounts:
            if acct in visited:
                continue
            visited.add(acct)
            
            # 找出该账户出账的去向（即其上级）
            acct_out = df_main[(df_main[col_self] == acct) & (df_main[col_direction] == dir_out)]
            
            if feature_amounts:
                # 优先找特征金额的去向
                feat_out = acct_out[acct_out[col_amount].isin(feature_amounts)]
                if len(feat_out) > 0:
                    acct_out = feat_out
            
            for opp in acct_out[col_opp].dropna().unique():
                if opp not in visited and opp not in current_level_accounts:
                    next_level_accounts.add(opp)
                    
                    opp_received = df_main[
                        (df_main[col_self] == opp) & (df_main[col_direction] == dir_in)
                    ][col_amount].sum()
                    opp_sent = df_main[
                        (df_main[col_self] == opp) & (df_main[col_direction] == dir_out)
                    ][col_amount].sum()
                    
                    opp_name_rows = df_main[df_main[col_opp] == opp]
                    opp_name = opp_name_rows[COL['opp_name']].iloc[0] if len(opp_name_rows) > 0 else '未知'
                    
                    level_info.append((opp, opp_name, opp_received, opp_sent))
        
        if level_info:
            hierarchy[depth + 1] = level_info
        current_level_accounts = next_level_accounts
    
    return hierarchy

# 使用示例：从底层参与人向上追溯
# bottom_accounts = [...]  # 已知的底层参与人账户
# hierarchy = trace_fund_hierarchy(df, bottom_accounts, COL['self_account'], ...)
```

### 迭代挖掘吸金账户

```python
def find_collection_accounts(df_main, col_self, col_opp, col_amount, col_direction, 
                              dir_in, feature_amounts=None, min_sources=5):
    """
    寻找吸金账户：接收大量不同来源资金的账户。
    min_sources: 至少来自N个不同来源才视为吸金账户
    """
    # 统计每个账户作为收款方时的来源数
    in_records = df_main[df_main[col_direction] == dir_in]
    
    if feature_amounts:
        # 仅统计特征金额的交易
        in_records = in_records[in_records[col_amount].isin(feature_amounts)]
    
    account_sources = in_records.groupby(col_self).agg(
        source_count=(col_opp, 'nunique'),
        total_received=(col_amount, 'sum'),
        txn_count=(col_amount, 'count')
    ).sort_values('source_count', ascending=False)
    
    collection_accounts = account_sources[account_sources['source_count'] >= min_sources]
    
    print(f"\n=== 疑似吸金/归集账户（来源>={min_sources}个）===")
    for acct, row in collection_accounts.iterrows():
        out_amt = df_main[(df_main[col_self] == acct) & (df_main[col_direction] == DIR_OUT)][col_amount].sum()
        retention = row['total_received'] - out_amt
        print(f"  {acct}: 来源{row['source_count']}个, 收入{row['total_received']:,.2f}, "
              f"支出{out_amt:,.2f}, 沉淀{retention:,.2f}")
    
    return collection_accounts

collection = find_collection_accounts(
    df, COL['self_account'], COL['opp_account'], COL['amount'],
    COL['direction'], DIR_IN, 
    feature_amounts=list(feature_amounts.keys()) if feature_amounts else None
)
```

### 共同交易对手分析（碰撞分析）

```python
def find_common_counterparties(df_main, accounts_list, col_self, col_opp, col_direction, dir_out):
    """
    碰撞多个账户的交易流水，找出共同转款对象。
    用于发现未掌握的吸金账户。
    """
    account_targets = {}
    for acct in accounts_list:
        targets = set(df_main[
            (df_main[col_self] == acct) & (df_main[col_direction] == dir_out)
        ][col_opp].dropna().unique())
        account_targets[acct] = targets
    
    # 找出被多个账户共同转款的对手
    all_targets = set()
    for targets in account_targets.values():
        all_targets |= targets
    
    common_targets = {}
    for target in all_targets:
        sources = [acct for acct, targets in account_targets.items() if target in targets]
        if len(sources) >= 2:
            common_targets[target] = sources
    
    print(f"\n=== 共同交易对手（被{len(accounts_list)}个账户中>=2个共同转款）===")
    for target, sources in sorted(common_targets.items(), key=lambda x: -len(x[1])):
        target_name_rows = df_main[df_main[col_opp] == target]
        name = target_name_rows[COL['opp_name']].iloc[0] if len(target_name_rows) > 0 else '未知'
        print(f"  {name}({target}): 被{len(sources)}个账户转入")
    
    return common_targets
```

---

## 6. 团伙划分（阶段C）

### Union-Find 算法

```python
class UnionFind:
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

### 基于 IP/MAC 的团伙聚类

```python
has_ip = COL.get('ip') and COL['ip'] in df.columns
has_mac = COL.get('mac') and COL['mac'] in df.columns

if has_ip or has_mac:
    # 从出方向交易中提取IP/MAC关联
    out_records = df[df[COL['direction']] == DIR_OUT].copy()
    
    if has_ip:
        ip_out = out_records.dropna(subset=[COL['ip']])
        ip_account_count = ip_out.groupby(COL['ip'])[COL['self_account']].nunique()
        shared_ips = ip_account_count[ip_account_count >= 2]
        print(f"\n共享IP（出方向，>=2个卡号）: {len(shared_ips)}个")
        for ip, cnt in shared_ips.sort_values(ascending=False).head(20).items():
            accounts = ip_out[ip_out[COL['ip']] == ip][COL['self_account']].unique()
            print(f"  {ip}: 涉及{cnt}个卡号 → {accounts.tolist()[:10]}")

    if has_mac:
        mac_out = out_records.dropna(subset=[COL['mac']])
        mac_account_count = mac_out.groupby(COL['mac'])[COL['self_account']].nunique()
        shared_macs = mac_account_count[mac_account_count >= 2]
        print(f"\n共享MAC（出方向，>=2个卡号）: {len(shared_macs)}个")

    # Union-Find 聚类
    uf = UnionFind()
    
    if has_ip:
        ip_data = out_records.dropna(subset=[COL['ip']])
        for ip, group in ip_data.groupby(COL['ip']):
            accounts = group[COL['self_account']].unique().tolist()
            for i in range(1, len(accounts)):
                uf.union(accounts[0], accounts[i])

    if has_mac:
        mac_data = out_records.dropna(subset=[COL['mac']])
        for mac, group in mac_data.groupby(COL['mac']):
            accounts = group[COL['self_account']].unique().tolist()
            for i in range(1, len(accounts)):
                uf.union(accounts[0], accounts[i])

    # 确保所有账户都进入并查集
    accounts_with_device = set()
    if has_ip:
        accounts_with_device |= set(ip_data[COL['self_account']].unique())
    if has_mac:
        accounts_with_device |= set(mac_data[COL['self_account']].unique())
    for acct in accounts_with_device:
        uf.find(acct)

    gangs = uf.get_groups()
    gangs = {k: v for k, v in gangs.items() if len(v) >= 2}

    print(f"\n识别出 {len(gangs)} 个设备关联团伙（>=2人）:")
    for i, (root, members) in enumerate(sorted(gangs.items(), key=lambda x: -len(x[1])), 1):
        print(f"\n--- 团伙{i} ({len(members)}人) ---")
        for m in sorted(members):
            txn = df[df[COL['opp_account']] == m]
            amt = txn[COL['amount']].sum() if len(txn) > 0 else 0
            print(f"  {m}: 与主体交易{len(txn)}笔, {amt:,.2f}元")
```

### 基于对手证件号前6位的地域聚合

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

    for prefix, row in region_gangs.sort_values('count', ascending=False).head(10).iterrows():
        accounts = row['accounts']
        total_amt = sum(
            df[df[COL['opp_account']] == acct][COL['amount']].sum()
            for acct in accounts
        )
        print(f"\n  地区代码 {prefix}: {len(accounts)}人, 交易合计{total_amt:,.2f}元")

    # 地域集中度
    if len(region_gangs) > 0:
        max_region = region_gangs['accounts'].apply(len).max()
        region_conc = max_region / total_opp_with_id * 100
        print(f"\n最大地域组占比: {max_region}/{total_opp_with_id} = {region_conc:.1f}%")
```

### 基于开户银行的聚类

```python
if COL.get('opp_bank') and COL['opp_bank'] in df.columns:
    bank_accounts = df.dropna(subset=[COL['opp_account'], COL['opp_bank']]).drop_duplicates(
        subset=[COL['opp_account']]
    )
    bank_groups = bank_accounts.groupby(COL['opp_bank']).agg(
        account_count=(COL['opp_account'], 'count'),
    ).sort_values('account_count', ascending=False)

    print(f"\n=== 对手开户银行聚类 ===")
    for bank, row in bank_groups.head(10).iterrows():
        print(f"  {bank}: {row['account_count']}个对手账户")
```

### 基于交易关系的团伙拓展

```python
# 同名多户
opp_name_accounts = df.dropna(subset=[COL['opp_account'], COL['opp_name']]).drop_duplicates(
    subset=[COL['opp_account']]
).groupby(COL['opp_name'])[COL['opp_account']].apply(set)

multi_name = opp_name_accounts[opp_name_accounts.apply(len) >= 2]
print(f"\n=== 同名多户 ===")
for name, accounts in multi_name.items():
    total = sum(df[df[COL['opp_account']] == acct][COL['amount']].sum() for acct in accounts)
    print(f"  {name}: {len(accounts)}个账户, 合计{total:,.2f}元")

# 涉众关键词搜索
if COL.get('memo') and COL['memo'] in df.columns:
    keywords = ['入门费', '会员费', '加盟费', '升级费', '培训费', '工资', '提成', 
                '奖金', '分红', '服务费', '投资款', '本金', '利息', '返利', '收益',
                '回报', '解债', '易物', '消费币', '积分', '会费', '份额', '充值', '提现']
    pattern = '|'.join(keywords)
    keyword_txns = df[df[COL['memo']].astype(str).str.contains(pattern, na=False)]
    
    if len(keyword_txns) > 0:
        print(f"\n=== 涉众关键词交易 ===")
        print(f"共{len(keyword_txns)}笔, 金额{keyword_txns[COL['amount']].sum():,.2f}元")
        
        for kw in keywords:
            kw_txns = keyword_txns[keyword_txns[COL['memo']].astype(str).str.contains(kw, na=False)]
            if len(kw_txns) > 0:
                print(f"  '{kw}': {len(kw_txns)}笔, {kw_txns[COL['amount']].sum():,.2f}元")
```

---

## 7. 可疑特征量化（阶段D）

### 核心指标自动计算

```python
results = {}

# 1. 收支平衡度
results['收支平衡度'] = abs(in_amount - out_amount) / (in_amount + out_amount) if (in_amount + out_amount) > 0 else None

# 2. 入门费特征值命中率
if feature_amounts:
    feat_amt_list = list(feature_amounts.keys())
    feat_in_count = df_in[df_in[COL['amount']].isin(feat_amt_list)].shape[0]
    results['入门费命中率'] = feat_in_count / in_count if in_count > 0 else 0
else:
    results['入门费命中率'] = 0

# 3. 散进整出比（单日多笔小额收入+少笔大额支出的天数）
daily_stats = df.groupby(['date', COL['direction']]).agg(
    count=(COL['amount'], 'count'),
    mean_amt=(COL['amount'], 'mean')
)
# （简化版本：比较每天收入/支出的笔数差和均额差）

# 4. 现金交易占比
if COL.get('cash_flag') and COL['cash_flag'] in df.columns:
    cash_mask = df[COL['cash_flag']].astype(str).str.contains('现')
    results['现金交易占比'] = cash_mask.sum() / total_count
else:
    results['现金交易占比'] = None

# 5. 对手集中度
if len(in_top) >= 3:
    results['来源集中度TOP3'] = in_top['total'].head(3).sum() / in_amount if in_amount > 0 else None
if len(out_top) >= 3:
    results['去向集中度TOP3'] = out_top['total'].head(3).sum() / out_amount if out_amount > 0 else None

# 6. 多对一收入比
daily_in_opp_count = df_in.groupby(df_in[COL['time']].dt.date)[COL['opp_account']].nunique()
multi_in_ratio = (daily_in_opp_count >= 3).sum() / total_days if total_days > 0 else 0
results['多对一收入比'] = multi_in_ratio

# 7. 一对多支出比
daily_out_opp_count = df_out.groupby(df_out[COL['time']].dt.date)[COL['opp_account']].nunique()
multi_out_ratio = (daily_out_opp_count >= 3).sum() / total_days if total_days > 0 else 0
results['一对多支出比'] = multi_out_ratio

# 8. 定额交易占比
top5_amounts = df[COL['amount']].value_counts().head(5)
results['定额交易占比'] = top5_amounts.sum() / total_count if total_count > 0 else 0

# 9. 快进快出天数占比
daily_in_amt = df_in.groupby(df_in[COL['time']].dt.date)[COL['amount']].sum()
daily_out_amt = df_out.groupby(df_out[COL['time']].dt.date)[COL['amount']].sum()
daily_combined = pd.DataFrame({'in': daily_in_amt, 'out': daily_out_amt}).fillna(0)
quick_days = ((daily_combined['in'] > 50000) & (daily_combined['out'] > 50000)).sum()
results['快进快出天数占比'] = quick_days / total_days if total_days > 0 else 0

# 10. 资金周转率
if COL['balance'] in df.columns:
    avg_balance = pd.to_numeric(df[COL['balance']], errors='coerce').mean()
    results['资金周转率'] = total_amount / avg_balance if avg_balance > 0 else None
else:
    results['资金周转率'] = None

# 11. 同名账户数
results['同名账户数'] = df[df[COL['opp_name']] == SELF_NAME][COL['opp_account']].nunique() if COL['opp_name'] in df.columns else 0

# 12. 涉众关键词笔数
if COL.get('memo') and COL['memo'] in df.columns:
    stakeholder_keywords = ['入门费', '会员费', '加盟费', '培训费', '工资', '提成', 
                            '奖金', '分红', '投资款', '返利', '收益', '解债', '易物']
    pattern = '|'.join(stakeholder_keywords)
    results['涉众关键词笔数'] = df[COL['memo']].astype(str).str.contains(pattern, na=False).sum()
else:
    results['涉众关键词笔数'] = None

# 13. 地域集中度
if COL.get('opp_id') and COL['opp_id'] in df.columns:
    opp_ids_clean = df.dropna(subset=[COL['opp_account'], COL['opp_id']]).drop_duplicates(subset=[COL['opp_account']])
    opp_ids_clean['prefix6'] = opp_ids_clean[COL['opp_id']].astype(str).str[:6]
    valid_ids = opp_ids_clean[opp_ids_clean['prefix6'].str.match(r'^\d{6}', na=False)]
    if len(valid_ids) > 0:
        max_region_count = valid_ids['prefix6'].value_counts().max()
        results['地域集中度'] = max_region_count / len(valid_ids)
    else:
        results['地域集中度'] = None
else:
    results['地域集中度'] = None

# 14. 开户银行集中度
if COL.get('opp_bank') and COL['opp_bank'] in df.columns:
    opp_banks_clean = df.dropna(subset=[COL['opp_account'], COL['opp_bank']]).drop_duplicates(subset=[COL['opp_account']])
    if len(opp_banks_clean) > 0:
        top1_bank = opp_banks_clean[COL['opp_bank']].value_counts().max()
        results['开户银行集中度'] = top1_bank / len(opp_banks_clean)
    else:
        results['开户银行集中度'] = None
else:
    results['开户银行集中度'] = None
```

### 阈值判定与输出

```python
thresholds = {
    '收支平衡度':        {'normal': 0.20, 'abnormal': 0.10, 'high': 0.03, 'direction': 'lower'},
    '入门费命中率':      {'normal': 0.05, 'abnormal': 0.15, 'high': 0.30, 'direction': 'higher'},
    '现金交易占比':      {'normal': 0.20, 'abnormal': 0.40, 'high': 0.60, 'direction': 'higher'},
    '来源集中度TOP3':    {'normal': 0.30, 'abnormal': 0.40, 'high': 0.60, 'direction': 'higher'},
    '去向集中度TOP3':    {'normal': 0.30, 'abnormal': 0.40, 'high': 0.60, 'direction': 'higher'},
    '多对一收入比':      {'normal': 0.05, 'abnormal': 0.15, 'high': 0.30, 'direction': 'higher'},
    '一对多支出比':      {'normal': 0.05, 'abnormal': 0.15, 'high': 0.30, 'direction': 'higher'},
    '定额交易占比':      {'normal': 0.10, 'abnormal': 0.25, 'high': 0.40, 'direction': 'higher'},
    '快进快出天数占比':  {'normal': 0.05, 'abnormal': 0.10, 'high': 0.20, 'direction': 'higher'},
    '资金周转率':        {'normal': 20,   'abnormal': 50,   'high': 100,  'direction': 'higher'},
    '同名账户数':        {'normal': 0,    'abnormal': 2,    'high': 5,    'direction': 'higher'},
    '涉众关键词笔数':    {'normal': 0,    'abnormal': 1,    'high': 10,   'direction': 'higher'},
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

    marker = '🔴' if level == '高度异常' else ('🟡' if level == '异常' else '🟢')
    print(f"  {marker} {name}: {display} → {level}")
```

---

## 8. 快进快出检测

```python
def find_quick_pairs(df, col_time, col_amount, col_direction, dir_in, dir_out, window_minutes=30):
    df_sorted = df.sort_values(col_time).copy()
    ins = df_sorted[df_sorted[col_direction] == dir_in][[col_time, col_amount]].values
    outs = df_sorted[df_sorted[col_direction] == dir_out][[col_time, col_amount]].values

    pairs = []
    used_outs = set()

    for in_time, in_amt in ins:
        if in_amt < 50000:
            continue
        for j, (out_time, out_amt) in enumerate(outs):
            if j in used_outs or out_amt < 50000:
                continue
            diff = (out_time - in_time)
            if hasattr(diff, 'total_seconds'):
                diff_minutes = diff.total_seconds() / 60
            else:
                diff_minutes = float(diff) / 60e9

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

## 9. 资金穿透分析

```python
def fund_penetration(df_main, target_accounts, col_self, col_opp, col_amount, 
                     col_direction, dir_in, dir_out):
    result = {}
    for acct in target_accounts:
        acct_data = df_main[df_main[col_self] == acct]
        if len(acct_data) == 0:
            continue

        upstream = acct_data[acct_data[col_direction] == dir_in].groupby(col_opp)[col_amount].agg(
            ['sum', 'count']).sort_values('sum', ascending=False)
        downstream = acct_data[acct_data[col_direction] == dir_out].groupby(col_opp)[col_amount].agg(
            ['sum', 'count']).sort_values('sum', ascending=False)

        result[acct] = {
            'upstream_top5': upstream.head(5),
            'downstream_top5': downstream.head(5),
            'total_in': acct_data[acct_data[col_direction] == dir_in][col_amount].sum(),
            'total_out': acct_data[acct_data[col_direction] == dir_out][col_amount].sum(),
        }

    return result

top_sources = in_top.head(5).index.get_level_values(0).tolist()
top_targets = out_top.head(5).index.get_level_values(0).tolist()

penetration = fund_penetration(
    df, top_sources + top_targets,
    COL['self_account'], COL['opp_account'], COL['amount'],
    COL['direction'], DIR_IN, DIR_OUT
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

## 10. 非吸资金链路闭环分析与可视化

### 闭环链路识别

```python
def find_fund_loops(df, col_self, col_opp, col_amount, col_direction, col_opp_name,
                    dir_in, dir_out, pool_accounts=None, return_accounts=None):
    """
    识别非吸资金闭环链路：投资人→资金池→返利账户→投资人
    pool_accounts: 已识别的资金池账户列表
    return_accounts: 已识别的返利账户列表
    """
    loops = []
    
    # 找出所有向资金池转入的对手（疑似投资人）
    if pool_accounts:
        investor_to_pool = df[
            (df[col_direction] == dir_in) & 
            (df[col_self].isin(pool_accounts))
        ].groupby(col_opp).agg(
            invest_amount=(col_amount, 'sum'),
            invest_count=(col_amount, 'count'),
            opp_name=(col_opp_name, 'first')
        ).reset_index()
        
        # 找出从返利账户收到资金的对手
        if return_accounts:
            pool_to_return = df[
                (df[col_direction] == dir_out) &
                (df[col_self].isin(pool_accounts)) &
                (df[col_opp].isin(return_accounts))
            ]
            return_to_investor = df[
                (df[col_direction] == dir_out) &
                (df[col_self].isin(return_accounts))
            ].groupby(col_opp).agg(
                return_amount=(col_amount, 'sum'),
                return_count=(col_amount, 'count')
            ).reset_index()
            
            # 碰撞：投资人 ∩ 返利接收人 = 闭环
            investor_set = set(investor_to_pool[col_opp].dropna())
            return_receiver_set = set(return_to_investor[col_opp].dropna())
            loop_accounts = investor_set & return_receiver_set
            
            for acct in loop_accounts:
                inv_row = investor_to_pool[investor_to_pool[col_opp] == acct].iloc[0]
                ret_row = return_to_investor[return_to_investor[col_opp] == acct].iloc[0]
                loops.append({
                    'investor': acct,
                    'investor_name': inv_row.get('opp_name', '未知'),
                    'invest_amount': inv_row['invest_amount'],
                    'return_amount': ret_row['return_amount'],
                    'return_ratio': ret_row['return_amount'] / inv_row['invest_amount'] 
                                    if inv_row['invest_amount'] > 0 else 0,
                })
        
        # 也检测庞氏特征：资金池直接向投资人支付（还本付息）
        pool_direct_return = df[
            (df[col_direction] == dir_out) &
            (df[col_self].isin(pool_accounts))
        ].groupby(col_opp).agg(
            repay_amount=(col_amount, 'sum'),
            repay_count=(col_amount, 'count')
        ).reset_index()
        
        direct_loop = set(investor_to_pool[col_opp].dropna()) & set(pool_direct_return[col_opp].dropna())
        for acct in direct_loop:
            if acct not in [l['investor'] for l in loops]:
                inv_row = investor_to_pool[investor_to_pool[col_opp] == acct].iloc[0]
                rep_row = pool_direct_return[pool_direct_return[col_opp] == acct].iloc[0]
                loops.append({
                    'investor': acct,
                    'investor_name': inv_row.get('opp_name', '未知'),
                    'invest_amount': inv_row['invest_amount'],
                    'return_amount': rep_row['repay_amount'],
                    'return_ratio': rep_row['repay_amount'] / inv_row['invest_amount']
                                    if inv_row['invest_amount'] > 0 else 0,
                    'direct_repay': True,
                })
    
    print(f"\n=== 非吸资金链路闭环分析 ===")
    print(f"存在资金回流的投资人数: {len(loops)}")
    if loops:
        total_invest = sum(l['invest_amount'] for l in loops)
        total_return = sum(l['return_amount'] for l in loops)
        print(f"闭环涉及投资总额: {total_invest:,.2f}元")
        print(f"闭环回流总额: {total_return:,.2f}元")
        print(f"平均回流比例: {total_return/total_invest*100:.1f}%")
        
        for i, l in enumerate(sorted(loops, key=lambda x: -x['invest_amount'])[:10], 1):
            tag = "（庞氏直接还本付息）" if l.get('direct_repay') else ""
            print(f"  {i}. {l['investor_name']}({l['investor']}): "
                  f"投入{l['invest_amount']:,.2f}→回流{l['return_amount']:,.2f}, "
                  f"回流比例{l['return_ratio']*100:.1f}%{tag}")
    
    return loops
```

### 资金链路闭环可视化图

```python
import matplotlib.pyplot as plt
import matplotlib
matplotlib.rcParams['font.sans-serif'] = ['SimHei', 'DejaVu Sans']
matplotlib.rcParams['axes.unicode_minus'] = False

try:
    import networkx as nx
    HAS_NETWORKX = True
except ImportError:
    HAS_NETWORKX = False
    print("networkx未安装，尝试 pip install networkx")

def draw_fund_loop_graph(loops, pool_accounts_info, return_accounts_info, 
                          save_path='资金链路闭环图.png'):
    """
    生成非吸资金链路闭环有向图。
    loops: find_fund_loops返回的闭环列表
    pool_accounts_info: dict, {账户号: {'name': 户名, 'total_in': 总收入}}
    return_accounts_info: dict, {账户号: {'name': 户名, 'total_out': 总支出}}
    """
    if not HAS_NETWORKX:
        print("缺少networkx库，跳过可视化")
        return
    
    G = nx.DiGraph()
    node_colors = {}
    node_labels = {}
    
    COLOR_MAP = {
        'investor': '#4A90D9',    # 蓝色-投资人
        'pool': '#D94A4A',        # 红色-资金池
        'return': '#4AD97A',      # 绿色-返利账户
        'transit': '#D9A04A',     # 橙色-归集/通道
        'controller': '#9B59B6',  # 紫色-实控人
    }
    
    # 选取TOP闭环展示（避免图太密集）
    top_loops = sorted(loops, key=lambda x: -x['invest_amount'])[:8]
    
    for pool_acct, info in pool_accounts_info.items():
        label = f"{info['name']}\n(资金池)"
        G.add_node(pool_acct)
        node_colors[pool_acct] = COLOR_MAP['pool']
        node_labels[pool_acct] = label
    
    for ret_acct, info in return_accounts_info.items():
        label = f"{info['name']}\n(返利)"
        G.add_node(ret_acct)
        node_colors[ret_acct] = COLOR_MAP['return']
        node_labels[ret_acct] = label
    
    for loop in top_loops:
        inv_acct = loop['investor']
        inv_label = f"{loop['investor_name']}\n(投资人)"
        G.add_node(inv_acct)
        node_colors[inv_acct] = COLOR_MAP['investor']
        node_labels[inv_acct] = inv_label
        
        # 投资人→资金池
        for pool_acct in pool_accounts_info:
            G.add_edge(inv_acct, pool_acct, 
                       weight=loop['invest_amount'],
                       label=f"投资{loop['invest_amount']/10000:.1f}万")
        
        # 资金池→返利账户→投资人 或 资金池→投资人（直接还本付息）
        if loop.get('direct_repay'):
            for pool_acct in pool_accounts_info:
                G.add_edge(pool_acct, inv_acct,
                           weight=loop['return_amount'],
                           label=f"还本付息{loop['return_amount']/10000:.1f}万",
                           style='dashed')
        else:
            for ret_acct in return_accounts_info:
                G.add_edge(list(pool_accounts_info.keys())[0], ret_acct,
                           weight=loop['return_amount'],
                           label=f"返利拨付")
                G.add_edge(ret_acct, inv_acct,
                           weight=loop['return_amount'],
                           label=f"返利{loop['return_amount']/10000:.1f}万")
    
    fig, ax = plt.subplots(1, 1, figsize=(16, 12))
    pos = nx.spring_layout(G, k=2, iterations=50, seed=42)
    
    colors = [node_colors.get(n, '#CCCCCC') for n in G.nodes()]
    labels = {n: node_labels.get(n, n) for n in G.nodes()}
    
    nx.draw_networkx_nodes(G, pos, node_color=colors, node_size=2000, alpha=0.9, ax=ax)
    nx.draw_networkx_labels(G, pos, labels, font_size=8, ax=ax)
    nx.draw_networkx_edges(G, pos, edge_color='#555555', arrows=True, 
                           arrowsize=20, width=1.5, ax=ax)
    
    edge_labels = nx.get_edge_attributes(G, 'label')
    nx.draw_networkx_edge_labels(G, pos, edge_labels, font_size=7, ax=ax)
    
    # 图例
    from matplotlib.patches import Patch
    legend_elements = [
        Patch(facecolor=COLOR_MAP['investor'], label='疑似投资人'),
        Patch(facecolor=COLOR_MAP['pool'], label='资金池账户'),
        Patch(facecolor=COLOR_MAP['return'], label='返利账户'),
        Patch(facecolor=COLOR_MAP['transit'], label='归集/通道账户'),
        Patch(facecolor=COLOR_MAP['controller'], label='实控人账户'),
    ]
    ax.legend(handles=legend_elements, loc='upper left', fontsize=10)
    ax.set_title('非吸资金链路闭环图', fontsize=16, fontweight='bold')
    plt.tight_layout()
    plt.savefig(save_path, dpi=150, bbox_inches='tight')
    plt.close()
    print(f"资金链路闭环图已保存: {save_path}")
```

---

## 11. 疑似投资人群体画像分析

```python
from datetime import datetime

# 中国行政区划代码前2位→省份映射
PROVINCE_MAP = {
    '11': '北京', '12': '天津', '13': '河北', '14': '山西', '15': '内蒙古',
    '21': '辽宁', '22': '吉林', '23': '黑龙江', '31': '上海', '32': '江苏',
    '33': '浙江', '34': '安徽', '35': '福建', '36': '江西', '37': '山东',
    '41': '河南', '42': '湖北', '43': '湖南', '44': '广东', '45': '广西',
    '46': '海南', '50': '重庆', '51': '四川', '52': '贵州', '53': '云南',
    '54': '西藏', '61': '陕西', '62': '甘肃', '63': '青海', '64': '宁夏',
    '65': '新疆',
}

def analyze_investor_profile(df, col_opp_id, col_opp_account, col_opp_name, 
                              col_amount, col_direction, dir_in,
                              pool_accounts=None, collection_accounts=None):
    """
    基于疑似投资人证件号分析总人数、地域分布、年龄结构、性别分布。
    pool_accounts/collection_accounts: 已识别的资金池/归集账户列表，用于筛选投资人
    """
    # 筛选收入端交易（向资金池/归集账户转入的对手即为疑似投资人）
    df_invest = df[df[col_direction] == dir_in].copy()
    if pool_accounts or collection_accounts:
        target_accounts = list(pool_accounts or []) + list(collection_accounts or [])
        if COL.get('self_account') and COL['self_account'] in df.columns:
            df_invest = df_invest[df_invest[COL['self_account']].isin(target_accounts)]
    
    # 按证件号去重，得到唯一投资人
    investors = df_invest.dropna(subset=[col_opp_id]).drop_duplicates(subset=[col_opp_id]).copy()
    investors_no_id = df_invest[df_invest[col_opp_id].isna()].drop_duplicates(subset=[col_opp_account])
    
    # 每个投资人的投资总额
    investor_amounts = df_invest.groupby(col_opp_id)[col_amount].sum().reset_index()
    investor_amounts.columns = ['证件号', '投资总额']
    investors = investors.merge(investor_amounts, left_on=col_opp_id, right_on='证件号', how='left')
    
    print(f"\n=== 疑似投资人群体画像 ===")
    print(f"有证件号的投资人数（去重）: {len(investors)}")
    print(f"仅有户名无证件号的投资人数: {len(investors_no_id)}")
    print(f"合计投资人数（下限估计）: {len(investors) + len(investors_no_id)}")
    
    # --- 地域分布 ---
    investors['id_str'] = investors[col_opp_id].astype(str).str.strip()
    valid_18 = investors['id_str'].str.match(r'^\d{17}[\dXx]$', na=False)
    investors_valid = investors[valid_18].copy()
    
    investors_valid['province_code'] = investors_valid['id_str'].str[:2]
    investors_valid['province'] = investors_valid['province_code'].map(PROVINCE_MAP).fillna('未知')
    investors_valid['region_code6'] = investors_valid['id_str'].str[:6]
    
    province_stats = investors_valid.groupby('province').agg(
        count=('id_str', 'count'),
        total_amount=('投资总额', 'sum')
    ).sort_values('count', ascending=False)
    province_stats['占比'] = province_stats['count'] / province_stats['count'].sum() * 100
    province_stats['人均投资'] = province_stats['total_amount'] / province_stats['count']
    
    print(f"\n--- 地域分布（TOP10省份）---")
    for province, row in province_stats.head(10).iterrows():
        print(f"  {province}: {row['count']}人 ({row['占比']:.1f}%), "
              f"投资总额{row['total_amount']:,.2f}元, 人均{row['人均投资']:,.2f}元")
    
    # --- 年龄结构 ---
    def parse_birth_date(id_str):
        try:
            return datetime.strptime(id_str[6:14], '%Y%m%d')
        except:
            return None
    
    investors_valid['birth_date'] = investors_valid['id_str'].apply(parse_birth_date)
    investors_valid = investors_valid.dropna(subset=['birth_date'])
    
    today = datetime.now()
    investors_valid['age'] = investors_valid['birth_date'].apply(
        lambda x: (today - x).days // 365 if x else None
    )
    
    age_bins = [0, 30, 40, 50, 60, 120]
    age_labels = ['18-30岁', '31-40岁', '41-50岁', '51-60岁', '60岁以上']
    investors_valid['age_group'] = pd.cut(investors_valid['age'], bins=age_bins, labels=age_labels, right=True)
    
    age_stats = investors_valid.groupby('age_group', observed=True).agg(
        count=('id_str', 'count'),
        total_amount=('投资总额', 'sum')
    )
    age_stats['占比'] = age_stats['count'] / age_stats['count'].sum() * 100
    age_stats['人均投资'] = age_stats['total_amount'] / age_stats['count']
    
    print(f"\n--- 年龄结构 ---")
    for age_grp, row in age_stats.iterrows():
        print(f"  {age_grp}: {row['count']}人 ({row['占比']:.1f}%), "
              f"投资总额{row['total_amount']:,.2f}元, 人均{row['人均投资']:,.2f}元")
    
    # --- 性别分布 ---
    def get_gender(id_str):
        try:
            return '男' if int(id_str[16]) % 2 == 1 else '女'
        except:
            return '未知'
    
    investors_valid['gender'] = investors_valid['id_str'].apply(get_gender)
    gender_stats = investors_valid.groupby('gender').agg(
        count=('id_str', 'count'),
        total_amount=('投资总额', 'sum')
    )
    gender_stats['占比'] = gender_stats['count'] / gender_stats['count'].sum() * 100
    
    print(f"\n--- 性别分布 ---")
    for gender, row in gender_stats.iterrows():
        print(f"  {gender}: {row['count']}人 ({row['占比']:.1f}%), 投资总额{row['total_amount']:,.2f}元")
    
    return {
        'total_with_id': len(investors),
        'total_without_id': len(investors_no_id),
        'province_stats': province_stats,
        'age_stats': age_stats,
        'gender_stats': gender_stats,
        'avg_age': investors_valid['age'].mean() if len(investors_valid) > 0 else None,
    }

# 使用示例
# profile = analyze_investor_profile(
#     df, COL['opp_id'], COL['opp_account'], COL['opp_name'],
#     COL['amount'], COL['direction'], DIR_IN,
#     pool_accounts=[...], collection_accounts=[...]
# )
```

---

## 12. 非吸资金实际用途分类

```python
def classify_fund_usage(df, col_opp_account, col_opp_name, col_amount, col_direction, 
                         col_memo, dir_out, col_self_account=None,
                         pool_accounts=None, investor_accounts=None, 
                         controller_accounts=None, commission_recipients=None):
    """
    对资金池/吸金账户的支出进行用途分类。
    pool_accounts: 资金池账户列表
    investor_accounts: 已识别的投资人账户集合
    controller_accounts: 实控人关联账户集合
    commission_recipients: 已识别的提成接收人账户集合
    """
    # 筛选资金池的支出交易
    df_pool_out = df[df[col_direction] == dir_out].copy()
    if pool_accounts and col_self_account and col_self_account in df.columns:
        df_pool_out = df_pool_out[df_pool_out[col_self_account].isin(pool_accounts)]
    
    if investor_accounts is None:
        investor_accounts = set()
    if controller_accounts is None:
        controller_accounts = set()
    if commission_recipients is None:
        commission_recipients = set()
    
    # 用途分类关键词
    USAGE_KEYWORDS = {
        '还本付息': ['利息', '返利', '本金', '还款', '兑付', '回报', '收益', '分红'],
        '业务提成': ['工资', '提成', '佣金', '奖金', '绩效', '薪资', '报酬'],
        '个人挥霍': ['消费', '购物', '餐饮', '旅游', '娱乐', '酒店', '奢侈', 'POS'],
        '购置资产': ['购房', '购车', '房款', '车款', '首付', '房产', '不动产', '按揭'],
        '运营开支': ['房租', '租金', '物业', '广告', '技术服务', '平台维护', '办公', '水电', '装修'],
        '关联公司': ['往来款', '借款', '拆借'],
    }
    
    def classify_single(row):
        opp_acct = row.get(col_opp_account, '')
        memo = str(row.get(col_memo, '')) if col_memo and col_memo in row.index else ''
        
        # 基于对手账户判断
        if opp_acct in investor_accounts:
            return '还本付息'
        if opp_acct in commission_recipients:
            return '业务提成'
        if opp_acct in controller_accounts:
            return '个人挥霍'
        
        # 基于摘要关键词判断
        for usage, keywords in USAGE_KEYWORDS.items():
            if any(kw in memo for kw in keywords):
                return usage
        
        # 基于对手户名特征判断
        opp_name = str(row.get(col_opp_name, ''))
        if any(kw in opp_name for kw in ['房地产', '置业', '地产', '房产']):
            return '购置资产'
        if any(kw in opp_name for kw in ['汽车', '4S', '车行']):
            return '购置资产'
        if any(kw in opp_name for kw in ['广告', '传媒', '科技', '网络', '信息']):
            return '运营开支'
        
        return '其他/不明'
    
    df_pool_out['用途分类'] = df_pool_out.apply(classify_single, axis=1)
    
    # 汇总统计
    usage_stats = df_pool_out.groupby('用途分类').agg(
        金额=(col_amount, 'sum'),
        笔数=(col_amount, 'count')
    ).sort_values('金额', ascending=False)
    
    total_out = usage_stats['金额'].sum()
    usage_stats['占比'] = usage_stats['金额'] / total_out * 100
    
    print(f"\n=== 非吸资金实际用途分类 ===")
    print(f"资金池支出总额: {total_out:,.2f}元")
    for usage, row in usage_stats.iterrows():
        print(f"  {usage}: {row['金额']:,.2f}元 ({row['占比']:.1f}%), {int(row['笔数'])}笔")
    
    # 关键比例
    repay = usage_stats.loc['还本付息', '金额'] if '还本付息' in usage_stats.index else 0
    personal = (
        (usage_stats.loc['个人挥霍', '金额'] if '个人挥霍' in usage_stats.index else 0) +
        (usage_stats.loc['购置资产', '金额'] if '购置资产' in usage_stats.index else 0)
    )
    operation = usage_stats.loc['运营开支', '金额'] if '运营开支' in usage_stats.index else 0
    
    print(f"\n关键比例指标:")
    print(f"  还本付息占比: {repay/total_out*100:.1f}%")
    print(f"  个人挥霍+购置资产占比: {personal/total_out*100:.1f}%")
    print(f"  运营开支占比: {operation/total_out*100:.1f}%")
    
    if personal / total_out > 0.3:
        print("  → 个人挥霍+购置资产占比较高，倾向认定存在非法占有目的（集资诈骗）")
    if repay / total_out > 0.5:
        print("  → 还本付息占比过半，呈现典型庞氏骗局特征")
    
    return usage_stats, df_pool_out

# 使用示例
# usage_stats, df_classified = classify_fund_usage(
#     df, COL['opp_account'], COL['opp_name'], COL['amount'], COL['direction'],
#     COL.get('memo'), DIR_OUT, col_self_account=COL['self_account'],
#     pool_accounts=[...], investor_accounts=set([...]),
#     controller_accounts=set([...]), commission_recipients=set([...])
# )
```

### 资金用途饼图

```python
def draw_fund_usage_pie(usage_stats, save_path='资金用途分类饼图.png'):
    """生成资金实际用途分类饼图"""
    fig, ax = plt.subplots(figsize=(10, 8))
    
    colors = ['#E74C3C', '#F39C12', '#9B59B6', '#3498DB', '#2ECC71', '#1ABC9C', '#95A5A6']
    
    labels = usage_stats.index.tolist()
    sizes = usage_stats['金额'].tolist()
    pcts = usage_stats['占比'].tolist()
    
    def make_autopct(sizes):
        def autopct(pct):
            total = sum(sizes)
            val = pct * total / 100.0
            return f'{pct:.1f}%\n({val/10000:,.1f}万元)'
        return autopct
    
    wedges, texts, autotexts = ax.pie(
        sizes, labels=labels, colors=colors[:len(labels)],
        autopct=make_autopct(sizes), startangle=90, pctdistance=0.75
    )
    
    for autotext in autotexts:
        autotext.set_fontsize(9)
    
    ax.set_title('非吸资金实际用途分类', fontsize=16, fontweight='bold', pad=20)
    plt.tight_layout()
    plt.savefig(save_path, dpi=150, bbox_inches='tight')
    plt.close()
    print(f"资金用途饼图已保存: {save_path}")

# draw_fund_usage_pie(usage_stats)
```

---

## 13. 涉案规模估算

```python
def estimate_stakeholder_scale(df, col_amount, col_direction, col_opp_account, col_opp_name,
                                dir_in, dir_out, feature_amounts=None):
    """估算涉众犯罪的涉案规模"""
    df_in = df[df[col_direction] == dir_in]
    df_out = df[df[col_direction] == dir_out]
    
    result = {
        'total_txn_count': len(df),
        'total_txn_amount': df[col_amount].sum(),
        'total_in_amount': df_in[col_amount].sum(),
        'total_out_amount': df_out[col_amount].sum(),
        'in_counterparties': df_in[col_opp_account].dropna().nunique(),
        'out_counterparties': df_out[col_opp_account].dropna().nunique(),
        'date_range': f"{df[COL['time']].min().date()} ~ {df[COL['time']].max().date()}",
    }
    
    # 基于入门费估算参与人数
    if feature_amounts:
        feat_txns = df_in[df_in[col_amount].isin(feature_amounts)]
        result['fee_txn_count'] = len(feat_txns)
        result['fee_total'] = feat_txns[col_amount].sum()
        result['fee_unique_payers'] = feat_txns[col_opp_account].dropna().nunique()
        
        # 估算参与人数（去重后的缴费人数）
        result['estimated_participants'] = result['fee_unique_payers']
    else:
        # 无特征金额时，用收入端对手数作为下限估计
        result['estimated_participants'] = result['in_counterparties']
    
    # 提成/返利估算
    stakeholder_kw = ['工资', '提成', '奖金', '分红', '返利', '收益']
    if COL.get('memo') and COL['memo'] in df.columns:
        pattern = '|'.join(stakeholder_kw)
        commission_txns = df_out[df_out[COL['memo']].astype(str).str.contains(pattern, na=False)]
        result['commission_total'] = commission_txns[col_amount].sum()
        result['commission_count'] = len(commission_txns)
    
    # 资金沉淀估算
    result['fund_retention'] = result['total_in_amount'] - result['total_out_amount']
    
    print("\n=== 涉案规模估算 ===")
    for k, v in result.items():
        if isinstance(v, float):
            print(f"  {k}: {v:,.2f}")
        else:
            print(f"  {k}: {v}")
    
    return result

scale = estimate_stakeholder_scale(
    df, COL['amount'], COL['direction'], COL['opp_account'], COL['opp_name'],
    DIR_IN, DIR_OUT,
    feature_amounts=list(feature_amounts.keys()) if feature_amounts else None
)
```

---

## 14. 报告生成辅助

### Markdown 格式化表格

```python
def to_md_table(headers, rows):
    lines = []
    lines.append('| ' + ' | '.join(str(h) for h in headers) + ' |')
    lines.append('| ' + ' | '.join('---' for _ in headers) + ' |')
    for row in rows:
        lines.append('| ' + ' | '.join(str(v) for v in row) + ' |')
    return '\n'.join(lines)
```

### Word 报告生成（使用 python-docx）

```python
from docx import Document
from docx.shared import Pt, Cm, RGBColor
from docx.enum.text import WD_PARAGRAPH_ALIGNMENT

def create_stakeholder_report():
    doc = Document()

    title = doc.add_heading('涉众型经济犯罪资金分析报告', level=0)
    title.alignment = WD_PARAGRAPH_ALIGNMENT.CENTER

    doc.add_heading('一、基本情况', level=1)
    doc.add_heading('（一）案件背景', level=2)
    doc.add_paragraph('此处填写案件背景...')

    doc.add_heading('（二）涉案账户基本信息', level=2)

    # 账户信息表
    table = doc.add_table(rows=1, cols=3)
    table.style = 'Table Grid'
    headers = table.rows[0].cells
    headers[0].text = '账户'
    headers[1].text = '户名'
    headers[2].text = '角色'

    doc.add_heading('（三）交易概况', level=2)

    table = doc.add_table(rows=1, cols=5)
    table.style = 'Table Grid'
    headers = table.rows[0].cells
    headers[0].text = '项目'
    headers[1].text = '笔数'
    headers[2].text = '金额'
    headers[3].text = '占比'
    headers[4].text = '对手账户数'

    row = table.add_row().cells
    row[0].text = '收入'
    row[1].text = f'{in_count}'
    row[2].text = f'{in_amount:,.2f}'
    row[3].text = f'{in_amount/total_amount*100:.1f}%'
    row[4].text = f'{df_in[COL["opp_account"]].dropna().nunique()}'

    doc.save('涉众型经济犯罪资金分析报告.docx')
    print("报告已生成")

# create_stakeholder_report()
```

### f-string 防错规范（重要）

生成 Word 报告时，所有包含变量引用的字符串**必须使用 f-string**（在引号前加 `f` 前缀），否则 `{变量名}` 会作为字面量原样输出到文档中。

**错误写法**：
```python
doc.add_paragraph('该账户收入{in_count}笔，支出{out_count}笔')  # 缺少 f 前缀！
```

**正确写法**：
```python
doc.add_paragraph(f'该账户收入{in_count}笔，支出{out_count}笔')
```

**检查方法**：生成报告后，打开 docx 文件搜索 `{` 字符。如果文档中出现 `{r[`、`{in_count}` 等原始占位符文本，说明有字符串遗漏了 `f` 前缀。
