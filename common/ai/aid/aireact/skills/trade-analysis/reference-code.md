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
from itertools import combinations

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
    except:
        try:
            df_tmp = pd.read_csv(f, nrows=1, encoding='gbk')
        except:
            continue
    cols = set(df_tmp.columns.tolist())

    if '交易时间' in cols and '收付标志' in cols:
        file_map['交易明细'] = f
    elif '客户名称' in cols and '证照号码' in cols:
        file_map['人员信息'] = f
    elif '账户开户名称' in cols and '开户网点' in cols:
        file_map['账户信息'] = f
    elif '子账户账号' in cols and '子账户类别' in cols:
        file_map['子账户信息'] = f
    elif '企业名称' in cols and '统一社会信用代码' in cols:
        file_map['企业信息'] = f
    elif '合同编号' in cols or '项目编号' in cols:
        file_map['合同信息'] = f

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
df_company = None
df_contract = None

if '人员信息' in file_map:
    df_person = pd.read_excel(file_map['人员信息'])
    print(f"\n=== 人员信息 ===\n形状: {df_person.shape}\n列名: {df_person.columns.tolist()}")

if '账户信息' in file_map:
    df_account = pd.read_excel(file_map['账户信息'])
    print(f"\n=== 账户信息 ===\n形状: {df_account.shape}\n列名: {df_account.columns.tolist()}")

if '企业信息' in file_map:
    df_company = pd.read_excel(file_map['企业信息'])
    print(f"\n=== 企业信息 ===\n形状: {df_company.shape}\n列名: {df_company.columns.tolist()}")

if '合同信息' in file_map:
    df_contract = pd.read_excel(file_map['合同信息'])
    print(f"\n=== 合同信息 ===\n形状: {df_contract.shape}\n列名: {df_contract.columns.tolist()}")
```

### 列名适配与数据预处理

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

### 检查主体账户数量与性质

```python
CORP_KEYWORDS = ['公司', '有限', '集团', '企业', '事务所', '合伙', '工厂', '厂']

def is_corporate(name):
    """判断户名是否为企业账户"""
    if pd.isna(name):
        return False
    return any(kw in str(name) for kw in CORP_KEYWORDS)

if COL['self_account'] in df.columns:
    accounts = df[COL['self_account']].unique()
    print(f"数据中涉及 {len(accounts)} 个主体账户:")
    for acct in accounts:
        name = df[df[COL['self_account']] == acct][COL['self_name']].iloc[0] if COL['self_name'] in df.columns else '未知'
        nature = '企业账户' if is_corporate(name) else '个人账户'
        cnt = len(df[df[COL['self_account']] == acct])
        print(f"  {name}({acct}): {nature}, {cnt}笔交易")
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

company_lookup = {}
if df_company is not None:
    for _, row in df_company.iterrows():
        company_lookup[row.get('企业名称', '')] = row.to_dict()
    print(f"企业信息已加载: {len(company_lookup)}条")

if df_account is not None:
    print("\n=== 涉案账户信息 ===")
    for _, row in df_account.iterrows():
        name = row.get('账户开户名称', '未知')
        card = row.get('交易卡号', '未知')
        nature = '企业' if is_corporate(name) else '个人'
        open_date = row.get('账号开户时间', '未知')
        bank = row.get('账号开户银行', '未知')
        status = row.get('账户状态', '未知')
        print(f"  [{nature}] {name}({card}): 开户={open_date}, 银行={bank}, 状态={status}")
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
print(f"收支平衡度: {balance_ratio*100:.2f}%")
print(f"日均交易: {total_count / max(date_span, 1):.1f}笔")
```

### 对公/对私交易比例

```python
def classify_counterparty(name):
    """对手账户按个人/企业分类"""
    if pd.isna(name):
        return '无对手信息'
    return '企业' if is_corporate(str(name)) else '个人'

df['opp_nature'] = df[COL['opp_name']].apply(classify_counterparty)

opp_nature_stats = df.groupby([COL['direction'], 'opp_nature']).agg(
    count=(COL['amount'], 'count'),
    total=(COL['amount'], 'sum')
)
print("\n=== 对公/对私交易分布 ===")
print(opp_nature_stats)

out_to_personal = df_out[df_out['opp_nature'] == '个人'][COL['amount']].sum()
out_to_personal_ratio = out_to_personal / out_amount * 100 if out_amount > 0 else 0
print(f"\n支出中对私转出占比: {out_to_personal_ratio:.1f}%")
```

### 年度/月度趋势

```python
df['year'] = df[COL['time']].dt.year
df['month'] = df[COL['time']].dt.to_period('M')

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
bins = [0, 1000, 5000, 10000, 50000, 100000, 500000, 1000000, float('inf')]
labels = ['0-1千', '1千-5千', '5千-1万', '1万-5万', '5万-10万', '10万-50万', '50万-100万', '100万以上']

df['amount_bin'] = pd.cut(df[COL['amount']], bins=bins, labels=labels, right=True)
amount_dist = df.groupby([COL['direction'], 'amount_bin'], observed=True).size().unstack(fill_value=0)
print("\n=== 金额分布 ===")
print(amount_dist)
```

### 交易摘要关键词分析

```python
if COL['memo'] in df.columns:
    import re
    
    biz_keywords = {
        '工程款': r'工程[款费]?|施工[款费]',
        '保证金': r'保证金|押金|履约金',
        '货款': r'货款|货物|采购|购[货物]',
        '投标': r'投标|招标|中标|标书',
        '合同款': r'合同[款费]|合同付款',
        '借款': r'借[款贷]|还[款贷]|贷款',
        '工资': r'工资|薪[酬资]|劳务费',
        '利息': r'利息|利率|计息',
        '投资': r'投资|出资|入股|分红',
        '转账': r'转[账帐]|汇[款入]|划[款转]',
        '现金': r'现金|取[现款]|存[现款]|ATM',
        '回扣': r'回扣|好处费|佣金|提成|中介费',
        '租金': r'租[金赁费]|房租',
    }
    
    memo_series = df[COL['memo']].astype(str)
    
    print("\n=== 交易摘要关键词分析 ===")
    for category, pattern in biz_keywords.items():
        mask = memo_series.str.contains(pattern, na=False, flags=re.IGNORECASE)
        if mask.sum() > 0:
            amount = df.loc[mask, COL['amount']].sum()
            print(f"  {category}: {mask.sum()}笔, {amount:,.2f}元")
    
    # 可疑关键词特别标记
    suspicious_keywords = ['回扣', '好处费', '佣金', '提成', '送礼', '红包', '感谢费']
    suspicious_pattern = '|'.join(suspicious_keywords)
    suspicious_mask = memo_series.str.contains(suspicious_pattern, na=False)
    if suspicious_mask.sum() > 0:
        print(f"\n  [警告] 发现可疑关键词交易: {suspicious_mask.sum()}笔")
        print(df[suspicious_mask][[COL['time'], COL['amount'], COL['opp_name'], COL['memo']]].to_string())
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

    # 大额现金明细（>5万）
    large_cash = df[cash_mask & (df[COL['amount']] >= 50000)]
    if len(large_cash) > 0:
        print(f"\n大额现金交易（>=5万）: {len(large_cash)}笔")
        print(large_cash[[COL['time'], COL['amount'], COL['direction'], COL['opp_name'], COL['memo']]].to_string())
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

