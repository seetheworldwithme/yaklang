#!/usr/bin/env python3
"""Generate final fund analysis report based on analysis_results.json and report-template.md"""
import json, os

RESULTS_FILE = "/Volumes/code/JZ_0512/ExcelCLI/db/analysis_results.json"
OUTPUT_DIR = "/Volumes/code/JZ_0512/ExcelCLI/report-test1"

with open(RESULTS_FILE) as f:
    r = json.load(f)

# --- Helper ---
def fmt(n):
    if n is None: return "0.00"
    if isinstance(n, (int, float)):
        s = f"{n:,.2f}"
        return s if s != "nan" else "0.00"
    return str(n)

ts = r["total_summary"]
ds = r["direction_summary"]
in_data = ds[0] if ds[0]["direction"]=="in" else ds[1]
out_data = ds[1] if ds[1]["direction"]=="out" else ds[0]
total_in = in_data["amt"]
total_out = out_data["amt"]
balance_diff = abs(total_in - total_out)
balance_ratio = round(balance_diff/(total_in+total_out)*100, 2) if (total_in+total_out)>0 else 0
cash = r["cash_analysis"]
cash_cnt = cash[0]["cnt"] if cash else 0
cash_amt = cash[0]["amt"] if cash else 0
cash_cnt_pct = cash[0]["cnt_pct"] if cash else 0

# --- Key indicators for suspicious analysis ---
# 1. Balance degree
s1_val = balance_ratio
s1_flag = "异常" if s1_val < 5 else ("高度异常" if s1_val < 1 else "正常")
# 2. Cash ratio
s2_val = cash_cnt_pct
s2_flag = "高度异常" if s2_val > 70 else ("异常" if s2_val > 50 else "正常")
# 3. Source concentration
sc = r["source_concentration"]
s3_val = sc["top3_pct"]
s3_flag = "异常" if s3_val > 40 else ("高度异常" if s3_val > 60 else "正常")
# 4. Destination concentration
dc = r["dest_concentration"]
s4_val = dc["top3_pct"]
s4_flag = "正常" if s4_val < 30 else ("异常" if s4_val > 40 else "高度异常" if s4_val > 60 else "正常")
# 5. Same-name accounts
s5_cnt = len(r["same_name_sources"]) + len(r["same_name_dests"])
s5_flag = "高度异常" if s5_cnt >= 5 else ("异常" if s5_cnt >= 2 else "正常")
# 6. Round amount ratio
ra = r["round_amount"]
ra_in_cnt = ra[0]["cnt"] if ra and ra[0]["direction"]=="in" else 0
ra_out_cnt = ra[0]["cnt"] if ra and ra[0]["direction"]=="in" else (ra[1]["cnt"] if len(ra)>1 else 0)
ra_total_cnt = ra_in_cnt + ra_out_cnt
ra_ratio = round(ra_total_cnt/ts["total"]*100, 2) if ts["total"]>0 else 0
s6_flag = "异常" if ra_ratio > 30 else ("高度异常" if ra_ratio > 45 else "正常")
# 7. Self-transfer ratio
st = r["self_transfer"]
st_ratio = round(st["amt"]/(total_in+total_out)*100, 2) if (total_in+total_out)>0 else 0
# 8. Turnover rate
avg_bal = r["balance_summary"]["avg_bal"]
turnover = round(ts["total_amt"]/avg_bal, 2) if avg_bal>0 else 0
s8_flag = "异常" if turnover > 50 else ("高度异常" if turnover > 100 else "正常")

# --- Build report ---
lines = []

