package coreplugin

import (
	"testing"
)

// TestCommunityBuildInPluginsReadable 校验社区内置插件映射表与 embed FS 的一致性(不依赖数据库)：
// 501 个条目、名字唯一、每个 FileName 都能读到非空内容
func TestCommunityBuildInPluginsReadable(t *testing.T) {
	if len(communityBuildInPluginMetas) != 501 {
		t.Fatalf("community buildin plugin count = %d, want 501", len(communityBuildInPluginMetas))
	}
	seen := make(map[string]bool)
	for _, meta := range communityBuildInPluginMetas {
		if meta.ScriptName == "" {
			t.Fatalf("empty script name for file %v", meta.FileName)
		}
		if seen[meta.ScriptName] {
			t.Fatalf("duplicate script name: %v", meta.ScriptName)
		}
		seen[meta.ScriptName] = true
		codeBytes, err := basePluginFS.ReadFile("base-yak-plugin/" + meta.FileName)
		if err != nil {
			t.Fatalf("read community buildin plugin [%v] failed: %v", meta.FileName, err)
		}
		if len(codeBytes) == 0 {
			t.Fatalf("community buildin plugin [%v] is empty", meta.FileName)
		}
	}
}