### TOP10 资金来源/去向

```python
def top_counterparties(df_subset, direction_label, n=10):
    """统计TOP-N对手账户，含证件号、账户性质"""
    grouped = df_subset.dropna(subset=[COL['opp_account']]).groupby(
        [COL['opp_account'], COL['opp_name']]
    ).agg(
        count=(COL['amount'], 'count'),
        total=(COL['amount'], 'sum'),
        avg=(COL['amount'], 'mean'),
        first_time=(COL['time'], 'min'),
        last_time=(COL['time'], 'max')
    ).sort_values('total', ascending=False)

    print(f"\n=== TOP{n} {direction_label} ===")
    for i, ((acct, name), row) in enumerate(grouped.head(n).iterrows(), 1):
        nature = '企业' if is_corporate(name) else '个人'
        print(f"{i}. [{nature}] {name}({acct}): {row['count']}笔, "
              f"合计{row['total']:,.2f}, 均笔{row['avg']:,.2f}, "
              f"{row['first_time'].date()}~{row['last_time'].date()}")

    return grouped

in_top = top_counterparties(df_in, '资金来源')
out_top = top_counterparties(df_out, '资金去向')
```

### 集中度计算

```python
def concentration(grouped_total, overall_total):
    result = {}
    cumsum = grouped_total.cumsum()
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
for acct in sorted(bidirectional):
    name = df[df[COL['opp_account']] == acct][COL['opp_name']].iloc[0]
    in_amt = df_in[df_in[COL['opp_account']] == acct][COL['amount']].sum()
    out_amt = df_out[df_out[COL['opp_account']] == acct][COL['amount']].sum()
    net = in_amt - out_amt
    print(f"  {name}({acct}): 进{in_amt:,.2f}, 出{out_amt:,.2f}, 净额{net:,.2f}")
```

---

## 4. 合同诈骗专项分析（阶段C1）

### 合同款收到后资金转移速度

```python
def track_fund_after_receipt(df, large_receipts, col_time, col_amount, col_direction, dir_out, days_window=3):
    """
    追踪大额收款后N天内的资金转出情况。
    large_receipts: 大额收入记录的DataFrame
    """
    results = []
    
    for _, receipt in large_receipts.iterrows():
        recv_time = receipt[col_time]
        recv_amount = receipt[col_amount]
        recv_source = receipt.get(COL['opp_name'], '未知')
        
        window_start = recv_time
        window_end = recv_time + pd.Timedelta(days=days_window)
        
        outflows = df[
            (df[col_direction] == dir_out) &
            (df[col_time] >= window_start) &
            (df[col_time] <= window_end)
        ]
        
        total_outflow = outflows[col_amount].sum()
        outflow_ratio = total_outflow / recv_amount * 100 if recv_amount > 0 else 0
        
        main_targets = outflows.groupby(COL['opp_name'])[col_amount].sum().sort_values(ascending=False).head(3)
        
        results.append({
            '来源': recv_source,
            '收款时间': recv_time,
            '收款金额': recv_amount,
            f'{days_window}天内转出': total_outflow,
            '转出比例': f'{outflow_ratio:.1f}%',
            '主要去向': ', '.join(f'{n}({a:,.0f})' for n, a in main_targets.items())
        })
    
    return pd.DataFrame(results)

# 找出大额收入（如>50万，可按实际调整）
large_receipts = df_in[df_in[COL['amount']] >= 500000].sort_values(COL['time'])
print(f"大额收入（>=50万）: {len(large_receipts)}笔")

if len(large_receipts) > 0:
    transfer_analysis = track_fund_after_receipt(
        df, large_receipts, COL['time'], COL['amount'], COL['direction'], DIR_OUT, days_window=3
    )
    print("\n=== 合同款收到后3天内转出情况 ===")
    print(transfer_analysis.to_string(index=False))
```

### 履约能力评估

```python
def assess_performance_capability(df, contract_date, contract_amount, col_time, col_balance):
    """评估签约日前的账户资金状况"""
    if col_balance not in df.columns:
        print("无余额数据，跳过履约能力评估")
        return None
    
    pre_contract = df[df[col_time] < contract_date].copy()
    pre_contract['balance_num'] = pd.to_numeric(pre_contract[col_balance], errors='coerce')
    
    if len(pre_contract) == 0:
        return None
    
    pre_30d = pre_contract[pre_contract[col_time] >= contract_date - pd.Timedelta(days=30)]
    
    avg_balance = pre_30d['balance_num'].mean() if len(pre_30d) > 0 else pre_contract['balance_num'].mean()
    max_balance = pre_contract['balance_num'].max()
    
    ratio = avg_balance / contract_amount * 100 if contract_amount > 0 else 0
    
    print(f"\n=== 履约能力评估 ===")
    print(f"合同金额: {contract_amount:,.2f}")
    print(f"签约前30天平均余额: {avg_balance:,.2f} (占合同金额{ratio:.1f}%)")
    print(f"历史最高余额: {max_balance:,.2f}")
    
    if ratio < 5:
        print("→ 判定：签约前账户资金严重不足，不具备履约的基本资金能力")
    elif ratio < 20:
        print("→ 判定：签约前账户资金不足，履约能力存疑")
    else:
        print("→ 判定：签约前账户有一定资金储备")
    
    return {'avg_balance': avg_balance, 'max_balance': max_balance, 'ratio': ratio}
```

### 资金挪用分类

