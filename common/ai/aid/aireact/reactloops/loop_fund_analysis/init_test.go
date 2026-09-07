package loop_fund_analysis_test

import (
	"bytes"
	"context"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
	"github.com/yaklang/yaklang/common/ai/aid/aicommon"
	"github.com/yaklang/yaklang/common/ai/aid/aireact"
	"github.com/yaklang/yaklang/common/ai/aid/aireact/reactloops"
	_ "github.com/yaklang/yaklang/common/ai/aid/aireact/reactloops/reactinit"
	"github.com/yaklang/yaklang/common/schema"
)

func TestFundAnalysis_ProgressAndDirectlyAnswer(t *testing.T) {
	var aiCallCount int32
	var events []*schema.AiOutputEvent
	var eventsMu sync.Mutex

	reactIns, err := aireact.NewTestReAct(
		aicommon.WithAICallback(func(i aicommon.AICallerConfigIf, req *aicommon.AIRequest) (*aicommon.AIResponse, error) {
			n := atomic.AddInt32(&aiCallCount, 1)
			rsp := i.NewAIResponse()

			switch n {
			case 1:
				rsp.EmitOutputStream(bytes.NewBufferString(
					`{"@action":"fund_analysis_progress","phase":"A","summary":"交易概况: 总笔数500笔, 总金额200万元","human_readable_thought":"阶段A完成"}`))
			default:
				rsp.EmitOutputStream(bytes.NewBufferString(
					`{"@action":"directly_answer","answer_payload":"# 资金分析报告\n\n## 一、基本情况\n测试报告","human_readable_thought":"输出报告"}`))
			}
			rsp.Close()
			return rsp, nil
		}),
		aicommon.WithEventHandler(func(e *schema.AiOutputEvent) {
			eventsMu.Lock()
			defer eventsMu.Unlock()
			events = append(events, e)
		}),
	)
	require.NoError(t, err)

	loop, err := reactloops.CreateLoopByName(schema.AI_REACT_LOOP_NAME_FUND_ANALYSIS, reactIns)
	require.NoError(t, err)

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	err = loop.Execute("fund-analysis-test", ctx, "分析 /tmp/test_transactions.xlsx 的资金流水")
	require.NoError(t, err)

	require.Equal(t, "true", loop.Get("phase_a_done"))
}

func TestFundAnalysis_ActionFilterBlocksRAG(t *testing.T) {
	reactIns, err := aireact.NewTestReAct(
		aicommon.WithAICallback(func(i aicommon.AICallerConfigIf, req *aicommon.AIRequest) (*aicommon.AIResponse, error) {
			rsp := i.NewAIResponse()
			rsp.EmitOutputStream(bytes.NewBufferString(
				`{"@action":"directly_answer","answer_payload":"done","human_readable_thought":"done"}`))
			rsp.Close()
			return rsp, nil
		}),
	)
	require.NoError(t, err)

	loop, err := reactloops.CreateLoopByName(schema.AI_REACT_LOOP_NAME_FUND_ANALYSIS, reactIns)
	require.NoError(t, err)

	actions := loop.GetAllActionNames()
	for _, name := range actions {
		require.NotEqual(t, "knowledge_enhance_answer", name, "RAG action should not be registered")
		require.NotEqual(t, "search_knowledge", name, "search_knowledge should not be in fund_analysis")
		require.NotEqual(t, "search_persistent_memory", name, "persistent memory search should not be in fund_analysis")
	}
}

func TestFundAnalysis_ExtractDataFilePaths(t *testing.T) {
	reactIns, err := aireact.NewTestReAct(
		aicommon.WithAICallback(func(i aicommon.AICallerConfigIf, req *aicommon.AIRequest) (*aicommon.AIResponse, error) {
			rsp := i.NewAIResponse()
			rsp.EmitOutputStream(bytes.NewBufferString(
				`{"@action":"directly_answer","answer_payload":"test done","human_readable_thought":"done"}`))
			rsp.Close()
			return rsp, nil
		}),
	)
	require.NoError(t, err)

	loop, err := reactloops.CreateLoopByName(schema.AI_REACT_LOOP_NAME_FUND_ANALYSIS, reactIns)
	require.NoError(t, err)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	err = loop.Execute("extract-paths-test", ctx, "分析 /data/kh123个人交易.xlsx 和 /data/对手交易.csv")
	require.NoError(t, err)
}