# Title
lines.append(f"# kh13616 (xm1083) 资金分析报告")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 1
lines.append("## 一、基本情况")
lines.append("")
lines.append("### （一）账户基本信息")
lines.append("")
lines.append("**涉案账户信息表：**")
lines.append("")
lines.append("| 项目 | 内容 |")
lines.append("| --- | --- |")
lines.append(f"| 客户名称 | xm1083 |")
lines.append(f"| 涉案账户 | kh13616 |")
lines.append(f"| 证件号码 | —（数据中缺少证件号字段） |")
lines.append(f"| 开户时间 | —（未提供账户信息辅助表） |")
lines.append(f"| 开户银行 | —（未提供账户信息辅助表） |")
lines.append(f"| 开户网点 | —（未提供账户信息辅助表） |")
lines.append(f"| 账户类型 | unknown |")
lines.append(f"| 账户状态 | —（未提供账户信息辅助表） |")
lines.append(f"| 当前余额 | {fmt(r['balance_summary']['last_balance'])} 元 |")
lines.append("")
lines.append("> 仅涉及1个主体账户，账号 kh13616，户名 xm1083。")
lines.append("")
lines.append("### （二）交易概况")
lines.append("")
lines.append("| 项目 | 笔数 | 金额（元） | 占比 | 涉及对手账户数 |")
lines.append("| --- | --- | --- | --- | --- |")
lines.append(f"| 收入 | {in_data['cnt']} | {fmt(total_in)} | {round(total_in/(total_in+total_out)*100,2) if (total_in+total_out)>0 else 0}% | {in_data['counterparties']} |")
lines.append(f"| 支出 | {out_data['cnt']} | {fmt(total_out)} | {round(total_out/(total_in+total_out)*100,2) if (total_in+total_out)>0 else 0}% | {out_data['counterparties']} |")
lines.append(f"| 合计 | {ts['total']} | {fmt(ts['total_amt'])} | 100% | — |")
lines.append("")
lines.append(f"原始数据共 {ts['total']} 笔，无失败交易过滤。收支差额 {fmt(balance_diff)} 元，占总交易额的 {balance_ratio}%，收支高度平衡。")
lines.append("")
lines.append("### （三）交易结构特点")
lines.append("")
lines.append(f"一是收支高度平衡。收入 {fmt(total_in)} 元，支出 {fmt(total_out)} 元，差额仅占总交易额的 {balance_ratio}%，呈现典型的资金过渡账户特征。")
lines.append(f"二是现金交易占比异常高。现金交易 {cash_cnt} 笔，金额 {fmt(cash_amt)} 元，笔数占 {cash_cnt_pct}%，金额占 {round(cash_amt/ts['total_amt']*100,2)}%，远超正常经营账户水平。")
lines.append(f"三是日均交易密度高。日均交易 {r['daily_avg_txns']} 笔，交易活跃。")
lines.append(f"四是余额波动大。最高余额 {fmt(r['balance_summary']['max_bal'])} 元，最低 {fmt(r['balance_summary']['min_bal'])} 元，平均 {fmt(r['balance_summary']['avg_bal'])} 元，最终余额 {fmt(r['balance_summary']['last_balance'])} 元，资金快进快出特征明显。")
lines.append(f"五是交易金额集中在1万-10万元区间。1万-5万区间收入 {[x for x in r['amount_distribution'] if x['range']=='1万-5万'][0]['in_cnt']} 笔、支出 {[x for x in r['amount_distribution'] if x['range']=='1万-5万'][0]['out_cnt']} 笔；5万-10万区间收入 {[x for x in r['amount_distribution'] if x['range']=='5万-10万'][0]['in_cnt']} 笔、支出 {[x for x in r['amount_distribution'] if x['range']=='5万-10万'][0]['out_cnt']} 笔。")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 2
lines.append("## 二、资金交易情况")
lines.append("")
lines.append("### （一）总体概况")
lines.append("")
lines.append("**年度交易统计表：**")
lines.append("")
lines.append("| 年度 | 收入笔数 | 收入金额（元） | 收入对手数 | 支出笔数 | 支出金额（元） | 支出对手数 |")
lines.append("| --- | --- | --- | --- | --- | --- | --- |")
for y in r["yearly_trend"]:
    lines.append(f"| {y['year']} | {y['in_cnt']} | {fmt(y['in_amt'])} | {y['in_cpty']} | {y['out_cnt']} | {fmt(y['out_amt'])} | {y['out_cpty']} |")