```python
def classify_expenditure(df_out, col_opp_name, col_memo, col_amount):
    """将支出按用途分类"""
    categories = {
        '合同相关支出': [],
        '向个人账户转出': [],
        '消费类支出': [],
        '其他支出': [],
    }
    
    contract_keywords = r'工程|材料|施工|加工|采购|供[货应]|运输|物流'
    consume_keywords = r'消费|餐饮|酒店|购物|娱乐|旅游|加油'
    
    for _, row in df_out.iterrows():
        name = str(row.get(col_opp_name, ''))
        memo = str(row.get(col_memo, ''))
        amount = row[col_amount]
        
        import re
        if re.search(contract_keywords, memo + name):
            categories['合同相关支出'].append(amount)
        elif not is_corporate(name) and name != 'nan':
            categories['向个人账户转出'].append(amount)
        elif re.search(consume_keywords, memo):
            categories['消费类支出'].append(amount)
        else:
            categories['其他支出'].append(amount)
    
    print("\n=== 支出用途分类 ===")
    total_out = df_out[col_amount].sum()
    for cat, amounts in categories.items():
        cat_total = sum(amounts)
        cat_count = len(amounts)
        ratio = cat_total / total_out * 100 if total_out > 0 else 0
        print(f"  {cat}: {cat_count}笔, {cat_total:,.2f}元 ({ratio:.1f}%)")

classify_expenditure(df_out, COL['opp_name'], COL['memo'], COL['amount'])
```

---

## 5. 职务侵占专项分析（阶段C2）

### 逆向可疑对手筛查

```python
def inverse_suspicious_counterparty_screening(df, col_opp_account, col_opp_name, col_amount, col_time):
    """
    逆向思维筛查可疑对手：
    放弃"大金额、多次数"常用维度，
    侧重筛查"冷门""零星""偶然"的交易对手。
    """
    opp_stats = df.dropna(subset=[col_opp_account]).groupby(
        [col_opp_account, col_opp_name]
    ).agg(
        count=(col_amount, 'count'),
        total=(col_amount, 'sum'),
        avg=(col_amount, 'mean'),
        first_time=(col_time, 'min'),
        last_time=(col_time, 'max')
    )
    
    # 计算账户的常规交易特征
    median_count = opp_stats['count'].median()
    median_amount = opp_stats['avg'].median()
    q25_amount = opp_stats['avg'].quantile(0.25)
    q75_amount = opp_stats['avg'].quantile(0.75)
    
    suspicious = []
    for (acct, name), row in opp_stats.iterrows():
        reasons = []
        
        # 低频对手（仅1-2次交易）
        if row['count'] <= 2:
            reasons.append('低频偶发')
        
        # 金额异常（不在常规区间内）
        if row['avg'] < q25_amount * 0.5 or row['avg'] > q75_amount * 2:
            reasons.append('金额偏离常规')
        
        # 时间集中（所有交易集中在很短时间内）
        if row['count'] >= 2:
            span = (row['last_time'] - row['first_time']).days
            if span <= 3:
                reasons.append('时间高度集中')
        
        # 对手户名与主体业务无关联
        if not is_corporate(name) and row['total'] > 10000:
            reasons.append('非企业对手大额转出')
        
        if reasons:
            suspicious.append({
                '对手账户': acct,
                '对手户名': name,
                '交易笔数': row['count'],
                '交易总额': row['total'],
                '异常原因': '、'.join(reasons)
            })
    
    suspicious_df = pd.DataFrame(suspicious).sort_values('交易总额', ascending=False)
    
    print(f"\n=== 逆向筛查可疑对手: {len(suspicious_df)}个 ===")
    for _, row in suspicious_df.head(20).iterrows():
        print(f"  {row['对手户名']}({row['对手账户']}): "
              f"{row['交易笔数']}笔, {row['交易总额']:,.2f}元 → {row['异常原因']}")
    
    return suspicious_df

suspicious_opps = inverse_suspicious_counterparty_screening(
    df_out, COL['opp_account'], COL['opp_name'], COL['amount'], COL['time']
)
```

### 周期性转出检测

```python
def detect_periodic_transfers(df_out, col_time, col_amount, col_opp_account, 
                              amount_tolerance=0.1, day_tolerance=3):
    """
    检测固定周期的相似金额转出（职务侵占蚕食模式）。
    amount_tolerance: 金额允许偏差比例
    day_tolerance: 日期允许偏差天数
    """
    opp_groups = df_out.groupby(col_opp_account)
    
    periodic_results = []
    for acct, group in opp_groups:
        if len(group) < 3:
            continue
        
        sorted_g = group.sort_values(col_time)
        amounts = sorted_g[col_amount].values
        dates = sorted_g[col_time].values
        
        # 检查金额相似性
        median_amt = np.median(amounts)
        if median_amt == 0:
            continue
        amt_deviation = np.std(amounts) / median_amt
        if amt_deviation > amount_tolerance:
            continue
        
        # 检查时间间隔规律性
        intervals = []
        for i in range(1, len(dates)):
            delta = (dates[i] - dates[i-1]) / np.timedelta64(1, 'D')
            intervals.append(delta)
        
        if len(intervals) == 0:
            continue
        
        median_interval = np.median(intervals)
        if median_interval < 7:  # 间隔至少7天才算周期性
            continue
        
        interval_deviation = np.std(intervals)
        if interval_deviation <= day_tolerance * 2:
            name = sorted_g[COL['opp_name']].iloc[0]
            periodic_results.append({
                '对手账户': acct,
                '对手户名': name,
                '交易次数': len(group),
                '平均金额': median_amt,
                '平均间隔天数': median_interval,
                '总金额': group[col_amount].sum()
            })
    
    if periodic_results:
        print(f"\n=== 周期性转出检测: {len(periodic_results)}个可疑对手 ===")
        for r in periodic_results:
            print(f"  {r['对手户名']}({r['对手账户']}): "
                  f"{r['交易次数']}次, 均笔{r['平均金额']:,.2f}, "
                  f"间隔约{r['平均间隔天数']:.0f}天, 累计{r['总金额']:,.2f}")
    
    return periodic_results

periodic = detect_periodic_transfers(
    df_out, COL['time'], COL['amount'], COL['opp_account']
)
```

---

## 6. 行受贿存取款碰撞分析（阶段C3）

### 存取款时间金额碰撞

