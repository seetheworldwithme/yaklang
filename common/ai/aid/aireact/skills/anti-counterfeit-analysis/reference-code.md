# 参考代码片段

按分析维度组织的 Python 代码片段。**不是可以直接运行的完整脚本**，而是供模型根据实际数据格式组合和调整的参考。

使用前请先确认：
1. 实际的列名（用 `df.columns.tolist()` 检查）
2. 数据类型（用 `df.dtypes` 检查）
3. 数据格式（时间格式、编码等）

---

## 1. 数据读取与探查

```python
import pandas as pd
import numpy as np
from collections import defaultdict
import re

# --- 识别并读取数据文件 ---
import glob

files = glob.glob('*.xlsx') + glob.glob('*.xls') + glob.glob('*.csv')
file_map = {}  # type -> filepath

for f in files:
    if f.endswith('.csv'):
        df_tmp = pd.read_csv(f, nrows=1, encoding='utf-8', errors='ignore')
    else:
        df_tmp = pd.read_excel(f, nrows=1)
    cols = set(df_tmp.columns.tolist())

    # 识别文件类型
    if '网盘UID' in cols or '文件名' in cols:
        file_map['网盘数据'] = f
    elif '购买人' in cols or '商品名称' in cols or '收货地址' in cols:
        file_map['物流记录'] = f
    elif '主叫号码' in cols or '被叫号码' in cols:
        file_map['通讯记录'] = f
    elif '交易卡号' in cols or '交易时间' in cols:
        file_map['交易明细'] = f
    elif '姓名' in cols and ('证件号' in cols or '手机号' in cols):
        file_map['人员信息'] = f

print("识别到的数据文件:", file_map)

# --- 读取各类数据 ---
df_person = None
df_netdisk = None
df_logistics = None
df_comm = None
df_trans = None

if '人员信息' in file_map:
    df_person = pd.read_excel(file_map['人员信息']) if file_map['人员信息'].endswith(('.xlsx', '.xls')) else pd.read_csv(file_map['人员信息'])
    print(f"\n=== 人员信息 ===")
    print(f"形状: {df_person.shape}")
    print(f"列名: {df_person.columns.tolist()}")

if '网盘数据' in file_map:
    df_netdisk = pd.read_excel(file_map['网盘数据']) if file_map['网盘数据'].endswith(('.xlsx', '.xls')) else pd.read_csv(file_map['网盘数据'])
    print(f"\n=== 网盘数据 ===")
    print(f"形状: {df_netdisk.shape}")
    print(f"列名: {df_netdisk.columns.tolist()}")

if '物流记录' in file_map:
    df_logistics = pd.read_excel(file_map['物流记录']) if file_map['物流记录'].endswith(('.xlsx', '.xls')) else pd.read_csv(file_map['物流记录'])
    print(f"\n=== 物流记录 ===")
    print(f"形状: {df_logistics.shape}")
    print(f"列名: {df_logistics.columns.tolist()}")

if '通讯记录' in file_map:
    df_comm = pd.read_excel(file_map['通讯记录']) if file_map['通讯记录'].endswith(('.xlsx', '.xls')) else pd.read_csv(file_map['通讯记录'])
    print(f"\n=== 通讯记录 ===")
    print(f"形状: {df_comm.shape}")
    print(f"列名: {df_comm.columns.tolist()}")

if '交易明细' in file_map:
    df_trans = pd.read_excel(file_map['交易明细']) if file_map['交易明细'].endswith(('.xlsx', '.xls')) else pd.read_csv(file_map['交易明细'])
    print(f"\n=== 交易明细 ===")
    print(f"形状: {df_trans.shape}")
    print(f"列名: {df_trans.columns.tolist()}")
```

---

## 2. 假币电子模板识别

### 网盘文件暗语识别

