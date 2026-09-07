# 经侦分析参考代码

本文档包含各类分析模型的核心代码模板，供分析时参考。

---

## 一、MDL文件结构模板

### 1.1 标准MDL结构
```xml
<model>
    <!-- 模型类型 -->
    <type>sql</type>

    <!-- 功能描述 -->
    <description>
        <html>
        &nbsp;&nbsp;分析说明文字<br>详细描述
        </html>
    </description>

    <!-- 预处理动作 -->
    <action>
        <sql>drop table tmp_xxx</sql>
        <sql>
            CREATE TEMPORARY TABLE tmp_xxx AS
            select ... from 资金表 where ...
        </sql>
        <sql>CREATE INDEX idx_xxx ON tmp_xxx(字段名)</sql>
    </action>

    <!-- 展示配置 -->
    <show>
        <style>grid</style>  <!-- 可选: grid, label, bar -->

        <!-- 下钻查询 -->
        <drill_down col=5>
            select * from 资金表 where 对手卡号='$对手卡号'
        </drill_down>

        <!-- 主查询SQL -->
        <sql title="标题" parameters="参数名:默认值;显示前:50,100,200">
            select ... from ... group by ... having ... order by ...
        </sql>

        <!-- 图表定义 -->
        <bar title="图表标题">
            select ... from ... group by ...
        </bar>

        <!-- 资金流向图 -->
        <flow title="资金流向图">
            select 交易方户名,对手户名,sum(交易金额) from ...
        </flow>
    </show>
</model>
```

---

## 二、资金分析核心SQL

### 2.1 资金来源分析
```sql
-- 按交易户名和对手户名统计资金来源
SELECT
    交易户名,
    证件号码,
    对手户名,
    对手证件号码,
    SUM(交易金额) as 转账总金额,
    COUNT(*) as 转账总笔数,
    SUM(转出额) as 转出金额,
    SUM(转出笔) as 转出笔数,
    SUM(转入额) as 转入金额,
    SUM(转入笔) as 转入笔数,
    SUM(交易双方差) as 交易双方差,
    MIN(datetime(交易时间/1000,'unixepoch','localtime')) as 首次交易时间,
    MAX(datetime(交易时间/1000,'unixepoch','localtime')) as 最后交易时间
FROM 资金表
WHERE 对手户名 IS NOT NULL
GROUP BY 交易户名, 对手户名
HAVING SUM(交易金额) >= $转账总金额不低于
ORDER BY SUM(交易金额) DESC
```

### 2.2 资金去向分析
```sql
-- 按对手卡号统计资金去向
SELECT
    对手卡号,
    交易户名,
    交易卡号,
    对手户名,
    SUM(转出金额) as 转账总金额,
    SUM(转出笔数) as 转账总笔数,
    SUM(转入金额) as 转入金额,
    SUM(转入笔数) as 转入笔数
FROM tmp_table
GROUP BY 对手卡号, 交易户名, 交易卡号, 对手户名
HAVING SUM(转出金额) >= $转账总金额不低于
ORDER BY SUM(转出金额) DESC
```

### 2.3 即进即出分析预处理
```sql
-- 创建即进即出分析临时表
CREATE TEMPORARY TABLE quickly_inout AS
-- 出账记录（对手方是交易卡号）
SELECT
    id,
    数据来源,
    收付标志,
    交易卡号,
    交易户名,
    对手卡号,
    对手户名,
    strftime('%Y-%m-%d', 交易时间/1000, 'unixepoch') as 交易日期,
    交易时间 as tim,
    abs(交易金额) as amount_out,
    0 as amount_in,
    abs(交易金额) as amount_all,
    交易卡号 as card,
    对手户名 as name,
    1 as count_out,
    0 as count_in,
    (0-abs(交易金额)) as amount
FROM 资金表
WHERE 收付标志 IN ('出','付','取','存')
  AND 交易卡号 <> ''
  AND 交易卡号 IS NOT NULL

UNION

-- 进账记录（对手方是对手卡号）
SELECT
    id,
    数据来源,
    收付标志,
    交易卡号,
    交易户名,
    对手卡号,
    对手户名,
    strftime('%Y-%m-%d', 交易时间/1000, 'unixepoch') as 交易日期,
    交易时间 as tim,
    0 as amount_out,
    abs(交易金额) as amount_in,
    abs(交易金额) as amount_all,
    对手卡号 as card,
    交易户名 as name,
    0 as count_out,
    1 as count_in,
    abs(交易金额) as amount
FROM 资金表
WHERE 收付标志 IN ('进','收','入')
  AND 对手卡号 <> ''
  AND 对手卡号 IS NOT NULL;

-- 创建索引
CREATE INDEX idx_quickly_inout_id ON quickly_inout(id);
CREATE INDEX idx_quickly_inout_card ON quickly_inout(card);
CREATE INDEX idx_quickly_inout_tim ON quickly_inout(tim);
```

