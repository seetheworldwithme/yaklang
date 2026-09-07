package loop_fund_analysis

import (
	"bytes"
	_ "embed"
	"fmt"
	"strconv"
	"strings"

	"github.com/yaklang/yaklang/common/ai/aid/aicommon"
	"github.com/yaklang/yaklang/common/ai/aid/aireact/reactloops"
	"github.com/yaklang/yaklang/common/ai/aid/aitool"
	"github.com/yaklang/yaklang/common/log"
	"github.com/yaklang/yaklang/common/schema"
	"github.com/yaklang/yaklang/common/utils"
)

//go:embed prompts/persistent_instruction.txt
var persistentInstruction string

//go:embed prompts/reactive_data.txt
var reactiveDataTemplate string

//go:embed prompts/reflection_output_example.txt
var reflectionOutputExample string

func init() {
	err := reactloops.RegisterLoopFactory(
		schema.AI_REACT_LOOP_NAME_FUND_ANALYSIS,
		func(r aicommon.AIInvokeRuntime, opts ...reactloops.ReActLoopOption) (*reactloops.ReActLoop, error) {
			state := NewAnalysisState()

			metaActions := []string{
				schema.AI_REACT_LOOP_ACTION_DIRECTLY_ANSWER,
				"finish",
				schema.AI_REACT_LOOP_ACTION_SEARCH_CAPABILITIES,
				schema.AI_REACT_LOOP_ACTION_LOAD_CAPABILITY,
				schema.AI_REACT_LOOP_ACTION_LOADING_SKILLS,
				schema.AI_REACT_LOOP_ACTION_LOAD_SKILL_RESOURCES,
				schema.AI_REACT_LOOP_ACTION_CHANGE_SKILL_VIEW_OFFSET,
				"fund_analysis_progress",
			}
			toolActions := []string{
				"read_file",
				"write_file",
				"find_files",
				"grep_text",
				"bash",
			}
			allowed := append(append([]string{}, metaActions...), toolActions...)
			if r.GetConfig().GetAllowUserInteraction() {
				allowed = append(allowed, schema.AI_REACT_LOOP_ACTION_ASK_FOR_CLARIFICATION)
			}

			maxIter := int(r.GetConfig().GetMaxIterationCount())
			if maxIter < 20 {
				maxIter = 20
			}

			preset := []reactloops.ReActLoopOption{
				reactloops.WithDisableDirectlyCallTool(true),
				reactloops.WithAllowRAG(false),
				reactloops.WithMemoryTriage(nil),
				reactloops.WithAllowToolCall(true),
				reactloops.WithAllowAIForge(false),
				reactloops.WithAllowPlanAndExec(false),
				reactloops.WithInitTask(buildInitTask(r, state)),
				reactloops.WithMaxIterations(maxIter),
				reactloops.WithAllowUserInteract(r.GetConfig().GetAllowUserInteraction()),
				reactloops.WithActionFilter(func(action *reactloops.LoopAction) bool {
					for _, name := range allowed {
						if action.ActionType == name {
							return true
						}
					}
					return false
				}),
				reactloops.WithPersistentInstruction(persistentInstruction),
				reactloops.WithReflectionOutputExample(reflectionOutputExample),
				reactloops.WithReactiveDataBuilder(func(loop *reactloops.ReActLoop, feedbacker *bytes.Buffer, nonce string) (string, error) {
					state.mu.RLock()
					renderMap := map[string]any{
						"Nonce":               nonce,
						"CurrentPhase":        state.CurrentPhase,
						"DataFiles":           state.GetDataFiles(),
						"SuggestedSkills":     state.GetSuggestedSkillsLine(),
						"LoadedSkills":        state.GetLoadedSkillsLine(),
						"PhaseADone":          state.PhaseADone,
						"PhaseBDone":          state.PhaseBDone,
						"PhaseCDone":          state.PhaseCDone,
						"PhaseDDone":          state.PhaseDDone,
						"FindingsCount":       state.FindingsCount,
						"ProgressLog":         state.GetProgressLog(),
						"FeedbackMessages":    strings.TrimSpace(feedbacker.String()),
						"SkillCatalogSummary": buildSkillCatalogSummary(),
					}
					state.mu.RUnlock()
					return utils.RenderTemplate(reactiveDataTemplate, renderMap)
				}),
				buildProgressAction(r, state),
				reactloops.WithOnPostIteraction(buildFinalize(r, state)),

				reactloops.WithSameActionTypeSpinThreshold(5),
				reactloops.WithEnableSelfReflection(false),
				reactloops.WithDisableLoopPerception(true),
				reactloops.WithPeriodicVerificationInterval(999999),
			}
			preset = append(preset, opts...)
			// ExecuteLoopTask 会在 opts 里追加 BasicAICommonConfigOption（含全局 MemoryTriage、
			// PeriodicVerificationInterval、EnableSelfReflection 等），顺序在 preset 之后，会覆盖本 loop 的约束。
			// 此处再次收紧，确保关闭记忆注入、RAG、反思与周期核实。
			preset = append(preset,
				reactloops.WithAllowRAG(false),
				reactloops.WithMemoryTriage(nil),
				reactloops.WithAllowAIForge(false),
				reactloops.WithAllowPlanAndExec(false),
				reactloops.WithEnableSelfReflection(false),
				reactloops.WithDisableLoopPerception(true),
				reactloops.WithPeriodicVerificationInterval(999999),
			)
			return reactloops.NewReActLoop(schema.AI_REACT_LOOP_NAME_FUND_ANALYSIS, r, preset...)
		},
		reactloops.WithLoopDescription("Economic crime investigation hub: route built-in skills (AML, tax, bribery, fund tracing, trade, stakeholder, counterfeit, general fund) to analyze user data and produce reports."),
		reactloops.WithLoopDescriptionZh("经侦综合研判：按用户目的加载内置技能（反洗钱、涉税、商业贿赂、资金穿透、商贸、涉众、假币、通用资金等），完成数据分析与报告输出。"),
		reactloops.WithVerboseName("Economic Crime Investigation"),
		reactloops.WithVerboseNameZh("经侦综合研判"),
		reactloops.WithLoopUsagePrompt("当用户就行侦相关数据（流水、发票、涉税表、商贸合同背景等）提出分析、研判或撰写报告时使用。先根据目的 loading_skills 加载对应技能（可多技能顺序执行），再按该技能文档的分阶段框架完成分析。"),
		reactloops.WithLoopOutputExample(reflectionOutputExample),
	)
	if err != nil {
		log.Errorf("register reactloop: %v failed: %v", schema.AI_REACT_LOOP_NAME_FUND_ANALYSIS, err)
	}
}