```python
# 假币暗语词典
FAKE_MONEY_KEYWORDS = {
    '10元': ['蓝', '蓝天', '一帆风顺', '10元模板', '十元模板'],
    '20元': ['黄', '黄鱼', 'Y', '20元模板', '二十元模板'],
    '50元': ['青', '青蛙', '绿', '50元模板', '五十元模板'],
    '100元': ['红', '红牛', '风生水起', '九五至尊', '100元模板', '百元模板', '一百元模板'],
}

def detect_fake_money_template(filename):
    """检测文件名是否为假币模板"""
    filename_lower = filename.lower()

    # 检查是否为psd文件
    if not filename_lower.endswith('.psd'):
        return None

    # 检查暗语
    for denomination, keywords in FAKE_MONEY_KEYWORDS.items():
        for keyword in keywords:
            if keyword in filename:
                return {
                    'filename': filename,
                    'denomination': denomination,
                    'matched_keyword': keyword,
                    'is_suspicious': True
                }

    # 检查是否直接包含"模板"字样
    if '模板' in filename:
        return {
            'filename': filename,
            'denomination': '未知',
            'matched_keyword': '模板',
            'is_suspicious': True
        }

    return None

# 扫描网盘文件
if df_netdisk is not None and '文件名' in df_netdisk.columns:
    detected_templates = []
    for filename in df_netdisk['文件名'].dropna().unique():
        result = detect_fake_money_template(str(filename))
        if result:
            detected_templates.append(result)

    print(f"\n=== 检测到 {len(detected_templates)} 个疑似假币模板文件 ===")
    for t in detected_templates:
        print(f"  {t['filename']} → 面额: {t['denomination']} (匹配: {t['matched_keyword']})")

    # 统计各面额数量
    denom_counts = {}
    for t in detected_templates:
        d = t['denomination']
        denom_counts[d] = denom_counts.get(d, 0) + 1
    print(f"\n按面额统计: {denom_counts}")
```

### 网盘关联分析

```python
# 人员与网盘关联
if df_person is not None and df_netdisk is not None:
    # 列名适配
    person_phone_col = '手机号' if '手机号' in df_person.columns else '手机号码'
    person_email_col = '电子邮箱' if '电子邮箱' in df_person.columns else '邮箱'
    netdisk_phone_col = '绑定手机' if '绑定手机' in df_netdisk.columns else '手机号'
    netdisk_email_col = '绑定邮箱' if '绑定邮箱' in df_netdisk.columns else '邮箱'

    # 手机号匹配
    phone_matches = []
    if person_phone_col in df_person.columns and netdisk_phone_col in df_netdisk.columns:
        person_phones = set(df_person[person_phone_col].dropna().astype(str).str.strip())
        netdisk_phones = set(df_netdisk[netdisk_phone_col].dropna().astype(str).str.strip())
        matched_phones = person_phones & netdisk_phones

        for phone in matched_phones:
            person_info = df_person[df_person[person_phone_col].astype(str).str.strip() == phone].iloc[0].to_dict()
            netdisk_info = df_netdisk[df_netdisk[netdisk_phone_col].astype(str).str.strip() == phone]
            phone_matches.append({
                '匹配方式': '手机号',
                '手机号': phone,
                '姓名': person_info.get('姓名', '未知'),
                '网盘UID': netdisk_info['网盘UID'].iloc[0] if '网盘UID' in netdisk_info.columns else '未知',
                '文件数': len(netdisk_info)
            })

    # 邮箱匹配
    email_matches = []
    if person_email_col in df_person.columns and netdisk_email_col in df_netdisk.columns:
        person_emails = set(df_person[person_email_col].dropna().astype(str).str.strip().str.lower())
        netdisk_emails = set(df_netdisk[netdisk_email_col].dropna().astype(str).str.strip().str.lower())
        matched_emails = person_emails & netdisk_emails

        for email in matched_emails:
            person_info = df_person[df_person[person_email_col].astype(str).str.strip().str.lower() == email].iloc[0].to_dict()
            netdisk_info = df_netdisk[df_netdisk[netdisk_email_col].astype(str).str.strip().str.lower() == email]
            email_matches.append({
                '匹配方式': '邮箱',
                '邮箱': email,
                '姓名': person_info.get('姓名', '未知'),
                '网盘UID': netdisk_info['网盘UID'].iloc[0] if '网盘UID' in netdisk_info.columns else '未知',
                '文件数': len(netdisk_info)
            })

    print(f"\n=== 网盘关联分析 ===")
    print(f"手机号匹配: {len(phone_matches)}人")
    for m in phone_matches:
        print(f"  {m['姓名']} - {m['手机号']} → 网盘UID: {m['网盘UID']}, 文件数: {m['文件数']}")

    print(f"邮箱匹配: {len(email_matches)}人")
    for m in email_matches:
        print(f"  {m['姓名']} - {m['邮箱']} → 网盘UID: {m['网盘UID']}, 文件数: {m['文件数']}")
```