### 2.4 过渡账户识别
```sql
-- 识别进出平衡的过渡账户
SELECT
    card as 本卡号,
    name as 户名,
    SUM(amount_all) as 交易总金额,
    COUNT(*) as 总笔数,
    SUM(amount) as 交易净额,
    SUM(amount_out) as 转账总金额,
    SUM(count_out) as 转出笔数,
    SUM(amount_in) as 转账总金额,
    SUM(count_in) as 转入笔数,
    MAX(amount_all) as 单笔最高
FROM tmp_table
GROUP BY card, name
HAVING SUM(amount_all) >= $交易总金额不低于
   AND ABS(SUM(amount)) <= $交易净额不超过
ORDER BY SUM(amount_all) DESC
```

### 2.5 吸金账户识别
```sql
-- 吸金账户：转入远大于转出
SELECT
    交易卡号,
    交易户名,
    SUM(转出金额) as 转出总金额,
    SUM(转出笔) as 转出总笔数,
    COUNT(转出笔) as 转出次数,
    COUNT(转入笔) as 转入次数
FROM 资金表
##where##
GROUP BY 交易卡号
HAVING SUM(转入金额)/(SUM(转出金额)+0.1) > $转入转出总金额比值大于
   AND SUM(转入金额) >= $转入总金额最低
   AND SUM(转入笔数)/(SUM(转出笔数)+0.1) >= $累入转入笔数/转出笔数比值最小
   AND SUM(转入笔数) >= $转入笔数最低
ORDER BY SUM(转入金额)
LIMIT 100000
```

### 2.6 集中转入分散转出
```sql
-- 集中转入分散转出特征识别
SELECT
    交易卡号,
    交易户名,
    SUM(转入金额) as 转入总金额,
    SUM(转出金额) as 转出总金额,
    COUNT(转入笔) as 转入次数,
    COUNT(转出笔) as 转出次数
FROM 资金表
##where##
GROUP BY 交易卡号
HAVING SUM(转入金额)/(SUM(转出金额)+0.1) > $转入转出总金额比值大于
   AND SUM(转入金额)/(SUM(转出金额)+0.1) < $转入转出总金额比值小于
   AND SUM(转入金额) >= $转入总金额最低
   AND SUM(转入笔数)/(SUM(转出笔数)+0.1) >= $累入转入笔数/转出笔数比值最小
   AND SUM(转入笔数) >= $转入笔数最低
   AND SUM(转入笔数) > 10
ORDER BY SUM(转入金额)
```

---

## 三、关联分析核心SQL

### 3.1 群组分析
```sql
-- 按群组标签进行群组分析
SELECT
    群组 as 群组,
    GROUP_CONCAT(DISTINCT 交易户名) as 成员列表,
    对手户名,
    对手卡号,
    对手标签,
    COUNT(DISTINCT 交易户名) as 群组成员,
    COUNT(DISTINCT 对手户名) as 涉及对手数,
    SUM(amount_all) as 交易总金额,
    COUNT(*) as 总笔数,
    SUM(amount_out) as 转账总金额,
    SUM(count_out) as 转出笔数,
    SUM(amount_in) as 转账总金额,
    SUM(count_in) as 转入笔数,
    MAX(amount_all) as 单笔最高,
    SUM(amount_in) - SUM(amount_out) as 交易双方差值,
    datetime(min(交易时间)/1000,'unixepoch') 首次交易时间,
    datetime(max(交易时间)/1000,'unixepoch') 最后交易时间
FROM tmp_0033
WHERE id NOT IN tmp_0035  -- 排除自身交易
GROUP BY 群组, 对手户名, 对手卡号
HAVING SUM(amount_all) >= $交易总金额不低于
ORDER BY SUM(amount_all) DESC
```