```python
def deposit_withdrawal_collision(df_briber, df_receiver, 
                                  col_time, col_amount, col_direction,
                                  dir_out, dir_in,
                                  time_window_hours=48, amount_tolerance=0.1):
    """
    行贿人取款与受贿人存款的碰撞分析。
    df_briber: 行贿人交易明细
    df_receiver: 受贿人交易明细
    """
    # 行贿人的取款/转出记录（含现金取款）
    briber_out = df_briber[df_briber[col_direction] == dir_out].sort_values(col_time)
    # 受贿人的存款/转入记录（含现金存入）
    receiver_in = df_receiver[df_receiver[col_direction] == dir_in].sort_values(col_time)
    
    collisions = []
    used_receiver = set()
    
    for _, b_row in briber_out.iterrows():
        b_time = b_row[col_time]
        b_amount = b_row[col_amount]
        
        if b_amount < 5000:  # 只关注有意义的金额
            continue
        
        for j, r_row in receiver_in.iterrows():
            if j in used_receiver:
                continue
            
            r_time = r_row[col_time]
            r_amount = r_row[col_amount]
            
            # 时间窗口：行贿人取款时间 <= 受贿人存款时间 <= 取款时间+N小时
            time_diff = (r_time - b_time).total_seconds() / 3600
            if time_diff < 0 or time_diff > time_window_hours:
                continue
            
            # 金额匹配
            amount_diff = abs(b_amount - r_amount) / b_amount
            if amount_diff > amount_tolerance:
                continue
            
            collisions.append({
                '行贿人取款时间': b_time,
                '取款金额': b_amount,
                '受贿人存款时间': r_time,
                '存款金额': r_amount,
                '时间间隔(小时)': f'{time_diff:.1f}',
                '金额差异': f'{amount_diff*100:.1f}%',
                '行贿人摘要': b_row.get(COL['memo'], ''),
                '受贿人摘要': r_row.get(COL['memo'], ''),
            })
            used_receiver.add(j)
            break
    
    if collisions:
        collision_df = pd.DataFrame(collisions)
        total_collision_amount = sum(c['取款金额'] for c in collisions)
        print(f"\n=== 存取款碰撞: {len(collisions)}对匹配 ===")
        print(f"碰撞总金额: {total_collision_amount:,.2f}")
        print(collision_df.to_string(index=False))
        return collision_df
    else:
        print("\n未发现存取款碰撞配对")
        return pd.DataFrame()

# 使用示例（需指定行贿人和受贿人的数据）
# collision_result = deposit_withdrawal_collision(
#     df_briber, df_receiver, COL['time'], COL['amount'], COL['direction'],
#     DIR_OUT, DIR_IN, time_window_hours=48, amount_tolerance=0.1
# )
```

### 第三方中转检测

```python
def detect_third_party_relay(df_all_accounts, briber_account, receiver_account,
                              col_self, col_opp, col_time, col_amount, col_direction,
                              dir_in, dir_out, time_window_hours=72):
    """
    检测行贿人→第三方→受贿人的中转路径。
    df_all_accounts: 包含所有涉案账户的交易明细
    """
    # 行贿人的出账对手
    briber_outs = df_all_accounts[
        (df_all_accounts[col_self] == briber_account) &
        (df_all_accounts[col_direction] == dir_out)
    ]
    briber_targets = set(briber_outs[col_opp].dropna().unique())
    
    # 受贿人的进账来源
    receiver_ins = df_all_accounts[
        (df_all_accounts[col_self] == receiver_account) &
        (df_all_accounts[col_direction] == dir_in)
    ]
    receiver_sources = set(receiver_ins[col_opp].dropna().unique())
    
    # 交集即为可能的第三方中转
    potential_relays = briber_targets & receiver_sources
    potential_relays -= {briber_account, receiver_account}
    
    relay_paths = []
    for relay_acct in potential_relays:
        # 行贿人→第三方
        leg1 = briber_outs[briber_outs[col_opp] == relay_acct]
        # 第三方→受贿人
        leg2 = receiver_ins[receiver_ins[col_opp] == relay_acct]
        
        for _, l1 in leg1.iterrows():
            for _, l2 in leg2.iterrows():
                time_diff = (l2[col_time] - l1[col_time]).total_seconds() / 3600
                if 0 < time_diff <= time_window_hours:
                    amount_match = abs(l1[col_amount] - l2[col_amount]) / l1[col_amount] < 0.15
                    relay_name = df_all_accounts[
                        df_all_accounts[col_opp] == relay_acct
                    ][COL['opp_name']].iloc[0] if len(df_all_accounts[df_all_accounts[col_opp] == relay_acct]) > 0 else '未知'
                    
                    relay_paths.append({
                        '第三方账户': relay_acct,
                        '第三方户名': relay_name,
                        '行贿人转出时间': l1[col_time],
                        '转出金额': l1[col_amount],
                        '受贿人收到时间': l2[col_time],
                        '收到金额': l2[col_amount],
                        '时间间隔(小时)': f'{time_diff:.1f}',
                        '金额匹配': '是' if amount_match else '否'
                    })
    
    if relay_paths:
        print(f"\n=== 第三方中转检测: {len(relay_paths)}条路径 ===")
        for r in relay_paths:
            print(f"  行贿人→{r['第三方户名']}({r['第三方账户']})→受贿人: "
                  f"转出{r['转出金额']:,.2f}→收到{r['收到金额']:,.2f}, 间隔{r['时间间隔(小时)']}小时")
    
    return relay_paths
```

---

## 7. 串通投标资金分析（阶段C4）

### 保证金逆向溯源

```python
def trace_deposit_source(df_companies, col_self, col_opp, col_time, col_amount, 
                          col_direction, col_memo, dir_in, deposit_keywords=None):
    """
    逆向追查投标保证金来源。
    df_companies: 各投标公司账户的交易明细（合并后）
    """
    if deposit_keywords is None:
        deposit_keywords = r'保证金|投标|押金|履约'
    
    import re
    
    # 筛选保证金相关的收入交易
    deposits = df_companies[
        (df_companies[col_direction] == dir_in) &
        (df_companies[col_memo].astype(str).str.contains(deposit_keywords, na=False))
    ].copy()
    
    if len(deposits) == 0:
        # 如果摘要中没有关键词，尝试按金额和时间特征筛选
        print("摘要中未找到保证金关键词，尝试按金额特征筛选...")
        deposits = df_companies[
            (df_companies[col_direction] == dir_in) &
            (df_companies[col_amount] % 10000 == 0) &
            (df_companies[col_amount] >= 50000)
        ].copy()
    
    print(f"\n=== 保证金溯源分析 ===")
    print(f"疑似保证金交易: {len(deposits)}笔")
    
    # 按来源账户汇总
    source_summary = deposits.groupby([col_opp]).agg(
        companies=(col_self, lambda x: list(x.unique())),
        company_count=(col_self, 'nunique'),
        total_amount=(col_amount, 'sum'),
        txn_count=(col_amount, 'count')
    ).sort_values('company_count', ascending=False)
    
    # 同源（为>=2家公司提供保证金的来源）
    same_source = source_summary[source_summary['company_count'] >= 2]
    
    if len(same_source) > 0:
        print(f"\n[警告] 发现{len(same_source)}个同源资金来源（为>=2家公司提供保证金）:")
        for source, row in same_source.iterrows():
            print(f"  来源账户 {source}: "
                  f"涉及{row['company_count']}家公司 {row['companies']}, "
                  f"合计{row['total_amount']:,.2f}元")
    
    return deposits, same_source
```

