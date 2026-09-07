package loop_fund_analysis

import (
	"strings"
	"unicode"
)

// Builtin economic-crime skill IDs shipped under common/ai/aid/aireact/skills/.
// loading_skills uses the `name` from each SKILL.md frontmatter.
const (
	SkillFundAnalysis            = "fund-analysis"
	SkillTaxFraudAnalysis        = "tax-fraud-analysis"
	SkillAntiMoneyAnalysis       = "anti-money-analysis"
	SkillCommercialBribery       = "commercial-bribery-analysis"
	SkillFundTracingAnalysis     = "fund-tracing-analysis"
	SkillTradeAnalysis           = "trade-analysis"
	SkillStakeholderAnalysis     = "stakeholder-analysis"
	SkillAntiCounterfeitAnalysis = "anti-counterfeit-analysis"
)

// skillCatalogEntry is used for prompts (short routing table).
var skillCatalogEntries = []struct {
	ID          string
	ZhLabel     string
	Keywords    []string
	Description string
}{
	// 具体罪名优先匹配；fund-analysis 作为默认（见 InferSuggestedSkills）
	{SkillAntiMoneyAnalysis, "反洗钱", []string{"洗钱", "反洗钱", "aml", "地下钱庄", "非法换汇"}, "融合反洗钱技战法规则的资金分析"},
	{SkillTaxFraudAnalysis, "涉税犯罪", []string{"涉税", "税务", "发票", "虚开", "增值税", "出口退税", "偷税"}, "发票 / 税务登记 / 涉税数据（技能内多为 SQL 编排）"},
	{SkillCommercialBribery, "商业贿赂", []string{"贿赂", "行贿", "受贿", "回扣", "佣金", "白手套"}, "商业贿赂「三定三破」资金流"},
	{SkillFundTracingAnalysis, "资金穿透", []string{"穿透", "资金链", "中转", "过渡户", "多层"}, "多层穿透、来源去向链路"},
	{SkillTradeAnalysis, "商贸犯罪", []string{"商贸", "合同诈骗", "职务侵占", "串通投标", "非法经营"}, "商贸领域经济犯罪资金分析"},
	{SkillStakeholderAnalysis, "涉众犯罪", []string{"传销", "非法集资", "集资诈骗", "涉众", "层级"}, "传销 / 非法集资等涉众型资金流"},
	{SkillAntiCounterfeitAnalysis, "假币犯罪", []string{"假币", "伪造货币", "出售假币"}, "假币类犯罪数据分析"},
	{SkillFundAnalysis, "通用资金可疑交易", []string{"流水", "可疑交易", "交易画像", "团伙", "九章"}, "银行流水可疑交易：四阶段 A–D + 九章报告"},
}

// InferSuggestedSkills scans user text and returns skill IDs in catalog order (deduped),
// best-effort hints for loading_skills — model should still confirm against user goal.
func InferSuggestedSkills(userInput string) []string {
	lower := strings.ToLower(strings.TrimSpace(userInput))
	var out []string
	seen := make(map[string]struct{})
	for _, ent := range skillCatalogEntries {
		if ent.ID == SkillFundAnalysis {
			continue
		}
		if skillMatchesInput(lower, ent.Keywords) {
			if _, ok := seen[ent.ID]; ok {
				continue
			}
			seen[ent.ID] = struct{}{}
			out = append(out, ent.ID)
		}
	}
	if skillMatchesInput(lower, []string{"流水", "可疑交易", "交易画像", "团伙", "九章"}) {
		if _, ok := seen[SkillFundAnalysis]; !ok {
			seen[SkillFundAnalysis] = struct{}{}
			out = append(out, SkillFundAnalysis)
		}
	}
	if len(out) == 0 {
		return []string{SkillFundAnalysis}
	}
	return out
}

func skillMatchesInput(lower string, keywords []string) bool {
	for _, kw := range keywords {
		kw = strings.TrimSpace(kw)
		if kw == "" {
			continue
		}
		if strings.Contains(lower, strings.ToLower(kw)) {
			return true
		}
	}
	return false
}

// NormalizePhase validates phase token for fund_analysis_progress.
func NormalizePhase(phase string) string {
	return strings.TrimSpace(phase)
}

// IsFundClassicPhase returns true for single-letter A–D (fund-analysis track).
func IsFundClassicPhase(phase string) bool {
	p := strings.ToUpper(strings.TrimSpace(phase))
	return len(p) == 1 && p[0] >= 'A' && p[0] <= 'D'
}

// IsAllowedPhaseToken allows A–D or a short custom stage id for other skills.
func IsAllowedPhaseToken(phase string) bool {
	p := strings.TrimSpace(phase)
	if p == "" {
		return false
	}
	if IsFundClassicPhase(p) {
		return true
	}
	if len(p) > 24 {
		return false
	}
	for i, r := range p {
		if i == 0 {
			if !unicode.IsLetter(r) && !unicode.IsDigit(r) {
				return false
			}
			continue
		}
		if unicode.IsLetter(r) || unicode.IsDigit(r) || r == '_' || r == '-' {
			continue
		}
		return false
	}
	return true
}