func buildInitTask(r aicommon.AIInvokeRuntime, state *AnalysisState) func(loop *reactloops.ReActLoop, task aicommon.AIStatefulTask, operator *reactloops.InitTaskOperator) {
	return func(loop *reactloops.ReActLoop, task aicommon.AIStatefulTask, operator *reactloops.InitTaskOperator) {
		userInput := task.GetUserInput()
		r.AddToTimeline("[ECONOMIC_CRIME_TASK_START]", utils.ShrinkTextBlock(userInput, 280))

		extractDataFilePaths(userInput, state)

		suggested := InferSuggestedSkills(userInput)
		state.SetSuggestedSkills(suggested)
		loop.Set("suggested_skills", strings.Join(suggested, ","))
		loop.Set("primary_skill_hint", suggested[0])
		loop.Set("current_phase", "init")

		operator.Continue()
	}
}

func buildSkillCatalogSummary() string {
	var b strings.Builder
	for _, ent := range skillCatalogEntries {
		b.WriteString("- `")
		b.WriteString(ent.ID)
		b.WriteString("` — ")
		b.WriteString(ent.ZhLabel)
		b.WriteString("：")
		b.WriteString(ent.Description)
		b.WriteString("\n")
	}
	return strings.TrimSpace(b.String())
}

func extractDataFilePaths(input string, state *AnalysisState) {
	exts := []string{".xlsx", ".xls", ".csv", ".tsv"}
	for _, word := range strings.Fields(input) {
		word = strings.Trim(word, "\"'()[]{}、，。")
		lower := strings.ToLower(word)
		for _, ext := range exts {
			if strings.HasSuffix(lower, ext) {
				state.AddDataFile(word)
				break
			}
		}
	}
}