### 工程款追踪

```python
def trace_project_funds(df, col_self, col_opp, col_opp_name, col_time, col_amount,
                         col_direction, col_memo, dir_out):
    """追踪中标公司工程款去向"""
    project_keywords = r'工程[款费]|材料[款费]|施工|结算|进度款|尾款'
    
    import re
    project_outs = df[
        (df[col_direction] == dir_out) &
        (df[col_memo].astype(str).str.contains(project_keywords, na=False) |
         (df[col_amount] >= 100000))
    ]
    
    # 按去向分组
    target_summary = project_outs.groupby([col_opp, col_opp_name]).agg(
        count=(col_amount, 'count'),
        total=(col_amount, 'sum')
    ).sort_values('total', ascending=False)
    
    total_project_out = project_outs[col_amount].sum()
    
    print(f"\n=== 工程款/大额资金去向 ===")
    print(f"合计: {len(project_outs)}笔, {total_project_out:,.2f}元")
    
    for (acct, name), row in target_summary.head(10).iterrows():
        nature = '企业' if is_corporate(name) else '个人'
        ratio = row['total'] / total_project_out * 100
        print(f"  [{nature}] {name}({acct}): {row['count']}笔, "
              f"{row['total']:,.2f}元 ({ratio:.1f}%)")
        
        if nature == '个人' and ratio > 10:
            print(f"    [警告] 大比例工程款流向个人账户")
    
    return target_summary
```

### "同时间、同金额"关联分析

```python
def same_time_amount_detection(df_multi_company, col_self, col_time, col_amount,
                                 time_tolerance_days=3, amount_tolerance=0.05):
    """
    "同时间、同金额、不同公司"筛选规则。
    在多家公司的交易中寻找时间和金额高度相似的交易。
    """
    df_sorted = df_multi_company.sort_values(col_time)
    
    # 按公司分组
    company_txns = {}
    for company, group in df_sorted.groupby(col_self):
        company_txns[company] = group
    
    companies = list(company_txns.keys())
    matches = []
    
    for i, j in combinations(range(len(companies)), 2):
        comp_a, comp_b = companies[i], companies[j]
        txns_a = company_txns[comp_a]
        txns_b = company_txns[comp_b]
        
        for _, row_a in txns_a.iterrows():
            for _, row_b in txns_b.iterrows():
                time_diff = abs((row_a[col_time] - row_b[col_time]).total_seconds()) / 86400
                if time_diff > time_tolerance_days:
                    continue
                
                amount_diff = abs(row_a[col_amount] - row_b[col_amount]) / max(row_a[col_amount], 1)
                if amount_diff > amount_tolerance:
                    continue
                
                matches.append({
                    '公司A': comp_a,
                    '公司A时间': row_a[col_time],
                    '公司A金额': row_a[col_amount],
                    '公司B': comp_b,
                    '公司B时间': row_b[col_time],
                    '公司B金额': row_b[col_amount],
                    '时间差(天)': f'{time_diff:.1f}',
                    '金额差': f'{amount_diff*100:.1f}%'
                })
    
    if matches:
        print(f"\n=== '同时间同金额'关联: {len(matches)}组 ===")
        for m in matches[:20]:
            print(f"  {m['公司A']}({m['公司A时间'].date()}, {m['公司A金额']:,.2f}) ↔ "
                  f"{m['公司B']}({m['公司B时间'].date()}, {m['公司B金额']:,.2f})")
    
    return matches
```

---

## 8. 资金闭环检测（阶段C5）

### 资金回路检测

```python
def detect_fund_loops(df, col_self, col_opp, col_amount, col_time, max_depth=3):
    """
    检测资金闭环（A→B→C→...→A的回路）。
    max_depth: 最大追踪深度
    """
    # 构建交易图
    edges = df.groupby([col_self, col_opp]).agg(
        total=(col_amount, 'sum'),
        count=(col_amount, 'count'),
        first_time=(col_time, 'min'),
        last_time=(col_time, 'max')
    ).reset_index()
    
    graph = defaultdict(list)
    for _, row in edges.iterrows():
        if pd.notna(row[col_self]) and pd.notna(row[col_opp]):
            graph[row[col_self]].append({
                'target': row[col_opp],
                'amount': row['total'],
                'count': row['count']
            })
    
    loops = []
    start_nodes = df[col_self].unique()
    
    for start in start_nodes:
        # BFS/DFS 找回路
        stack = [(start, [start], [])]
        while stack:
            node, path, edge_info = stack.pop()
            if len(path) > max_depth + 1:
                continue
            
            for edge in graph.get(node, []):
                target = edge['target']
                if target == start and len(path) > 2:
                    loops.append({
                        'path': path + [start],
                        'depth': len(path),
                        'min_amount': min(e['amount'] for e in edge_info + [edge])
                    })
                elif target not in path:
                    stack.append((target, path + [target], edge_info + [edge]))
    
    # 去重（同一回路可能从不同节点发现）
    unique_loops = []
    seen = set()
    for loop in loops:
        key = tuple(sorted(loop['path'][:-1]))
        if key not in seen:
            seen.add(key)
            unique_loops.append(loop)
    
    if unique_loops:
        print(f"\n=== 资金回路检测: {len(unique_loops)}条 ===")
        for loop in unique_loops[:10]:
            path_str = ' → '.join(str(n) for n in loop['path'])
            print(f"  {path_str} (深度{loop['depth']}, 最小环节金额{loop['min_amount']:,.2f})")
    
    return unique_loops
```

### 对冲交易检测

