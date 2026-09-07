# 参考代码（Python / pandas）

本参考采用 **pandas 直接读 xlsx/csv**，不在流程中使用 SQLite 或 `xlsx_to_sqlite`。产物为 Markdown 报告与可选的分析脚本（`.py`）。

---

## 一、执行原则

1. 用 `pd.read_excel` / `read_csv` 加载数据；多 sheet 用 `sheet_name=[...]` 或循环读取。
2. 分析逻辑优先 **DataFrame** 操作：`merge`、`groupby`、布尔索引、`rolling` 等。
3. Markdown 报告可直接 `write_file` 落盘；复杂流程可将脚本写入工作目录后 `python3 ./analyze_tax.py` 执行。
4. 大表可先 `df.to_csv("cache/xxx.csv", index=False)` 再分块 `read_csv(..., chunksize=...)`。

---

## 二、逻辑表映射（一次）

目标是在内存中持有（名称自取，以下为约定变量名）：

- `seller_invoice`
- `buyer_invoice`
- `tax_registration`
- `business_registration`
- `bank_transaction`（可缺）

示例：从目录加载并打成字典（列名以实际文件为准）：

```python
from pathlib import Path
import pandas as pd

DATA = Path("/path/to/xlsx/dir")
tables: dict[str, pd.DataFrame] = {}

# 示例：单文件多 sheet
xf = pd.ExcelFile(DATA / "涉税数据.xlsx")
for sheet in xf.sheet_names:
    tables[f"raw__{sheet}"] = pd.read_excel(xf, sheet_name=sheet)

# 根据列命中人工或启发式挑选后赋值，例如：
# tables["seller_invoice"] = tables.pop("raw__销项明细")
```

映射后可选择性保存：

```python
import json
mapping = {"seller_invoice": "涉税数据.xlsx::销项明细"}
Path("output/tax-fraud-analysis-0327/table-mapping.json").write_text(
    json.dumps(mapping, ensure_ascii=False, indent=2), encoding="utf-8"
)
```

---

## 三、中文路径与编码

- 路径统一使用 `pathlib.Path`。
- 建议环境：`PYTHONUTF8=1`，终端 UTF-8。
- 日志乱码优先检查终端编码，不直接判定数据异常。

---

## 四、逐户对象识别（pandas）

从税务登记、销项、购方表中汇总纳税人键（识别号优先，否则名称）：

```python
import pandas as pd

def _trim_series(s: pd.Series) -> pd.Series:
    return s.astype(str).str.strip().replace({"nan": pd.NA, "": pd.NA})

def collect_taxpayers(
    tax_registration: pd.DataFrame,
    seller_invoice: pd.DataFrame,
    buyer_invoice: pd.DataFrame,
) -> pd.DataFrame:
    parts = []
    if "纳税人识别号" in tax_registration.columns:
        parts.append(
            pd.DataFrame({
                "tpid": _trim_series(tax_registration["纳税人识别号"]),
                "tpname": _trim_series(tax_registration.get("纳税人名称", pd.NA)),
            })
        )
    parts.append(
        pd.DataFrame({
            "tpid": _trim_series(seller_invoice["销方识别号"]),
            "tpname": _trim_series(seller_invoice.get("销方名称", pd.NA)),
        })
    )
    parts.append(
        pd.DataFrame({
            "tpid": _trim_series(buyer_invoice["购方识别号"]),
            "tpname": _trim_series(buyer_invoice.get("购方名称", pd.NA)),
        })
    )
    ids = pd.concat(parts, ignore_index=True)
    ids["tpkey"] = ids["tpid"].fillna(ids["tpname"])
    ids = ids.dropna(subset=["tpkey"])
    out = ids.groupby("tpkey", as_index=False).agg(
        tpid=("tpid", lambda s: s.dropna().iloc[0] if s.notna().any() else ""),
        tpname=("tpname", lambda s: s.dropna().iloc[0] if s.notna().any() else ""),
    )
    return out.sort_values("tpkey")
```

---

## 五、核心风险指标（示例，销项侧）

以下假定已得到 `seller_invoice`，且列名与 SKILL 正文一致；参数 `tpid`、`tpname` 为字符串。

### 1）销项概况（单个纳税人）