### 3.2 共同对手分析
```sql
-- 查找两个调查对象的共同交易对手
SELECT DISTINCT dd.*, ee.*
FROM (
    -- 调查对象1的交易对手
    SELECT
        '$调查方1' as 调查方1,
        aa.对手户名 as 对手户名1,
        aa.对手卡号 as 对手卡号1,
        SUM(aa.交易金额) AS 交易总金额1,
        COUNT(aa.交易金额) AS 交易总笔数1,
        SUM(转入金额) AS 转入金额1,
        SUM(CASE aa.收付标志 WHEN '进' THEN 1 ELSE 0 END) AS 转入笔数1,
        SUM(转出金额) AS 转出金额1,
        SUM(CASE aa.收付标志 WHEN '出' THEN 1 ELSE 0 END) AS 转出笔数1,
        SUM(交易双方差) AS 交易双方差1
    FROM 资金表 aa
    INNER JOIN (
        SELECT DISTINCT 对手卡号 FROM tmp_0001 WHERE 标签 = '$调查方1'
    ) bb ON aa.对手卡号 = bb.对手卡号
    WHERE aa.对手户名 NOT IN (排除列表)
      AND 对手户名 <> '$调查方1'
    GROUP BY aa.对手户名, aa.对手卡号
    ORDER BY 交易双方差 DESC
) dd
INNER JOIN (
    -- 调查对象2的交易对手
    SELECT ... -- 类似结构
) ee ON dd.对手户名1 = ee.对手户名2  -- 或按卡号匹配
```

---

## 四、电子支付分析SQL

### 4.1 支付宝交易概览
```sql
-- 支付宝交易基础统计
SELECT
    COUNT(DISTINCT 买家ID) as 买家ID数,
    COUNT(DISTINCT 卖家ID) as 卖家ID数,
    COUNT(*) as 交易记录数,
    SUM(abs(交易金额)) as 交易总金额
FROM 支付宝交易记录
```

### 4.2 支付宝大额卖家
```sql
SELECT
    卖家ID,
    卖家昵称,
    SUM(交易金额) as 卖出总金额,
    COUNT(*) as 卖出总笔数,
    COUNT(DISTINCT 买家ID) as 买家数,
    MAX(交易金额) as 单笔最高
FROM 支付宝交易记录
##where##
GROUP BY 卖家ID, 卖家昵称
ORDER BY SUM(交易金额) DESC
```

### 4.3 财付通可疑交易分析
```sql
SELECT
    用户ID,
    用户昵称账号,
    对方ID,
    对手昵称账号,
    SUM(转出金额) as 转账总金额,
    SUM(转出笔数) as 转出笔数,
    SUM(转入金额) as 转账总金额,
    SUM(转入笔数) as 转入笔数,
    SUM(转入金额)-SUM(转出金额) as 交易双方差,
    datetime(min(交易时间)/1000,'unixepoch') 首次交易时间,
    datetime(max(交易时间)/1000,'unixepoch') 最后交易时间
FROM 财付通交易记录
##where##
AND (转出金额 > 0 OR 转入金额 > 0)
GROUP BY 用户ID, 用户昵称账号, 对方ID, 对手昵称账号
ORDER BY SUM(转出金额) DESC
```

---

## 五、通信分析SQL

### 5.1 通话来源分析
```sql
-- 预处理：合并主叫和被叫记录
CREATE TEMPORARY TABLE tmp_0033 AS
-- 主叫记录
SELECT
    id, 数据来源,
    datetime(通话时间/1000,'unixepoch','localtime') 通话时间,
    0 as amount_out,
    abs(通话时长) as amount_in,
    abs(通话时长) as amount_all,
    主叫号码,
    主叫姓名,
    被叫号码,
    被叫姓名,
    0 as count_out,
    1 as count_in
FROM 通话记录
WHERE 主叫号码 IN ('主叫') AND 主叫号码 <> '' AND 被叫号码 <> ''

UNION

-- 被叫记录
SELECT
    id, 数据来源,
    datetime(通话时间/1000,'unixepoch','localtime') 通话时间,
    abs(通话时长) as amount_in,
    0 as amount_out,
    abs(通话时长) as amount_all,
    主叫号码,
    主叫姓名,
    被叫号码,
    被叫姓名,
    1 as count_out,
    0 as count_in
FROM 通话记录
WHERE 主叫号码 IN ('主叫') AND 主叫号码 <> '' AND 被叫号码 <> ''

-- 主查询
SELECT
    主叫号码,
    主叫姓名,
    被叫号码,
    被叫姓名,
    SUM(amount_out) as 主叫总时长,
    SUM(count_out) as 主叫次数,
    SUM(amount_in) as 被叫总时长,
    SUM(count_in) as 被叫次数
FROM tmp_0033
GROUP BY 主叫号码, 被叫号码
HAVING SUM(amount_in) >= $通话时长总和不低于
ORDER BY SUM(amount_in) DESC
```