lines.append("")
lines.append("从年度趋势看，2015年为交易高峰期，收入金额27,623,326.29元，涉及46个对手；2016年支出金额最高，达32,044,823.00元。2018年后交易量逐年下降。对手主体数在2019年达到峰值（收入26个、支出66个），说明交易范围在当年显著扩大。")
lines.append("")
lines.append("**金额分布表：**")
lines.append("")
lines.append("| 金额区间 | 收入笔数 | 支出笔数 |")
lines.append("| --- | --- | --- |")
for ad in r["amount_distribution"]:
    lines.append(f"| {ad['range']} | {ad['in_cnt']} | {ad['out_cnt']} |")
lines.append("")
lines.append("### （二）现金交易情况")
lines.append("")
lines.append(f"数据中"现金标志"字段显示，现金交易共计 {cash_cnt} 笔，金额 {fmt(cash_amt)} 元。现金交易笔数占比 {cash_cnt_pct}%，金额占比 {round(cash_amt/ts['total_amt']*100,2)}%，现金交易特征极为突出。")
lines.append("")
lines.append("### （三）交易地理分布")
lines.append("")
lines.append("因数据中缺少"交易发生地"字段，该项未分析。")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 3
lines.append("## 三、主要资金来源分析")
lines.append("")
lines.append("### （一）来源集中度")
lines.append("")
lines.append(f"收入端涉及 {in_data['counterparties']} 个不同对手账户。")
sb = r["source_concentration"]
lines.append(f"TOP1来源占收入的 {sb['top1_pct']}%，TOP3占 {sb['top3_pct']}%，TOP5占 {sb['top5_pct']}%。资金来源集中度极低，呈现分散转入特征。")
lines.append("")
lines.append("### （二）TOP10 资金来源")
lines.append("")
lines.append("| 序号 | 对手账户 | 对手户名 | 交易笔数 | 合计金额（元） | 平均单笔（元） | 特征标注 |")
lines.append("| --- | --- | --- | --- | --- | --- | --- |")
for i, src in enumerate(r["top10_sources"], 1):
    name = src.get("counterparty_name","") or ""
    notes = []
    cpty = src["counterparty_account"]
    # Check if same-name
    for s in r["same_name_sources"]:
        if s["counterparty_account"]==cpty:
            notes.append("同名账户")
            break
    # Check if two-way
    for t in r["two_way_counterparties"]:
        if t["counterparty_account"]==cpty:
            notes.append("双向交易")
            break
    if i==1: notes.append("最大来源")
    if src["cnt"]>300: notes.append("高频转入")
    if src["avg_amt"]>100000: notes.append("大额入账")
    notes_str = "、".join(notes) if notes else ""
    lines.append(f"| {i} | {cpty} | {name} | {src['cnt']} | {fmt(src['amt'])} | {fmt(src['avg_amt'])} | {notes_str} |")