---

## 3. 制假材料物流分析

### 可疑材料识别

```python
# 可疑材料清单
SUSPICIOUS_MATERIALS = {
    '证券纸': ['证券纸', '特种纸', '80g纸', '90g纸'],
    '6色打印机': ['6色打印机', 'R330', '爱普森', 'epson', '彩色打印机'],
    '水性油墨': ['水性油墨', '油墨', '墨水', 'ink'],
    '扫描仪': ['扫描仪', 'scanner'],
    '烫金机': ['烫金机', '烫金'],
    '烫金线': ['烫金线'],
    '金粉': ['金粉', '金色粉'],
    '丝网': ['丝网', '印刷网'],
    '切纸机': ['切纸机', '裁纸机'],
    '清零软件': ['清零', 'reset', '计数器'],
    '图像处理软件': ['photoshop', 'ps软件', '图像处理'],
}

def detect_suspicious_materials(df_logistics):
    """识别可疑制假材料"""
    if df_logistics is None or '商品名称' not in df_logistics.columns:
        return pd.DataFrame()

    results = []
    for idx, row in df_logistics.iterrows():
        product = str(row.get('商品名称', '')).lower()
        for material_type, keywords in SUSPICIOUS_MATERIALS.items():
            for keyword in keywords:
                if keyword.lower() in product:
                    results.append({
                        '购买人': row.get('购买人', '未知'),
                        '手机号': row.get('手机号', row.get('购买人手机', '未知')),
                        '材料类型': material_type,
                        '商品名称': row.get('商品名称', '未知'),
                        '数量': row.get('数量', 1),
                        '购买时间': row.get('购买时间', '未知'),
                        '收货地址': row.get('收货地址', '未知'),
                        '匹配关键词': keyword
                    })
                    break

    return pd.DataFrame(results)

# 执行检测
df_suspicious = detect_suspicious_materials(df_logistics)

if len(df_suspicious) > 0:
    print(f"\n=== 检测到 {len(df_suspicious)} 条可疑材料购买记录 ===")

    # 按购买人统计
    buyer_stats = df_suspicious.groupby('购买人').agg(
        材料种类数=('材料类型', 'nunique'),
        购买次数=('材料类型', 'count'),
        涉及材料类型=('材料类型', lambda x: ', '.join(set(x)))
    ).sort_values('材料种类数', ascending=False)

    print("\n按购买人统计:")
    for buyer, row in buyer_stats.iterrows():
        suspicious_level = '高度可疑' if row['材料种类数'] >= 4 else ('可疑' if row['材料种类数'] >= 2 else '需关注')
        print(f"  {buyer}: {row['材料种类数']}种材料 ({row['涉及材料类型']}) → {suspicious_level}")
```

### 购买序列分析

```python
def analyze_purchase_sequence(df_suspicious, buyer_name):
    """分析某人的购买序列，判断窝点阶段"""
    if df_suspicious is None or len(df_suspicious) == 0:
        return None

    buyer_data = df_suspicious[df_suspicious['购买人'] == buyer_name].copy()
    if len(buyer_data) == 0:
        return None

    # 转换时间
    buyer_data['购买时间'] = pd.to_datetime(buyer_data['购买时间'], errors='coerce')
    buyer_data = buyer_data.sort_values('购买时间')

    # 分析阶段
    total_types = buyer_data['材料类型'].nunique()
    total_count = len(buyer_data)

    # 判断是否进入批量生产阶段
    has_printer = buyer_data['材料类型'].str.contains('打印机').any()
    paper_records = buyer_data[buyer_data['材料类型'] == '证券纸']
    paper_count = paper_records['数量'].sum() if len(paper_records) > 0 else 0

    stage = '未知'
    if total_types < 2:
        stage = '初期/观望'
    elif paper_count >= 500 or (has_printer and total_count >= 5):
        stage = '批量生产'
    else:
        stage = '测试/调试'

    return {
        '购买人': buyer_name,
        '材料种类数': total_types,
        '购买总次数': total_count,
        '证券纸数量': paper_count,
        '是否有打印机': has_printer,
        '判断阶段': stage,
        '首次购买时间': buyer_data['购买时间'].min(),
        '最近购买时间': buyer_data['购买时间'].max()
    }

# 执行序列分析
if len(df_suspicious) > 0:
    print("\n=== 购买序列分析（窝点阶段研判）===")
    for buyer in df_suspicious['购买人'].unique():
        result = analyze_purchase_sequence(df_suspicious, buyer)
        if result:
            print(f"\n{result['购买人']}:")
            print(f"  材料种类: {result['材料种类数']}种")
            print(f"  购买次数: {result['购买总次数']}次")
            print(f"  证券纸: {result['证券纸数量']}张")
            print(f"  打印机: {'有' if result['是否有打印机'] else '无'}")
            print(f"  判断阶段: {result['判断阶段']}")
```

