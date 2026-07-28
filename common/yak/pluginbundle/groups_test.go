package pluginbundle

import (
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/yaklang/yaklang/common/schema"
)

func TestNormalizeGroupsAddsVisibleFallbackForOfflinePoc(t *testing.T) {
	groups := NormalizeGroups(&schema.YakScript{
		ScriptName: "offline-poc",
		Type:       "mitm",
	}, nil, OfflineImportGroup)

	require.Len(t, groups, 1)
	require.Equal(t, OfflineImportGroup, groups[0].Group)
	require.True(t, groups[0].IsPocBuiltIn)
	require.NotEmpty(t, groups[0].Hash)
}

func TestNormalizeGroupsUsesPluginTagsBeforeOfflineFallback(t *testing.T) {
	groups := NormalizeGroups(&schema.YakScript{
		ScriptName: "SSH 常见弱口令检查（10组以内）",
		Type:       "port-scan",
		Tags:       "主流第三方服务,SSH",
	}, nil, OfflineImportGroup)

	names := make([]string, 0, len(groups))
	for _, group := range groups {
		names = append(names, group.Group)
	}
	require.ElementsMatch(t, []string{"主流第三方服务", "SSH"}, names)
	require.NotContains(t, names, OfflineImportGroup)
}

func TestNormalizeGroupsPreservesGroupsAndMakesPocVisible(t *testing.T) {
	groups := NormalizeGroups(&schema.YakScript{
		ScriptName: "server-poc",
		Type:       "nuclei",
	}, []Group{{Name: "Spring"}, {Name: "Spring"}}, OnlineDefaultGroup)

	require.Len(t, groups, 1)
	require.Equal(t, "Spring", groups[0].Group)
	require.True(t, groups[0].IsPocBuiltIn)
}

func TestNormalizeGroupsDropsLegacyCloudGroup(t *testing.T) {
	groups := NormalizeGroups(&schema.YakScript{
		ScriptName: "Fastjson 综合检测",
		Type:       "mitm",
	}, []Group{
		{Name: OnlineDefaultGroup, IsPocBuiltIn: true},
		{Name: "FastJSON", IsPocBuiltIn: true},
	}, "")

	names := make([]string, 0, len(groups))
	for _, group := range groups {
		names = append(names, group.Group)
	}
	require.NotContains(t, names, OnlineDefaultGroup)
	require.Contains(t, names, "FastJSON")
}

func TestNormalizeGroupsUsesRealOfflineClassificationForBundledPoc(t *testing.T) {
	groups := NormalizeGroups(&schema.YakScript{
		ScriptName: "Fastjson 综合检测",
		Type:       "mitm",
	}, nil, BuiltinPocGroup)

	names := make([]string, 0, len(groups))
	for _, group := range groups {
		names = append(names, group.Group)
	}
	require.ElementsMatch(t, []string{"FastJSON", "IIS", "Java", "Spring", "远程代码执行（扫描）"}, names)
	require.NotContains(t, names, OnlineDefaultGroup)
	require.NotContains(t, names, BuiltinPocGroup)
}

func TestBundledPocClassificationMatchesOnlineDatasetShape(t *testing.T) {
	require.Len(t, bundledPocGroups, 17)
	groupNames := make(map[string]struct{})
	groupCounts := make(map[string]int)
	for _, groups := range bundledPocGroups {
		for _, group := range groups {
			groupNames[group] = struct{}{}
			groupCounts[group]++
		}
	}
	require.Len(t, groupNames, 10)
	require.Equal(t, map[string]int{
		"FastJSON": 1, "IIS": 1, "Java": 8, "PHP": 3, "Shiro": 1,
		"Spring": 1, "SQL注入": 6, "XSS": 2, "安全产品": 5, "远程代码执行（扫描）": 5,
	}, groupCounts)
}

func TestNormalizeGroupsDoesNotPutUtilityPluginOnPocPage(t *testing.T) {
	groups := NormalizeGroups(&schema.YakScript{
		ScriptName: "utility",
		Type:       "codec",
	}, nil, OfflineImportGroup)

	require.Empty(t, groups)
}
