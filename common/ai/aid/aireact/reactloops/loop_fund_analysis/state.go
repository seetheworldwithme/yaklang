package loop_fund_analysis

import (
	"fmt"
	"strings"
	"sync"
)

type AnalysisState struct {
	mu sync.RWMutex

	DataFiles    []string
	CurrentPhase string // "init", "A", "B", "C", "D", "report", or custom stage label for non–fund-analysis skills

	SuggestedSkills []string // from InferSuggestedSkills at task start (hints only)

	PhaseADone bool
	PhaseBDone bool
	PhaseCDone bool
	PhaseDDone bool

	// LoadedSkills tracks loading_skills targets seen this task (deduped order).
	LoadedSkills []string

	ProgressEntries []string
	FindingsCount   int
}

func NewAnalysisState() *AnalysisState {
	return &AnalysisState{
		CurrentPhase: "init",
	}
}

func (s *AnalysisState) SetPhase(phase string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.CurrentPhase = phase
}

func (s *AnalysisState) GetPhase() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.CurrentPhase
}

func (s *AnalysisState) AddDataFile(path string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.DataFiles = append(s.DataFiles, path)
}

func (s *AnalysisState) GetDataFiles() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if len(s.DataFiles) == 0 {
		return ""
	}
	return strings.Join(s.DataFiles, ", ")
}

func (s *AnalysisState) MarkPhaseDone(phase string) {
	s.MarkPhaseDoneForSkill("", phase)
}

// MarkPhaseDoneForSkill updates phase flags. For skill fund-analysis (or empty),
// classic A→D advances CurrentPhase; other skills only update CurrentPhase to the given stage token.
func (s *AnalysisState) MarkPhaseDoneForSkill(skillName, phase string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	skill := strings.TrimSpace(skillName)
	if skill == "" {
		skill = SkillFundAnalysis
	}
	p := strings.TrimSpace(phase)

	if skill == SkillFundAnalysis && IsFundClassicPhase(p) {
		switch strings.ToUpper(p) {
		case "A":
			s.PhaseADone = true
			s.CurrentPhase = "B"
		case "B":
			s.PhaseBDone = true
			s.CurrentPhase = "C"
		case "C":
			s.PhaseCDone = true
			s.CurrentPhase = "D"
		case "D":
			s.PhaseDDone = true
			s.CurrentPhase = "report"
		}
		return
	}
	s.CurrentPhase = p
}

func (s *AnalysisState) RememberLoadedSkill(skillName string) {
	name := strings.TrimSpace(skillName)
	if name == "" {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, x := range s.LoadedSkills {
		if x == name {
			return
		}
	}
	s.LoadedSkills = append(s.LoadedSkills, name)
}

func (s *AnalysisState) GetLoadedSkillsLine() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if len(s.LoadedSkills) == 0 {
		return ""
	}
	return strings.Join(s.LoadedSkills, ", ")
}

func (s *AnalysisState) SetSuggestedSkills(ids []string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.SuggestedSkills = append([]string{}, ids...)
}

func (s *AnalysisState) GetSuggestedSkillsLine() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if len(s.SuggestedSkills) == 0 {
		return ""
	}
	return strings.Join(s.SuggestedSkills, ", ")
}

func (s *AnalysisState) AppendProgress(entry string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.ProgressEntries = append(s.ProgressEntries, entry)
	if len(s.ProgressEntries) > 30 {
		s.ProgressEntries = s.ProgressEntries[len(s.ProgressEntries)-30:]
	}
}

func (s *AnalysisState) GetProgressLog() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return strings.Join(s.ProgressEntries, "\n")
}

func (s *AnalysisState) IncrFindings(n int) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.FindingsCount += n
}

func (s *AnalysisState) GetStateSummary() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return fmt.Sprintf("phase=%s A=%v B=%v C=%v D=%v findings=%d files=%d",
		s.CurrentPhase, s.PhaseADone, s.PhaseBDone, s.PhaseCDone, s.PhaseDDone,
		s.FindingsCount, len(s.DataFiles))
}