---

## 六、税务分析SQL

### 6.1 大额涉税分析(购方)
```sql
SELECT
    购方识别号,
    购方名称,
    COUNT(DISTINCT 销方识别号) as 销方数,
    SUM(金额) as 总金额,
    COUNT(*) as 开票次数
FROM 税票
GROUP BY 购方识别号, 购方名称
HAVING SUM(金额) >= $金额阈值
ORDER BY SUM(金额) DESC
```

### 6.2 涉税频率分析
```sql
SELECT
    购方识别号,
    购方名称,
    COUNT(DISTINCT 销方识别号) as 销方数,
    SUM(金额) as 金额,
    COUNT(*) as 开票次数
FROM 税票
GROUP BY 购方识别号
HAVING COUNT(DISTINCT 销方识别号) >= 1
ORDER BY COUNT(*) DESC
```

---

## 七、资金流向图SQL

### 7.1 标准资金流向图
```sql
SELECT
    交易卡号,
    交易户名,
    对手卡号,
    对手户名,
    SUM(转出金额),
    SUM(转出笔数)
FROM 资金表
##where##
GROUP BY 交易卡号, 交易户名, 对手卡号, 对手户名
HAVING SUM(转出金额) > 0
ORDER BY SUM(转出金额) DESC
LIMIT 500
```

---

## 八、时间维度处理

### 8.1 时间字段转换
```sql
-- 将毫秒时间戳转换为日期时间
datetime(交易时间/1000, 'unixepoch', 'localtime') as 交易时间

-- 提取各时间维度
strftime('%Y-%m-%d', 交易时间/1000, 'unixepoch') as 交易日期
strftime('%Y-%m', 交易时间/1000, 'unixepoch') as 交易月份
strftime('%Y', 交易时间/1000, 'unixepoch') as 年
strftime('%m', 交易时间/1000, 'unixepoch') as 月
strftime('%d', 交易时间/1000, 'unixepoch') as 日
strftime('%H', 交易时间/1000, 'unixepoch') as 时
strftime('%w', 交易时间/1000, 'unixepoch') as 周
```

---

## 九、排除关键词列表

### 9.1 金融机构排除
```sql
WHERE 对手户名 NOT LIKE '%银行%'
  AND 对手户名 NOT LIKE '%支付%'
  AND 对手户名 NOT LIKE '%财付通%'
  AND 对手户名 NOT LIKE '%支付宝%'
  AND 对手户名 NOT LIKE '%微信%'
  AND 对手户名 NOT LIKE '%保险%'
  AND 对手户名 NOT LIKE '%证券%'
  AND 对手户名 NOT LIKE '%基金%'
  AND 对手户名 NOT LIKE '%电信%'
  AND 对手户名 NOT LIKE '%移动%'
  AND 对手户名 NOT LIKE '%联通%'
```

### 9.2 常见排除项
```
银行、支付、财付通、支付宝、保险、证券、基金、
电信、移动、联通、电力、水务、燃气、
医院、学校、大学、教育、培训、
平台、电商、淘宝、京东、美团
```

---

## 十、参数配置说明

### 10.1 常用参数定义格式
```
parameters="转账总金额不低于:10000;显示前:50,100,200,500,1000;直方图汇总按:交易日期,交易月份,年,月,日,时,周"
```

### 10.2 下钻查询配置
```xml
<!-- 按列号配置下钻 -->
<drill_down col=5>
    select * from 资金表 where 对手卡号='$对手卡号'
</drill_down>

<!-- 带图表目标的下钻 -->
<drill_down target=chart col=0>
    select sum(amount_in), sum(0-amount_out), avg(数据来源)
    from tmp_table where card='$本卡号'
    group by $直方图汇总按
</drill_down>
```

### 10.3 点击图表联动
```xml
<click_chart>
>0:  -- 正值时查询
select * from 资金表 where 收付标志 in ('出','付','取')
  and 对手卡号='$$title'
  and strftime('$$format', 交易时间/1000, 'unixepoch')='$$bar_value'

<0:  -- 负值时查询
select * from 资金表 where 收付标志 in ('进','收','入')
  and 对手卡号='$$title'
  and strftime('$$format', 交易时间/1000, 'unixepoch')='$$bar_value'
</click_chart>
```