```python
def detect_hedge_transactions(df, col_opp_account, col_direction, col_amount, col_time,
                               dir_in, dir_out, amount_tolerance=0.1):
    """检测同一对手间一进一出、金额相近的对冲交易"""
    hedges = []
    
    opp_groups = df.dropna(subset=[col_opp_account]).groupby(col_opp_account)
    
    for acct, group in opp_groups:
        ins = group[group[col_direction] == dir_in].sort_values(col_time)
        outs = group[group[col_direction] == dir_out].sort_values(col_time)
        
        if len(ins) == 0 or len(outs) == 0:
            continue
        
        used_outs = set()
        for _, in_row in ins.iterrows():
            for out_idx, out_row in outs.iterrows():
                if out_idx in used_outs:
                    continue
                
                amt_diff = abs(in_row[col_amount] - out_row[col_amount]) / max(in_row[col_amount], 1)
                if amt_diff > amount_tolerance:
                    continue
                
                time_diff = abs((in_row[col_time] - out_row[col_time]).total_seconds()) / 86400
                
                hedges.append({
                    '对手账户': acct,
                    '对手户名': group[COL['opp_name']].iloc[0],
                    '进账金额': in_row[col_amount],
                    '进账时间': in_row[col_time],
                    '出账金额': out_row[col_amount],
                    '出账时间': out_row[col_time],
                    '金额差异': f'{amt_diff*100:.1f}%',
                    '时间间隔(天)': f'{time_diff:.1f}'
                })
                used_outs.add(out_idx)
                break
    
    hedge_ratio = len(hedges) * 2 / len(df) * 100 if len(df) > 0 else 0
    
    print(f"\n=== 对冲交易检测: {len(hedges)}对 ===")
    print(f"对冲交易涉及笔数占比: {hedge_ratio:.1f}%")
    for h in hedges[:10]:
        print(f"  {h['对手户名']}: 进{h['进账金额']:,.2f}({h['进账时间'].date()}) ↔ "
              f"出{h['出账金额']:,.2f}({h['出账时间'].date()})")
    
    return hedges, hedge_ratio
```

---

## 9. 团伙网络分析（阶段C6）

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

### 基于IP/MAC的设备关联

```python
has_ip = COL.get('ip') and COL['ip'] in df.columns
has_mac = COL.get('mac') and COL['mac'] in df.columns

if has_ip or has_mac:
    out_records = df[df[COL['direction']] == DIR_OUT].copy()
    
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
    
    for acct in df[COL['self_account']].unique():
        uf.find(acct)
    
    gangs = uf.get_groups()
    gangs = {k: v for k, v in gangs.items() if len(v) >= 2}
    
    print(f"\n=== 设备关联团伙: {len(gangs)}个 ===")
    for i, (root, members) in enumerate(sorted(gangs.items(), key=lambda x: -len(x[1])), 1):
        print(f"\n--- 团伙{i} ({len(members)}个账户) ---")
        for m in sorted(members):
            name_rows = df[df[COL['self_account']] == m]
            name = name_rows[COL['self_name']].iloc[0] if len(name_rows) > 0 else '未知'
            nature = '企业' if is_corporate(name) else '个人'
            print(f"  [{nature}] {name}({m})")
```

### 企业关联穿透分析

```python
def corporate_association_analysis(df, col_opp_account, col_opp_name, col_opp_id):
    """
    通过交易数据中的企业户名和证件号识别关联企业群。
    """
    # 提取企业对手
    corp_opps = df[df[col_opp_name].apply(lambda x: is_corporate(str(x)) if pd.notna(x) else False)]
    corp_accounts = corp_opps.drop_duplicates(subset=[col_opp_account])[
        [col_opp_account, col_opp_name]
    ]
    
    print(f"\n=== 企业关联分析 ===")
    print(f"交易中涉及的企业对手: {len(corp_accounts)}家")
    
    # 分析同一自然人控制的多家企业（通过证件号）
    if col_opp_id in df.columns:
        person_companies = defaultdict(set)
        for _, row in corp_opps.dropna(subset=[col_opp_id]).iterrows():
            person_id = str(row[col_opp_id])[:18]  # 标准化证件号
            if len(person_id) >= 15:
                company_name = row[col_opp_name]
                person_companies[person_id].add(company_name)
        
        multi_company_persons = {k: v for k, v in person_companies.items() if len(v) >= 2}
        
        if multi_company_persons:
            print(f"\n同一证件号关联多家企业: {len(multi_company_persons)}人")
            for pid, companies in multi_company_persons.items():
                print(f"  证件号{pid[:6]}******: {len(companies)}家企业")
                for c in companies:
                    print(f"    - {c}")
    
    return corp_accounts

corp_analysis = corporate_association_analysis(
    df, COL['opp_account'], COL['opp_name'], COL['opp_id']
)
```

### 地域聚合分析

```python
if COL.get('opp_id') and COL['opp_id'] in df.columns:
    df_id = df.dropna(subset=[COL['opp_account'], COL['opp_id']])[
        [COL['opp_account'], COL['opp_name'], COL['opp_id']]
    ].drop_duplicates(subset=[COL['opp_account']])
    
    df_id['id_prefix6'] = df_id[COL['opp_id']].astype(str).str[:6]
    df_id = df_id[df_id['id_prefix6'].str.match(r'^\d{6}', na=False)]
    
    region_groups = df_id.groupby('id_prefix6').agg(
        count=(COL['opp_account'], 'count'),
        names=(COL['opp_name'], list)
    )
    region_gangs = region_groups[region_groups['count'] >= 2]
    
    print(f"\n=== 地域聚合 ===")
    print(f"有证件号的对手: {len(df_id)}个")
    print(f"同地域组（>=2人）: {len(region_gangs)}组")
    
    for prefix, row in region_gangs.sort_values('count', ascending=False).head(10).iterrows():
        print(f"  地区代码{prefix}: {row['count']}人")
```

---

## 10. 可疑指标量化（阶段D）

### 通用指标计算

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

# 3. 对手集中度
if len(in_top) >= 3:
    results['来源集中度TOP3'] = in_top['total'].head(3).sum() / in_amount
else:
    results['来源集中度TOP3'] = in_top['total'].sum() / in_amount if in_amount > 0 else None

if len(out_top) >= 3:
    results['去向集中度TOP3'] = out_top['total'].head(3).sum() / out_amount
else:
    results['去向集中度TOP3'] = out_top['total'].sum() / out_amount if out_amount > 0 else None

# 4. 同名账户数
SELF_NAME = df[COL['self_name']].mode()[0] if COL['self_name'] in df.columns else ''
results['同名账户数'] = df[df[COL['opp_name']] == SELF_NAME][COL['opp_account']].nunique()

# 5. 整额交易占比
round_10k = df[df[COL['amount']] % 10000 == 0]
results['整万交易占比'] = len(round_10k) / total_count if total_count > 0 else 0