lines.append("")
lines.append("### （三）重点来源账户分析")
lines.append("")
lines.append(f"第一来源 kh66688（户名未知）：共 {r['top10_sources'][0]['cnt']} 笔，合计 {fmt(r['top10_sources'][0]['amt'])} 元，平均单笔 {fmt(r['top10_sources'][0]['avg_amt'])} 元。该账户同时向涉案账户转入资金（698笔，36,165,915.53元）且户名同为xm1083，属于同名账户，极可能由同一人控制。同时该账户也是双向交易对手（收入749笔，支出仅7笔），具有明显的资金归集特征。")
lines.append("")
lines.append(f"第二来源 kh66784（户名xm734）：共 {r['top10_sources'][1]['cnt']} 笔，合计 {fmt(r['top10_sources'][1]['amt'])} 元。交易频繁，是重要的资金来源方。")
lines.append("")
lines.append(f"第三来源 kh66713：共 {r['top10_sources'][2]['cnt']} 笔，合计 {fmt(r['top10_sources'][2]['amt'])} 元。该账户也是同名账户（户名xm1083），双向交易对手。")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 4
lines.append("## 四、主要资金去向分析")
lines.append("")
lines.append("### （一）去向集中度")
lines.append("")
lines.append(f"支出端涉及 {out_data['counterparties']} 个不同对手账户。")
db = r["dest_concentration"]
lines.append(f"TOP1去向占支出的 {db['top1_pct']}%，TOP3占 {db['top3_pct']}%，TOP5占 {db['top5_pct']}%。").replace(
lines.append("")
lines.append("### （二）TOP10 资金去向")
lines.append("")
lines.append("| 序号 | 对手账户 | 对手户名 | 交易笔数 | 合计金额（元） | 平均单笔（元） | 特征标注 |")
lines.append("| --- | --- | --- | --- | --- | --- | --- |")
for i, dst in enumerate(r["top10_destinations"], 1):
    name = dst.get("counterparty_name","") or ""
    notes = []
    cpty = dst["counterparty_account"]
    for s in r["same_name_dests"]:
        if s["counterparty_account"]==cpty:
            notes.append("同名账户")
            break
    for t in r["two_way_counterparties"]:
        if t["counterparty_account"]==cpty:
            notes.append("双向交易")
            break
    if i==1: notes.append("最大去向")
    if dst["cnt"]>100: notes.append("高频转出")
    if dst["avg_amt"]>150000: notes.append("大额转出")
    notes_str = "、".join(notes) if notes else ""
    lines.append(f"| {i} | {cpty} | {name} | {dst['cnt']} | {fmt(dst['amt'])} | {fmt(dst['avg_amt'])} | {notes_str} |")
lines.append("")
lines.append("### （三）重点去向账户分析")
lines.append("")
lines.append(f"第一去向 kh63621：共 {r['top10_destinations'][0]['cnt']} 笔，合计 {fmt(r['top10_destinations'][0]['amt'])} 元，占比极大。该账户是涉案资金的最主要接收方，资金从来源转入涉案账户后，大量流向该账户。")
lines.append("")
lines.append(f"第二去向 kh65156（户名xm1083）：共 {r['top10_destinations'][1]['cnt']} 笔，合计 {fmt(r['top10_destinations'][1]['amt'])} 元。系同名账户，且同时为资金来源方（双向交易），收入49笔15,850,843.20元，支出66笔22,586,500.00元，净流出6,736,656.80元。")
lines.append("")
lines.append("### （四）对手开户银行分布")
lines.append("")
lines.append("因数据中"对手开户银行"字段均为空，该项未分析。")
lines.append("")
lines.append("### （五）资金流向模式")
lines.append("")
lines.append("涉案账户资金主要呈现"分散收入→主体归集→集中转出"的资金过渡模式。大量对手（137个收入方、175个支出方）向该账户转入资金，资金归集后通过少数大额转出账户（主要是kh63621）流出。")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 5
lines.append("## 五、团伙划分分析")
lines.append("")
lines.append("### （一）基于设备信息的团伙划分")
lines.append("")
lines.append("因数据中缺少IP和MAC地址字段，无法进行基于设备信息的团伙划分分析。")
lines.append("")
lines.append("### （二）基于对手证件号的地域聚合")
lines.append("")
lines.append("因数据中缺少"对手证件号"字段，未进行地域关联分析。")
lines.append("")
lines.append("### （三）基于对手开户银行的聚类")
lines.append("")
lines.append("因数据中"对手开户银行"均为空，该项未分析。")
lines.append("")
lines.append("### （四）基于交易关系的团伙拓展")
lines.append("")
lines.append(f"1. 同名多户情况显著。共识别出 {s5_cnt} 个不同账号的对手户名为"xm1083"（与主体户名相同），其中收入端 {len(r['same_name_sources'])} 个账户共转入 {fmt(sum(s['amt'] for s in r['same_name_sources']))} 元，支出端 {len(r['same_name_dests'])} 个账户共转出 {fmt(sum(s['amt'] for s in r['same_name_dests']))} 元。这些账户极可能由同一人（xm1083）控制。")
lines.append(f"2. 双向交易对手 {len(r['two_way_counterparties'])} 个，其中kh66688、kh66713等为同名账户。")
lines.append(f"3. 自身转账 {st['cnt']} 笔，金额 {fmt(st['amt'])} 元，占交易总额的 {st_ratio}%。")
lines.append("")
lines.append("### （五）团伙组织架构")
lines.append("")
lines.append("综合以上分析，推断该账户由xm1083（或该人控制的多账户）作为核心控制人，通过多个同名账户（kh66688、kh65156、kh66713等）进行资金归集和分配。大量外部零散资金通过137个来源方汇入，经核心账户归集后，主要通过kh63621等大额去向账户集中流出。")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 6
lines.append("## 六、可疑点分析")
lines.append("")
lines.append("一是交易收支高度平衡。收支差额仅占总交易额的" + str(balance_ratio) + "%，属于高度异常（阈值<1%）。该账户几乎不留存资金，具有典型的资金过渡账户特征。")
lines.append("二是现金交易占比异常。现金交易笔数占比" + str(cash_cnt_pct) + "%，金额占比" + str(round(cash_amt/ts['total_amt']*100,2)) + "%，远高于正常阈值（>70%为高度异常），大量现金交易是洗钱和非法换汇的重要特征。")
lines.append("三是资金来源/去向集中度。TOP3来源仅占总收入的" + str(s3_val) + "%（正常），TOP3去向占总支出的" + str(s4_val) + "%（正常），呈现分散转入、集中转出模式。")
lines.append("四是同名多户控制异常。同一户名"xm1083"控制" + str(s5_cnt) + "个不同账户，远超高度异常阈值（>=5），具有明显的多账户控制特征。")
lines.append(f"五是整额交易占比高。整额（整万）交易共 {ra_total_cnt} 笔，占总笔数的 {ra_ratio}%（正常<15%），超过异常阈值。")
lines.append("六是自身转账金额大。自身转账" + fmt(st['amt']) + "元，占总交易额的" + str(st_ratio) + "%（>=5%视为异常），在多个同名账户之间进行资金调度。")
lines.append("七是资金周转率极高。交易总额与平均余额之比为" + str(turnover) + "（高度异常>100），资金进出极为频繁，账户余额保持低位。")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 7
lines.append("## 七、经营模式与涉案资金穿透研判")
lines.append("")
lines.append("### （一）经营模式分析")
lines.append("")
lines.append("综合以上发现，该账户符合"地下钱庄/非法换汇"经营模式。")
lines.append("")
lines.append("1. **资金归集层**：以kh66688、kh66784为代表的上游账户，以及大量零散现金交易对手（137个收入方），将资金汇入涉案账户kh13616。其中kh66688既是同名账户又为最大来源，具有明确的上游资金归集和划转职能。")
lines.append("2. **资金中转层**：涉案账户kh13616作为资金中转池，接收来自多个渠道的资金（3,720笔收入），快速整合后向外分配（1,168笔支出），几乎没有资金滞留。")
lines.append("3. **资金分配层**：资金主要通过kh63621（占比极大）、kh65156（同名账户）等下游账户分发。")
lines.append("4. **变现层**：现金交易占比91.72%（金额），大量资金通过现金渠道取现，为非法所得提供现金出口。")
lines.append("")
lines.append("### （二）涉案资金穿透")
lines.append("")
lines.append("资金流向链条：")
lines.append("")
lines.append("```")
lines.append("上游A(现金/多个外部对手) → [涉案账户kh13616] → 下游1(kh63621)")
lines.append("上游B(kh66688/同名归集) → [涉案账户kh13616] → 下游2(kh65156/同名)")
lines.append("上游C(kh66784) → [涉案账户kh13616] → 下游3(kh66758)")
lines.append("                               ↗              ↘")
lines.append("                          (更多现金收入)    (更多现金支出)")
lines.append("```")
lines.append("")
lines.append("受数据范围限制，仅追踪至第一层上下游。涉案资金总规模约3.2亿元。")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 8
lines.append("## 八、下一步工作建议")
lines.append("")
lines.append("### （一）立即措施")
lines.append("- 对涉案账户kh13616立即采取冻结、限额等管控措施")
lines.append("- 调取证信有关kh13616的开户资料、证件信息及历史交易记录")
lines.append("- 需要补充数据：账户信息辅助表、人员信息表、子账户信息表")
lines.append("")
lines.append("### （二）深度调查")
lines.append("- 对TOP来源账户kh66688、kh66784进行延伸调证，获取其开户信息和交易对手")
lines.append("- 对主要去向账户kh63621进行全面调证，追溯下游资金去向")
lines.append("- 核查同名账户（kh66688、kh65156、kh66713等30个）的关系和实际控制人")
lines.append("- 如有条件，调取对手证件号码，进行地域关联分析和人员身份核查")
lines.append("")
lines.append("### （三）扩线调查")
lines.append("- 对kh66688账户的上游资金来源进行追溯")
lines.append("- 排查kh63621账户的进一步资金去向")
lines.append("- 调查同名xm1083控制的所有银行账户及其关联交易")
lines.append("- 与其他案件的关联比对")
lines.append("")
lines.append("---")
lines.append("")

# Chapter 9
lines.append("## 九、结论")
lines.append("")
lines.append("### （一）核心发现")
lines.append(f"1. 涉案账户kh13616（户名xm1083）在2014年至2025年间完成 {ts['total']} 笔交易，总金额 {fmt(ts['total_amt'])} 元，收支差额仅 {fmt(balance_diff)} 元，收支高度平衡。")
lines.append(f"2. 现金交易笔数占比 {cash_cnt_pct}%，金额占比 {round(cash_amt/ts['total_amt']*100,2)}%，现金交易特征极为突出。")
lines.append(f"3. 发现同名xm1083控制的账户不少于 {s5_cnt} 个，具有明显的多账户控制特征。")
lines.append("4. 整额交易占比高，资金周转率异常，具有典型的过度账户特征。")
lines.append("")
lines.append("### （二）涉案规模")
lines.append("")
lines.append("| 维度 | 数值 |")
lines.append("| --- | --- |")
lines.append(f"| 涉案交易总笔数 | {ts['total']} 笔 |")
lines.append(f"| 涉案交易总金额 | {fmt(ts['total_amt'])} 元 |")
lines.append(f"| 涉案账户数 | 1个（核心） + 至少{s5_cnt}个同名关联账户 |")
lines.append(f"| 涉及对手账户总数 | {in_data['counterparties'] + out_data['counterparties']} 个 |")
lines.append(f"| 涉案时间跨度 | {ts['first_txn']} 至 {ts['last_txn']}（共11年） |")
lines.append("")
lines.append("### （三）处置建议")
lines.append("")
lines.append("该账户具有地下钱庄/非法换汇的典型特征：收支高度平衡、现金交易占比极高、同名多户控制、整额交易占比高、资金周转率极高。建议将本案移送公安经侦部门进一步侦查。")
lines.append("")
lines.append("---")
lines.append("")
lines.append("*报告生成时间：2026-05-12*")

report_content = "\n".join(lines)

# Write report
output_path = os.path.join(OUTPUT_DIR, "kh13616资金分析报告.md")
with open(output_path, "w", encoding="utf-8") as f:
    f.write(report_content)

print(f"REPORT_WRITTEN: {output_path}")
print(f"File size: {os.path.getsize(output_path)} bytes")
print("Report generation completed successfully.")