### 生产规模估算

```python
def estimate_production_scale(paper_count, printer_count=1, hours_per_day=10):
    """估算假币生产规模"""
    # 爱普森R330打印机效率：约2分钟/张
    minutes_per_sheet = 2
    sheets_per_hour = 60 / minutes_per_sheet  # 30张/小时
    sheets_per_day = sheets_per_hour * hours_per_day * printer_count  # 日产量

    # 估算生产周期
    production_days = paper_count / sheets_per_day if sheets_per_day > 0 else 0

    return {
        '证券纸数量': paper_count,
        '打印机数量': printer_count,
        '单台日产量': int(sheets_per_day / printer_count) if printer_count > 0 else 0,
        '总日产量': int(sheets_per_day),
        '预计生产周期': f"{production_days:.1f}天",
        '估算总产量': paper_count  # 假设1张纸印1张假币
    }

# 示例
example_scale = estimate_production_scale(paper_count=1000, printer_count=2)
print(f"\n=== 生产规模估算示例 ===")
for k, v in example_scale.items():
    print(f"  {k}: {v}")
```

---

## 4. 通讯记录分析

### 深夜通话检测

```python
def analyze_night_calls(df_comm, night_hours=(1, 5)):
    """分析深夜通话情况"""
    if df_comm is None or '通话时间' not in df_comm.columns:
        return None

    df = df_comm.copy()
    df['通话时间'] = pd.to_datetime(df['通话时间'], errors='coerce')
    df['通话小时'] = df['通话时间'].dt.hour

    # 深夜通话
    night_calls = df[(df['通话小时'] >= night_hours[0]) & (df['通话小时'] < night_hours[1])]

    # 按主叫号码统计
    stats = df.groupby('主叫号码').agg(
        总通话次数=('主叫号码', 'count'),
        深夜通话次数=('通话小时', lambda x: ((x >= night_hours[0]) & (x < night_hours[1])).sum())
    )
    stats['深夜通话占比'] = stats['深夜通话次数'] / stats['总通话次数'] * 100
    stats['可疑程度'] = stats['深夜通话占比'].apply(
        lambda x: '高度可疑' if x > 20 else ('可疑' if x > 10 else '正常')
    )

    print(f"\n=== 深夜通话分析（{night_hours[0]}时-{night_hours[1]}时）===")
    suspicious_phones = stats[stats['可疑程度'] != '正常'].sort_values('深夜通话占比', ascending=False)
    for phone, row in suspicious_phones.head(10).iterrows():
        print(f"  {phone}: 深夜通话{row['深夜通话次数']}次/{row['总通话次数']}次 ({row['深夜通话占比']:.1f}%) → {row['可疑程度']}")

    return stats
```

### 异常开关机检测

```python
def detect_abnormal_power_patterns(df_comm, home_location_col='通话地点'):
    """检测异常开关机模式（需要位置信息）"""
    if df_comm is None or home_location_col not in df_comm.columns:
        print("缺少位置信息，无法进行异常开关机分析")
        return None

    df = df_comm.copy()
    df['通话时间'] = pd.to_datetime(df['通话时间'], errors='coerce')
    df['日期'] = df['通话时间'].dt.date

    # 按号码+日期+位置分析
    # 简化版：检测某号码是否在离开常驻位置后没有通话记录
    results = []

    for phone in df['主叫号码'].unique():
        phone_data = df[df['主叫号码'] == phone].sort_values('通话时间')

        # 找出常驻位置（通话次数最多的位置）
        location_counts = phone_data[home_location_col].value_counts()
        if len(location_counts) == 0:
            continue
        home_location = location_counts.index[0]

        # 检测位置变化
        prev_location = None
        for idx, row in phone_data.iterrows():
            curr_location = row[home_location_col]
            if prev_location == home_location and curr_location != home_location:
                # 离开常驻位置
                results.append({
                    '号码': phone,
                    '时间': row['通话时间'],
                    '从位置': home_location,
                    '到位置': curr_location,
                    '模式': '离开常驻位置'
                })
            prev_location = curr_location

    return pd.DataFrame(results)
```