```python
import pandas as pd
import numpy as np

def mask_taxpayer(df: pd.DataFrame, tpid: str, tpname: str) -> pd.Series:
    m = df["销方识别号"].astype(str).str.strip() == tpid.strip()
    if not tpid.strip():
        m = df["销方名称"].astype(str).str.strip() == tpname.strip()
    return m

def seller_summary(seller_invoice: pd.DataFrame, tpid: str, tpname: str) -> pd.Series:
    s = seller_invoice.loc[mask_taxpayer(seller_invoice, tpid, tpname)].copy()
    amt = pd.to_numeric(s["价税合计"], errors="coerce")
    goods = pd.to_numeric(s.get("货物金额", np.nan), errors="coerce")
    tax = pd.to_numeric(s.get("货物税额", np.nan), errors="coerce")
    inv_key = s["发票代码"].astype(str) + s["发票号码"].astype(str)
    return pd.Series({
        "明细行数": len(s),
        "发票张数": inv_key.nunique(),
        "货物金额": goods.sum(skipna=True),
        "税额": tax.sum(skipna=True),
        "价税合计": amt.sum(skipna=True),
    })
```

### 2）R02 / R05 / R08 等量化（示例）

```python
def risk_flags_seller(seller_invoice: pd.DataFrame, tpid: str, tpname: str) -> pd.Series:
    s = seller_invoice.loc[mask_taxpayer(seller_invoice, tpid, tpname)].copy()
    amt = pd.to_numeric(s["价税合计"], errors="coerce")
    total_cnt = len(s)
    amt_cnt = amt.notna().sum()
    whole_cnt = ((amt % 10000) == 0) & amt.notna()
    cancel = s.get("作废标志", pd.Series("", index=s.index)).astype(str)
    cancel_cnt = cancel.str.contains("是|Y|作废|1", case=False, regex=True).sum()
    remote = s.get("异地发票标志", pd.Series("", index=s.index)).astype(str).str.strip()
    remote_cnt = (remote.ne("") & ~remote.isin(["0", "否", "N"])).sum()
    return pd.Series({
        "R02_整额开票占比": round(whole_cnt.sum() * 100.0 / amt_cnt, 2) if amt_cnt else 0.0,
        "R05_作废率": round(cancel_cnt * 100.0 / total_cnt, 2) if total_cnt else 0.0,
        "R08_异地发票占比": round(remote_cnt * 100.0 / total_cnt, 2) if total_cnt else 0.0,
    })
```

### 3）够罪条件（专票税额 / 普票金额）

```python
def crime_thresholds(seller_invoice: pd.DataFrame, tpid: str, tpname: str) -> pd.Series:
    s = seller_invoice.loc[mask_taxpayer(seller_invoice, tpid, tpname)].copy()
    inv_type = s.get("发票类型", "").astype(str)
    tax_amt = pd.to_numeric(s.get("货物税额", np.nan), errors="coerce")
    total_amt = pd.to_numeric(s["价税合计"], errors="coerce")

    vat_mask = inv_type.str.contains("专用|专票", regex=True) | (inv_type == "")
    norm_mask = inv_type.str.contains("普通|普票", regex=True)

    vat_sum = tax_amt.where(vat_mask, np.nan).sum(skipna=True)
    norm_sum = total_amt.where(norm_mask, np.nan).sum(skipna=True)

    def bucket_vat(x: float) -> str:
        if x >= 5_000_000:
            return "专票第三档（>=500万）"
        if x >= 500_000:
            return "专票第二档（>=50万）"
        if x >= 50_000:
            return "专票第一档（>=5万）"
        return "专票未达入罪参考阈值"

    def bucket_norm(x: float) -> str:
        if x >= 5_000_000:
            return "普票情节特别严重（>=500万）"
        if x >= 500_000:
            return "普票情节严重（>=50万）"
        return "普票一般或未达高风险阈值"

    return pd.Series({
        "专票税额": round(vat_sum, 2),
        "专票档次": bucket_vat(vat_sum),
        "普票金额": round(norm_sum, 2),
        "普票档次": bucket_norm(norm_sum),
    })
```

---

## 六、Markdown 报告落盘

- 逐户：`output/tax-fraud-analysis-0327/reports/taxpayer-*.md`
- 汇总：`output/tax-fraud-analysis-0327/summary-report.md`

可直接写入上述路径；也可用 Python：

```python
from pathlib import Path
Path("output/tax-fraud-analysis-0327/reports/taxpayer-example.md").write_text(
    "# 纳税人 XXX\n\n...", encoding="utf-8"
)
```

---

## 七、何时使用额外库

- **NetworkX**：关联企业 / 票流图遍历。
- **python-Levenshtein / rapidfuzz**：名称相似度（按需安装）。
- **matplotlib**：必要时出简单分布图（非必需）。

即便使用这些库，仍保持「读 xlsx → pandas → 报告」主线，不引入 SQLite。