func buildProgressAction(r aicommon.AIInvokeRuntime, state *AnalysisState) reactloops.ReActLoopOption {
	return reactloops.WithRegisterLoopAction(
		"fund_analysis_progress",
		"记录当前所用技能下的阶段进度与关键发现。fund-analysis 可用经典阶段 A/B/C/D；其他技能请使用该技能文档中的阶段编号或简短代号（如 stage2、H3）。",
		[]aitool.ToolOption{
			aitool.WithStringParam("skill_name",
				aitool.WithParam_Required(false),
				aitool.WithParam_Description("当前进度对应的技能 ID（如 fund-analysis、anti-money-analysis）；默认 fund-analysis"),
			),
			aitool.WithStringParam("phase",
				aitool.WithParam_Required(true),
				aitool.WithParam_Description("刚完成的阶段：fund-analysis 填 A/B/C/D；其他技能填文档中的阶段标识"),
			),
			aitool.WithStringParam("summary",
				aitool.WithParam_Required(true),
				aitool.WithParam_Description("本阶段关键发现的简要摘要"),
			),
		},
		func(loop *reactloops.ReActLoop, action *aicommon.Action) error {
			phaseRaw := NormalizePhase(action.GetString("phase"))
			if !IsAllowedPhaseToken(phaseRaw) {
				return fmt.Errorf("phase invalid or too long: %q", action.GetString("phase"))
			}
			if action.GetString("summary") == "" {
				return utils.Error("fund_analysis_progress requires a non-empty summary")
			}
			return nil
		},
		func(loop *reactloops.ReActLoop, action *aicommon.Action, op *reactloops.LoopActionHandlerOperator) {
			skillName := strings.TrimSpace(action.GetString("skill_name"))
			if skillName == "" {
				skillName = SkillFundAnalysis
			}
			state.RememberLoadedSkill(skillName)

			phase := NormalizePhase(action.GetString("phase"))
			summary := action.GetString("summary")

			state.MarkPhaseDoneForSkill(skillName, phase)
			entry := fmt.Sprintf("[%s phase %s] %s", skillName, phase, summary)
			state.AppendProgress(entry)

			r.AddToTimeline("economic_crime_progress_"+skillName+"_"+phase, summary)
			loop.Set("current_phase", state.GetPhase())
			if skillName == SkillFundAnalysis && IsFundClassicPhase(phase) {
				loop.Set("phase_"+strings.ToLower(phase)+"_done", "true")
			}

			findingsStr := action.GetString("findings_count")
			if n, err := strconv.Atoi(findingsStr); err == nil && n > 0 {
				state.IncrFindings(n)
			}

			op.Feedback(fmt.Sprintf("Phase %s recorded. Current phase: %s. Proceed to next phase or directly_answer if all done.", phase, state.GetPhase()))
			op.Continue()
		},
	)
}

func buildFinalize(r aicommon.AIInvokeRuntime, state *AnalysisState) func(loop *reactloops.ReActLoop, iteration int, task aicommon.AIStatefulTask, isDone bool, reason any, op *reactloops.OnPostIterationOperator) {
	return func(loop *reactloops.ReActLoop, iteration int, task aicommon.AIStatefulTask, isDone bool, reason any, op *reactloops.OnPostIterationOperator) {
		if !isDone {
			return
		}
		if loop.Get("fund_report_delivered") == "true" {
			return
		}

		progressLog := state.GetProgressLog()
		if progressLog == "" {
			r.EmitResultAfterStream("# 资金分析报告\n\n本次分析未产生有效结果，请提供交易流水数据后重新开始。")
			return
		}

		r.AddToTimeline("[FUND_ANALYSIS_FALLBACK]", "loop ended without report, emitting progress summary")
		fallbackReport := "# 资金分析报告（未完成）\n\n分析在完成报告前终止，以下是已收集的阶段性发现：\n\n" + progressLog
		r.EmitResultAfterStream(fallbackReport)

		if reasonErr, ok := reason.(error); ok && strings.Contains(reasonErr.Error(), "max iterations") {
			op.IgnoreError()
		}
	}
}