### 高频通讯网络

```python
def build_communication_network(df_comm, threshold=5):
    """构建通讯关系网络"""
    if df_comm is None:
        return None

    # 统计通讯频次
    network = df_comm.groupby(['主叫号码', '被叫号码']).size().reset_index(name='通话次数')
    network = network[network['通话次数'] >= threshold].sort_values('通话次数', ascending=False)

    print(f"\n=== 高频通讯网络（通话>={threshold}次）===")
    print(f"发现 {len(network)} 对高频通讯关系")
    for idx, row in network.head(20).iterrows():
        print(f"  {row['主叫号码']} <-> {row['被叫号码']}: {row['通话次数']}次")

    return network
```

---

## 5. 资金流分析

### 大额现金存入检测

```python
def detect_large_cash_deposits(df_trans, threshold=50000):
    """检测大额现金存入"""
    if df_trans is None:
        return None

    # 列名适配
    cash_col = '现金标志' if '现金标志' in df_trans.columns else None
    direction_col = '收付标志' if '收付标志' in df_trans.columns else '借贷标志'
    amount_col = '交易金额' if '交易金额' in df_trans.columns else '金额'

    df = df_trans.copy()

    # 筛选现金存入
    if cash_col:
        cash_mask = df[cash_col].astype(str).str.contains('现', na=False)
        df_cash = df[cash_mask].copy()
    else:
        # 如果没有现金标志，用存入方向判断
        df_cash = df[df[direction_col].isin(['进', '收', '贷'])].copy()

    # 筛选大额
    df_large = df_cash[df_cash[amount_col] >= threshold].copy()

    if len(df_large) == 0:
        print(f"未发现{threshold/10000:.0f}万以上的大额现金存入")
        return None

    # 按账户统计
    account_col = '交易卡号' if '交易卡号' in df_large.columns else '账号'
    stats = df_large.groupby(account_col).agg(
        大额存入笔数=(amount_col, 'count'),
        大额存入总额=(amount_col, 'sum'),
        单笔最高=(amount_col, 'max'),
        首次存入时间=('交易时间', 'min'),
        最近存入时间=('交易时间', 'max')
    ).sort_values('大额存入总额', ascending=False)

    print(f"\n=== 大额现金存入分析（>={threshold/10000:.0f}万）===")
    for acct, row in stats.iterrows():
        print(f"  {acct}: {row['大额存入笔数']}笔, 合计{row['大额存入总额']:,.0f}元, 单笔最高{row['单笔最高']:,.0f}元")

    return stats
```

---

## 6. 团伙关系分析

### Union-Find 团伙聚类

