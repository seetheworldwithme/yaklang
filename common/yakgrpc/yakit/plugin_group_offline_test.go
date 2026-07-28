package yakit

import (
	"testing"

	"github.com/jinzhu/gorm"
	_ "github.com/mattn/go-sqlite3"
	"github.com/stretchr/testify/require"
	"github.com/yaklang/yaklang/common/schema"
	"github.com/yaklang/yaklang/common/yak/pluginbundle"
)

func TestEnsurePocBuiltInGroupsRemovesLegacyGenericGroups(t *testing.T) {
	db, err := gorm.Open("sqlite3", ":memory:")
	require.NoError(t, err)
	t.Cleanup(func() { _ = db.Close() })
	require.NoError(t, db.AutoMigrate(&schema.YakScript{}, &schema.PluginGroup{}).Error)

	script := &schema.YakScript{
		ScriptName: "Fastjson 综合检测",
		Type:       "mitm",
		Content:    "fastjson java spring iis RCE",
	}
	require.NoError(t, db.Create(script).Error)
	for _, name := range pluginbundle.LegacyGenericPocGroups() {
		group := &schema.PluginGroup{YakScriptName: script.ScriptName, Group: name, IsPocBuiltIn: true}
		group.Hash = group.CalcHash()
		require.NoError(t, db.Create(group).Error)
	}
	stale := &schema.PluginGroup{YakScriptName: script.ScriptName, Group: "Shiro", IsPocBuiltIn: true}
	stale.Hash = stale.CalcHash()
	require.NoError(t, db.Create(stale).Error)

	require.NoError(t, EnsurePocBuiltInGroups(db))
	var groups []*schema.PluginGroup
	require.NoError(t, db.Where("yak_script_name = ?", script.ScriptName).Find(&groups).Error)
	names := make([]string, 0, len(groups))
	for _, group := range groups {
		names = append(names, group.Group)
	}
	require.NotContains(t, names, pluginbundle.OnlineDefaultGroup)
	require.NotContains(t, names, pluginbundle.BuiltinPocGroup)
	require.NotContains(t, names, "Shiro")
	require.Contains(t, names, "FastJSON")
	require.Contains(t, names, "Java")
	require.Contains(t, names, "Spring")
	require.Contains(t, names, "IIS")
	require.Contains(t, names, "远程代码执行（扫描）")
}

func TestQueryGroupCountHidesLegacyGenericPocGroup(t *testing.T) {
	db, err := gorm.Open("sqlite3", ":memory:")
	require.NoError(t, err)
	t.Cleanup(func() { _ = db.Close() })
	require.NoError(t, db.AutoMigrate(&schema.YakScript{}, &schema.PluginGroup{}).Error)

	script := &schema.YakScript{ScriptName: "cloud-poc", Type: "mitm"}
	require.NoError(t, db.Create(script).Error)
	legacy := &schema.PluginGroup{
		YakScriptName: script.ScriptName,
		Group:         pluginbundle.OnlineDefaultGroup,
		IsPocBuiltIn:  true,
	}
	legacy.Hash = legacy.CalcHash()
	require.NoError(t, db.Create(legacy).Error)

	groups, err := QueryGroupCount(db, []string{"yak", "codec", "lua"}, 2)
	require.NoError(t, err)
	for _, group := range groups {
		require.NotEqual(t, pluginbundle.OnlineDefaultGroup, group.Value)
		require.NotEqual(t, pluginbundle.BuiltinPocGroup, group.Value)
	}
}

func TestEnsurePocBuiltInGroupsMigratesOfflineImportToTagGroups(t *testing.T) {
	db, err := gorm.Open("sqlite3", ":memory:")
	require.NoError(t, err)
	t.Cleanup(func() { _ = db.Close() })
	require.NoError(t, db.AutoMigrate(&schema.YakScript{}, &schema.PluginGroup{}).Error)

	script := &schema.YakScript{
		ScriptName: "SSH 常见弱口令检查（10组以内）",
		Type:       "port-scan",
		Tags:       "主流第三方服务,SSH",
	}
	require.NoError(t, db.Create(script).Error)
	offline := &schema.PluginGroup{
		YakScriptName: script.ScriptName,
		Group:         pluginbundle.OfflineImportGroup,
		IsPocBuiltIn:  true,
	}
	offline.Hash = offline.CalcHash()
	require.NoError(t, db.Create(offline).Error)

	require.NoError(t, EnsurePocBuiltInGroups(db))
	var groups []*schema.PluginGroup
	require.NoError(t, db.Where("yak_script_name = ?", script.ScriptName).Find(&groups).Error)
	names := make([]string, 0, len(groups))
	for _, group := range groups {
		names = append(names, group.Group)
	}
	require.Contains(t, names, "主流第三方服务")
	require.Contains(t, names, "SSH")
	require.NotContains(t, names, pluginbundle.OfflineImportGroup)
}

func TestGetPluginGroupsByScriptNamesAndMergeTags(t *testing.T) {
	db, err := gorm.Open("sqlite3", ":memory:")
	require.NoError(t, err)
	t.Cleanup(func() { _ = db.Close() })
	require.NoError(t, db.AutoMigrate(&schema.PluginGroup{}).Error)

	groups := []*schema.PluginGroup{
		{YakScriptName: "SQL注入-MySQL-ErrorBased", Group: "SQL注入"},
		{YakScriptName: "SQL注入-MySQL-ErrorBased", Group: "安全产品"},
		{YakScriptName: "SQL注入-MySQL-ErrorBased", Group: pluginbundle.OfflineImportGroup},
	}
	for _, group := range groups {
		group.Hash = group.CalcHash()
		require.NoError(t, db.Create(group).Error)
	}

	groupMap, err := GetPluginGroupsByScriptNames(db, []string{"SQL注入-MySQL-ErrorBased"})
	require.NoError(t, err)
	require.Equal(t, []string{"SQL注入", "安全产品"}, groupMap["SQL注入-MySQL-ErrorBased"])
	require.Equal(t, "原生插件,SQL注入,安全产品", MergePluginTagsAndGroups(
		`["原生插件","SQL注入"]`,
		groupMap["SQL注入-MySQL-ErrorBased"],
	))
}