# 6. 快进快出
df['date'] = df[COL['time']].dt.date
daily_in = df_in.groupby(df_in[COL['time']].dt.date)[COL['amount']].sum()
daily_out = df_out.groupby(df_out[COL['time']].dt.date)[COL['amount']].sum()
daily = pd.DataFrame({'in': daily_in, 'out': daily_out}).fillna(0)
quick_days = ((daily['in'] > 50000) & (daily['out'] > 50000)).sum()
total_days = df['date'].nunique()
results['快进快出天数占比'] = quick_days / total_days if total_days > 0 else 0

# 7. 资金周转率
if COL['balance'] in df.columns:
    avg_balance = pd.to_numeric(df[COL['balance']], errors='coerce').mean()
    results['资金周转率'] = total_amount / avg_balance if avg_balance > 0 else None
else:
    results['资金周转率'] = None

# 8. 对公转对私比例
out_personal = df_out[df_out[COL['opp_name']].apply(
    lambda x: not is_corporate(str(x)) if pd.notna(x) else False
)][COL['amount']].sum()
results['对公转对私比例'] = out_personal / out_amount if out_amount > 0 else 0

# 9. 非工作时间交易
df['hour'] = df[COL['time']].dt.hour
non_work = df[(df['hour'] >= 22) | (df['hour'] < 8)]
results['非工作时间交易占比'] = len(non_work) / total_count if total_count > 0 else 0
```

### 阈值判定

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
    '对公转对私比例':    {'normal': 0.30, 'abnormal': 0.50, 'high': 0.70, 'direction': 'higher'},
    '非工作时间交易占比': {'normal': 0.05, 'abnormal': 0.15, 'high': 0.30, 'direction': 'higher'},
}

print("\n=== 可疑指标判定 ===")
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

    display = f"{value*100:.2f}%" if isinstance(value, float) and value < 1 else f"{value}"
    marker = '⚠⚠' if level == '高度异常' else ('⚠' if level == '异常' else '✓')
    print(f"  [{marker}] {name}: {display} → {level}")
```

---

## 11. 多层递归资金穿透分析

```python
def deep_fund_penetration(df, start_accounts, col_self, col_opp, col_opp_name,
                           col_amount, col_direction, dir_out, top_n=5):
    """
    多层递归资金穿透分析，不受固定层数限制，穿透到数据范围内的最底层。
    每层取交易金额TOP-N对手继续向下穿透。

    参数：
        df: 完整交易明细（需包含所有涉案账户的交易记录）
        start_accounts: 起始账户列表
        col_self: 交易方账号列名
        col_opp: 对手账号列名
        col_opp_name: 对手户名列名
        col_amount: 交易金额列名
        col_direction: 收付标志列名
        dir_out: 出账标志值
        top_n: 每层取TOP-N对手继续穿透（默认5）
    """
    # 获取数据中所有可分析的账户集合
    all_accounts = set(df[col_self].dropna().unique())

    # 判断是否为终端交易（现金取款、消费等，对手不在账户体系中）
    terminal_keywords = ['取现', '取款', 'ATM', '消费', '缴费', '还款',
                         '贷款', '理财', '购房', '购车', '税款']

    def is_terminal(row):
        """判断是否为终端交易（资金最终去向已明确）"""
        if pd.isna(row.get(col_opp)):
            return True, '无对手信息（现金存取）'
        opp_acct = str(row[col_opp]).strip()
        if not opp_acct or opp_acct == 'nan':
            return True, '无对手信息（现金存取）'
        return False, ''

    def classify_account_role(acct_data):
        """判断账户角色：过渡账户（即进即出）或终端账户（资金沉淀）"""
        in_amt = acct_data[acct_data[col_direction] == DIR_IN][col_amount].sum() if DIR_IN in acct_data[col_direction].values else 0
        out_amt = acct_data[acct_data[col_direction] == dir_out][col_amount].sum()
        if in_amt + out_amt == 0:
            return '未知'
        diff_ratio = abs(in_amt - out_amt) / max(in_amt, out_amt)
        if diff_ratio <= 0.15:
            return '过渡账户（即进即出）'
        else:
            return '终端账户（资金沉淀）'

    # 穿透结果存储
    all_paths = []      # 所有穿透路径
    visited = set()     # 已穿透的账户（防止循环）

    def penetrate_recursive(current_acct, current_path, path_amounts, depth):
        """
        递归穿透函数
        current_acct: 当前穿透的账户
        current_path: 当前路径上的账户列表
        path_amounts: 当前路径上每层的交易金额
        depth: 当前深度
        """
        # 循环检测
        if current_acct in visited:
            all_paths.append({
                'path': current_path + [current_acct],
                'amounts': path_amounts,
                'depth': depth,
                'terminate_reason': '循环穿透（账户重复出现）'
            })
            return

        visited.add(current_acct)

        # 获取当前账户的支出数据
        acct_data = df[df[col_self] == current_acct]
        out_data = acct_data[acct_data[col_direction] == dir_out].copy()

        if len(out_data) == 0:
            all_paths.append({
                'path': current_path + [current_acct],
                'amounts': path_amounts,
                'depth': depth,
                'terminate_reason': '无支出交易'
            })
            visited.discard(current_acct)
            return

        # 按对手汇总支出，取TOP-N
        opp_summary = out_data.groupby([col_opp, col_opp_name]).agg(
            total_amount=(col_amount, 'sum'),
            count=(col_amount, 'count'),
            first_time=(COL['time'], 'min'),
            last_time=(COL['time'], 'max')
        ).sort_values('total_amount', ascending=False)

        top_opps = opp_summary.head(top_n)

        has_continued = False
        for (opp_acct, opp_name), row in top_opps.iterrows():
            # 检查对手是否为终端交易
            terminal, reason = is_terminal(row)

            if terminal:
                all_paths.append({
                    'path': current_path + [f'{opp_name}({opp_acct})'],
                    'amounts': path_amounts + [row['total_amount']],
                    'depth': depth + 1,
                    'terminate_reason': reason
                })
            elif str(opp_acct) not in all_accounts:
                # 对手账户不在数据范围内
                all_paths.append({
                    'path': current_path + [f'{opp_name}({opp_acct})'],
                    'amounts': path_amounts + [row['total_amount']],
                    'depth': depth + 1,
                    'terminate_reason': '不在数据范围内'
                })
            else:
                # 对手在数据范围内，继续穿透
                has_continued = True
                penetrate_recursive(
                    str(opp_acct),
                    current_path + [f'{opp_name}({opp_acct})'],
                    path_amounts + [row['total_amount']],
                    depth + 1
                )

        # 如果TOP-N全部为终端或不在数据范围内，且没有继续穿透的分支
        if not has_continued:
            # 把当前路径也记录下来（包含所有TOP-N的分支已在上面的循环中记录）
            pass

        visited.discard(current_acct)

    # 对每个起始账户执行穿透
    for start_acct in start_accounts:
        start_name = df[df[col_self] == start_acct][COL['self_name']].iloc[0] if COL['self_name'] in df.columns else '未知'
        visited.clear()
        penetrate_recursive(
            start_acct,
            [f'{start_name}({start_acct})'],
            [],
            0
        )

    return all_paths


# === 使用示例 ===

# 指定需要穿透的起始账户（如涉案公司账户）
penetration_targets = ['公司账号A']  # 替换为实际账号

print("=== 多层资金穿透分析 ===")
print(f"起始账户: {penetration_targets}")
print(f"穿透规则: 每层取TOP5对手继续穿透，穿透到数据范围内最底层\n")

paths = deep_fund_penetration(
    df, penetration_targets,
    COL['self_account'], COL['opp_account'], COL['opp_name'],
    COL['amount'], COL['direction'], DIR_OUT, top_n=5
)

# 输出穿透链路
max_depth = max(p['depth'] for p in paths) if paths else 0
print(f"共发现 {len(paths)} 条穿透链路，最大穿透深度 {max_depth} 层\n")

for i, path_info in enumerate(paths, 1):
    print(f"--- 链路{i}（穿透{path_info['depth']}层，终止: {path_info['terminate_reason']}）---")
    for j, (acct, amt) in enumerate(zip(path_info['path'], path_info['amounts'])):
        indent = "  " * j
        amt_str = f" 金额: {amt:,.2f}" if j > 0 else ""
        print(f"{indent}{'└─' if j > 0 else ''}→ {acct}{amt_str}")
    print()
```