```python
class UnionFind:
    """并查集，用于将共享设备/地址的人员聚类为团伙"""

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

def cluster_gang_by_address(df_logistics, min_members=2):
    """根据物流收货地址聚类团伙"""
    if df_logistics is None or '收货地址' not in df_logistics.columns:
        return None

    # 提取地址关联（同一地址多人收货）
    address_people = df_logistics.groupby('收货地址')['购买人'].apply(set).reset_index()
    address_people = address_people[address_people['购买人'].apply(len) >= min_members]

    uf = UnionFind()

    # 合并同地址的人
    for idx, row in address_people.iterrows():
        people = list(row['购买人'])
        for i in range(1, len(people)):
            uf.union(people[0], people[i])

    # 确保所有人员都被查找
    for people_set in address_people['购买人']:
        for p in people_set:
            uf.find(p)

    gangs = uf.get_groups()
    gangs = {k: v for k, v in gangs.items() if len(v) >= min_members}

    print(f"\n=== 物流地址团伙聚类 ===")
    print(f"发现 {len(gangs)} 个团伙")
    for i, (root, members) in enumerate(sorted(gangs.items(), key=lambda x: -len(x[1])), 1):
        print(f"\n团伙{i} ({len(members)}人):")
        for m in sorted(members):
            print(f"  - {m}")

    return gangs

def cluster_gang_by_region(df_person, id_col='证件号', min_prefix=6, min_members=2):
    """根据证件号前6位（户籍地）聚类团伙"""
    if df_person is None or id_col not in df_person.columns:
        return None

    df = df_person.copy()
    df['证件前6位'] = df[id_col].astype(str).str[:6]

    # 过滤无效证件号
    df = df[df['证件前6位'].str.match(r'^\d{6}$', na=False)]

    # 按地区分组
    region_groups = df.groupby('证件前6位').agg({
        '姓名': list
    })
    region_groups['人数'] = region_groups['姓名'].apply(len)
    region_groups = region_groups[region_groups['人数'] >= min_members]

    print(f"\n=== 地域团伙聚类（证件号前6位）===")
    for prefix, row in region_groups.sort_values('人数', ascending=False).iterrows():
        names = row['姓名']
        print(f"  地区{prefix}: {len(names)}人 - {', '.join(names[:5])}{'...' if len(names) > 5 else ''}")

    return region_groups
```

---

## 7. 窝点定位辅助

```python
def locate_suspicious_locations(df_logistics, df_comm=None):
    """整合物流地址和通讯位置信息，定位可疑窝点"""
    locations = []

    # 从物流地址提取
    if df_logistics is not None and '收货地址' in df_logistics.columns:
        address_stats = df_logistics.groupby('收货地址').agg(
            购买人数=('购买人', 'nunique'),
            可疑材料种类=('商品名称', 'count'),
            最近购买时间=('购买时间', 'max')
        ).sort_values('购买人数', ascending=False)

        for addr, row in address_stats.head(10).iterrows():
            locations.append({
                '来源': '物流收货地址',
                '地址': addr,
                '关联人数': row['购买人数'],
                '备注': f"购买{row['可疑材料种类']}次"
            })

    print(f"\n=== 可疑地点定位 ===")
    for loc in locations:
        print(f"  [{loc['来源']}] {loc['地址']}")
        print(f"    关联人数: {loc['关联人数']}, {loc['备注']}")

    return locations
```

---

## 8. 可疑指标汇总

