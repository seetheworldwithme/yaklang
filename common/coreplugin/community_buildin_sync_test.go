package coreplugin_test

import (
	"path/filepath"
	"testing"

	"github.com/yaklang/yaklang/common/consts"
	"github.com/yaklang/yaklang/common/coreplugin"
	"github.com/yaklang/yaklang/common/schema"
)

// TestCommunityBuildInPluginSyncE2E 绑定临时 profile 库后强制同步内置插件，
// 验证 501 个社区内置插件全部落库：上传者为 Admin、is_core_plugin=true、内容非空。
func TestCommunityBuildInPluginSyncE2E(t *testing.T) {
	dbPath := filepath.Join(t.TempDir(), "profile.db")
	db, err := consts.CreateProfileDatabase(dbPath)
	if err != nil {
		t.Fatalf("create temp profile db: %v", err)
	}
	consts.BindProfileDatabase(db, dbPath)

	if err := coreplugin.ForceSyncCorePlugin(); err != nil {
		t.Fatalf("force sync core plugin: %v", err)
	}

	var total, coreCount, adminCount, nonEmpty, visible int64
	if err := db.Model(&schema.YakScript{}).Count(&total).Error; err != nil {
		t.Fatalf("count yak scripts: %v", err)
	}
	if err := db.Model(&schema.YakScript{}).Where("is_core_plugin = ?", true).Count(&coreCount).Error; err != nil {
		t.Fatalf("count core plugins: %v", err)
	}
	if err := db.Model(&schema.YakScript{}).Where("is_core_plugin = ? AND author = ?", true, "Admin").Count(&adminCount).Error; err != nil {
		t.Fatalf("count admin plugins: %v", err)
	}
	if err := db.Model(&schema.YakScript{}).Where("is_core_plugin = ? AND length(content) > 0", true).Count(&nonEmpty).Error; err != nil {
		t.Fatalf("count non-empty plugins: %v", err)
	}
	if err := db.Model(&schema.YakScript{}).Where("is_core_plugin = ? AND ignored = ?", true, false).Count(&visible).Error; err != nil {
		t.Fatalf("count visible plugins: %v", err)
	}

	// 49 官方内置 + 501 社区内置 = 550；其中 ignored=true 的 11 个官方插件不在插件仓库展示
	if coreCount != 550 {
		t.Fatalf("core plugin count = %d, want 550 (49 official + 501 community)", coreCount)
	}
	// 插件仓库可见: (49-11) + 501 = 539，与「539个插件」包一致
	if visible != 539 {
		t.Fatalf("visible plugin count = %d, want 539", visible)
	}
	if adminCount != coreCount {
		t.Fatalf("author=Admin count = %d, want %d (全部内置插件上传者应为 Admin)", adminCount, coreCount)
	}
	if nonEmpty != coreCount {
		t.Fatalf("non-empty content count = %d, want %d", nonEmpty, coreCount)
	}
	t.Logf("total=%d core=%d admin=%d nonEmpty=%d visible=%d", total, coreCount, adminCount, nonEmpty, visible)
}