### 公司→个人穿透专项分析

```python
def company_to_person_penetration(df, company_accounts, col_self, col_opp,
                                   col_opp_name, col_amount, col_direction,
                                   dir_out, min_amount=50000, top_n=5):
    """
    专门追踪公司账户转出到个人账户后的资金穿透。
    对每笔大额对公→对私转出，独立进行穿透追踪。
    """
    CORP_KEYWORDS = ['公司', '有限', '集团', '企业', '事务所', '合伙', '工厂', '厂']
    all_accounts = set(df[col_self].dropna().unique())

    results = []

    for comp_acct in company_accounts:
        comp_data = df[(df[col_self] == comp_acct) & (df[col_direction] == dir_out)]

        # 筛选对公→对私的大额转出
        personal_transfers = comp_data[
            comp_data[col_opp_name].apply(
                lambda x: not any(kw in str(x) for kw in CORP_KEYWORDS) if pd.notna(x) else False
            ) & (comp_data[col_amount] >= min_amount)
        ].sort_values(col_amount, ascending=False)

        if len(personal_transfers) == 0:
            print(f"公司账户 {comp_acct} 无大额对私转出记录")
            continue

        print(f"\n=== {comp_acct} 对私大额转出穿透 ===")
        print(f"大额对私转出（>={min_amount:,.0f}元）: {len(personal_transfers)}笔, "
              f"合计 {personal_transfers[col_amount].sum():,.2f}元\n")

        # 对每笔大额转出进行独立穿透
        for idx, transfer in personal_transfers.iterrows():
            person_acct = transfer[col_opp]
            person_name = transfer[col_opp_name]
            transfer_amount = transfer[col_amount]
            transfer_time = transfer[COL['time']]

            # 穿透该个人账户的资金去向
            paths = deep_fund_penetration(
                df, [str(person_acct)],
                col_self, col_opp, col_opp_name,
                col_amount, col_direction, dir_out, top_n=top_n
            )

            max_pen_depth = max(p['depth'] for p in paths) if paths else 0

            print(f"  [{transfer_time.date()}] {comp_acct} → {person_name}({person_acct}) "
                  f"金额: {transfer_amount:,.2f}元 → 穿透{max_pen_depth}层")

            for path_info in paths:
                chain = ' → '.join(path_info['path'][1:])  # 跳过起始账户（已在上面显示）
                final_amt = path_info['amounts'][-1] if path_info['amounts'] else 0
                print(f"    链路: {chain} | 终止: {path_info['terminate_reason']}")

            results.append({
                '转出时间': transfer_time,
                '转出金额': transfer_amount,
                '收款人': person_name,
                '收款账号': person_acct,
                '穿透层数': max_pen_depth,
                '穿透链路数': len(paths),
                '链路详情': paths
            })

    return results


# === 使用示例 ===
# 指定需要分析的公司账户
company_accounts = ['公司账号A']  # 替换为实际公司账号

corp_penetration = company_to_person_penetration(
    df, company_accounts,
    COL['self_account'], COL['opp_account'], COL['opp_name'],
    COL['amount'], COL['direction'], DIR_OUT, min_amount=50000, top_n=5
)
```

---

## 12. 报告生成辅助

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

### Word 报告生成

```python
from docx import Document
from docx.shared import Pt, Cm, RGBColor
from docx.enum.text import WD_PARAGRAPH_ALIGNMENT

def create_trade_report():
    doc = Document()
    
    title = doc.add_heading('商贸类经济犯罪资金分析报告', level=0)
    title.alignment = WD_PARAGRAPH_ALIGNMENT.CENTER
    
    doc.add_heading('一、基本情况', level=1)
    doc.add_heading('（一）案件背景', level=2)
    doc.add_paragraph('此处填写案件背景...')
    
    doc.add_heading('（二）账户基本信息', level=2)
    
    table = doc.add_table(rows=1, cols=5)
    table.style = 'Table Grid'
    headers = table.rows[0].cells
    for i, h in enumerate(['项目', '笔数', '金额', '占比', '对手数']):
        headers[i].text = h
    
    row = table.add_row().cells
    row[0].text = '收入'
    row[1].text = f'{in_count}'
    row[2].text = f'{in_amount:,.2f}'
    row[3].text = f'{in_amount/total_amount*100:.1f}%'
    row[4].text = f'{df_in[COL["opp_account"]].dropna().nunique()}'
    
    doc.save('商贸犯罪资金分析报告.docx')
    print("报告已生成: 商贸犯罪资金分析报告.docx")

# create_trade_report()
```

### f-string 防错规范

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