```python
def calculate_suspicious_indicators(df_person, df_netdisk, df_logistics, df_comm, df_trans):
    """计算所有可疑指标"""
    results = {}

    # 1. 制假材料购买种类数
    if df_logistics is not None and len(df_logistics) > 0:
        df_suspicious = detect_suspicious_materials(df_logistics)
        if len(df_suspicious) > 0:
            buyer_types = df_suspicious.groupby('购买人')['材料类型'].nunique()
            results['最高材料种类数'] = buyer_types.max()
            results['购买多种材料人数'] = (buyer_types >= 2).sum()
        else:
            results['最高材料种类数'] = 0
            results['购买多种材料人数'] = 0

    # 2. 网盘模板文件数
    if df_netdisk is not None and '文件名' in df_netdisk.columns:
        template_count = 0
        for filename in df_netdisk['文件名'].dropna().unique():
            if detect_fake_money_template(str(filename)):
                template_count += 1
        results['假币模板文件数'] = template_count

    # 3. 深夜通话占比
    if df_comm is not None and '通话时间' in df_comm.columns:
        df_comm['通话时间'] = pd.to_datetime(df_comm['通话时间'], errors='coerce')
        df_comm['通话小时'] = df_comm['通话时间'].dt.hour
        night_calls = ((df_comm['通话小时'] >= 1) & (df_comm['通话小时'] < 5)).sum()
        total_calls = len(df_comm)
        results['深夜通话占比'] = night_calls / total_calls if total_calls > 0 else 0

    # 4. 大额现金存入
    if df_trans is not None:
        large_deposits = detect_large_cash_deposits(df_trans)
        if large_deposits is not None:
            results['大额存入账户数'] = len(large_deposits)
            results['大额存入总金额'] = large_deposits['大额存入总额'].sum()

    # 5. 地域集中度
    if df_person is not None and '证件号' in df_person.columns:
        df_person['证件前6位'] = df_person['证件号'].astype(str).str[:6]
        valid_ids = df_person[df_person['证件前6位'].str.match(r'^\d{6}$', na=False)]
        if len(valid_ids) > 0:
            max_region_count = valid_ids['证件前6位'].value_counts().max()
            results['地域集中度'] = max_region_count / len(valid_ids)

    # 输出结果
    print("\n=== 可疑指标汇总 ===")
    thresholds = {
        '最高材料种类数': {'abnormal': 2, 'high': 4, 'direction': 'higher'},
        '假币模板文件数': {'abnormal': 1, 'high': 5, 'direction': 'higher'},
        '深夜通话占比': {'abnormal': 0.10, 'high': 0.20, 'direction': 'higher'},
        '大额存入账户数': {'abnormal': 1, 'high': 3, 'direction': 'higher'},
        '地域集中度': {'abnormal': 0.40, 'high': 0.60, 'direction': 'higher'},
    }

    for name, value in results.items():
        if value is None:
            continue
        th = thresholds.get(name)
        if th:
            if th['direction'] == 'higher':
                if value >= th['high']:
                    level = '高度异常'
                elif value >= th['abnormal']:
                    level = '异常'
                else:
                    level = '正常'
            else:
                if value <= th['high']:
                    level = '高度异常'
                elif value <= th['abnormal']:
                    level = '异常'
                else:
                    level = '正常'

            if isinstance(value, float) and value < 1:
                display = f"{value*100:.1f}%"
            else:
                display = f"{value}"

            print(f"  [{level}] {name}: {display}")
        else:
            print(f"  {name}: {value}")

    return results
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

# 使用示例
if len(df_suspicious) > 0:
    headers = ['序号', '购买人', '材料类型', '商品名称', '购买时间', '可疑程度']
    rows = []
    for i, (idx, row) in enumerate(df_suspicious.head(10).iterrows(), 1):
        rows.append([i, row['购买人'], row['材料类型'], row['商品名称'], row['购买时间'], '可疑'])

    table_str = to_md_table(headers, rows)
    print("\n=== 可疑材料购买记录表格 ===")
    print(table_str)
```

### Word 报告生成（使用 python-docx）

```python
from docx import Document
from docx.shared import Pt, Cm, RGBColor
from docx.enum.text import WD_PARAGRAPH_ALIGNMENT

def create_anti_counterfeit_report():
    """生成假币犯罪分析报告"""
    doc = Document()

    # 标题
    title = doc.add_heading('假币犯罪分析报告', level=0)
    title.alignment = WD_PARAGRAPH_ALIGNMENT.CENTER

    # 一、案件概述
    doc.add_heading('一、案件概述', level=1)
    doc.add_heading('（一）案件背景', level=2)
    doc.add_paragraph('此处填写案件背景...')

    doc.add_heading('（二）分析目标', level=2)
    doc.add_paragraph('本次分析目标：识别假币制售团伙成员，锁定制假窝点位置...')

    # 二、人员分析
    doc.add_heading('二、人员分析', level=1)
    doc.add_heading('（一）前科人员库', level=2)
    doc.add_paragraph('此处填写前科人员信息...')

    # ... 其他章节

    doc.save('假币犯罪分析报告.docx')
    print("报告已生成: 假币犯罪分析报告.docx")

# create_anti_counterfeit_report()  # 取消注释执行
```

### f-string 防错规范（重要）

生成 Word 报告时，所有包含变量引用的字符串**必须使用 f-string**（在引号前加 `f` 前缀），否则 `{变量名}` 会作为字面量原样输出到文档中。

**错误写法**（变量不会被求值，直接输出 `{buyer}` 这样的文字）：
```python
doc.add_paragraph('购买人{buyer}购买{count}种可疑材料')  # 缺少 f 前缀！
```

**正确写法**（变量会被正确求值并替换为真实数据）：
```python
doc.add_paragraph(f'购买人{buyer}购买{count}种可疑材料')
```

**检查方法**：生成报告后，打开 docx 文件搜索 `{` 字符。如果文档中出现 `{buyer}`、`{count}` 等原始占位符文本，说明有字符串遗漏了 `f` 前缀，需要逐一排查修复。
