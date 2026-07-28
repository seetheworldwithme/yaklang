package yakit

import (
	"context"
	"encoding/json"
	"strings"

	"github.com/yaklang/gorm"
	"github.com/yaklang/yaklang/common/consts"
	"github.com/yaklang/yaklang/common/log"
	"github.com/yaklang/yaklang/common/schema"
	"github.com/yaklang/yaklang/common/utils"
	"github.com/yaklang/yaklang/common/utils/bizhelper"
	"github.com/yaklang/yaklang/common/yak/pluginbundle"
	"github.com/yaklang/yaklang/common/yakgrpc/ypb"
)

func GetPluginGroupsByScriptNames(db *gorm.DB, scriptNames []string) (map[string][]string, error) {
	result := make(map[string][]string)
	if db == nil || len(scriptNames) == 0 {
		return result, nil
	}

	var groups []*schema.PluginGroup
	hiddenGroups := append(pluginbundle.LegacyGenericPocGroups(), pluginbundle.OfflineImportGroup)
	query := db.Model(&schema.PluginGroup{}).
		Where("yak_script_name IN (?)", scriptNames).
		Where("`group` NOT IN (?)", hiddenGroups).
		Order("yak_script_name asc, `group` asc")
	if err := query.Find(&groups).Error; err != nil {
		return nil, utils.Wrap(err, "query plugin groups by script names failed")
	}

	seen := make(map[string]map[string]struct{})
	for _, group := range groups {
		if group == nil {
			continue
		}
		scriptName := strings.TrimSpace(group.YakScriptName)
		groupName := strings.TrimSpace(group.Group)
		if scriptName == "" || groupName == "" {
			continue
		}
		if seen[scriptName] == nil {
			seen[scriptName] = make(map[string]struct{})
		}
		if _, ok := seen[scriptName][groupName]; ok {
			continue
		}
		seen[scriptName][groupName] = struct{}{}
		result[scriptName] = append(result[scriptName], groupName)
	}
	return result, nil
}

func MergePluginTagsAndGroups(rawTags string, groups []string) string {
	var tags []string
	normalizedTags := strings.TrimSpace(rawTags)
	if normalizedTags != "" {
		if err := json.Unmarshal([]byte(normalizedTags), &tags); err != nil {
			tags = strings.FieldsFunc(normalizedTags, func(r rune) bool {
				return r == ',' || r == '，'
			})
		}
	}

	merged := make([]string, 0, len(tags)+len(groups))
	seen := make(map[string]struct{})
	for _, value := range append(tags, groups...) {
		value = strings.TrimSpace(value)
		if value == "" {
			continue
		}
		key := strings.ToLower(value)
		if _, ok := seen[key]; ok {
			continue
		}
		seen[key] = struct{}{}
		merged = append(merged, value)
	}
	return strings.Join(merged, ",")
}

var pocBuiltInGroups = map[string]string{
	"ThinkPHP":      "thinkphp",
	"Shiro":         "shiro",
	"FastJSON":      "fastjson",
	"Struts":        "struts",
	"Tomcat":        "tomcat",
	"Weblogic":      "weblogic",
	"Spring":        "spring,springboot,springcloud,springframework",
	"Jenkins":       "jenkins",
	"IIS":           "iis",
	"ElasticSearch": "elastic",
	"致远 OA":         "seeyou,seeyon,zhiyuan",
	"Exchange":      "exchange",
	"通达 OA":         "tongda",
	"PhpMyAdmin":    "phpmyadmin",
	"Nexus":         "nexus",
	"Laravel":       "laravel",
	"JBoss":         "jboss",
	"ColdFusion":    "coldfusion",
	"ActiveMQ":      "activemq",
	"Wordpress":     "wordpress",
	"Java":          "java",
	"PHP":           "php",
	"Python":        "python",
	"Nginx":         "nginx",

	"网络设备与OA系统":  "锐捷,若依,金和,金山,金蝶,致远,Seeyou,seeyou,通达,tonged,Tongda,银澎,浪潮,泛微,方维,帆软,向日葵,ecshop,dahua,huawei,zimbra,coremail,Coremail,邮件服务器,",
	"安全产品":       "防火墙,行为管理,绿盟,天擎,tianqing,防篡改,网御星云,安防,审计系统,天融信,安全系统",
	"Log4j":      "Log4j,log4j,Log4shell,log4shell,Log4Shell",
	"远程代码执行（扫描）": "RCE,rce",
	"XSS":        "xss,XSS",
	"SQL注入":      "sql注入",
}

func init() {
	RegisterPostInitDatabaseFunction(func() error {
		defer func() {
			if err := recover(); err != nil {
				log.Errorf("DeletePluginGroupsWithNonEmptyTemporaryId failed: %s", err)
			}
		}()
		if db := consts.GetGormProfileDatabase(); db != nil {
			err := DeletePluginGroupsWithNonEmptyTemporaryId(db)
			if err != nil {
				return err
			}
			return EnsurePocBuiltInGroups(db)
		}
		return nil
	})
}

func EnsurePocBuiltInGroups(db *gorm.DB) error {
	if db == nil {
		return utils.Error("empty database")
	}
	if err := db.Model(&schema.PluginGroup{}).
		Where("`group` IN (?)", pluginbundle.LegacyGenericPocGroups()).
		Unscoped().Delete(&schema.PluginGroup{}).Error; err != nil {
		return utils.Wrap(err, "delete legacy generic POC groups failed")
	}
	if err := migrateOfflineImportGroupsFromTags(db); err != nil {
		return err
	}
	bundledNames := pluginbundle.BundledPocScriptNames()
	if err := db.Model(&schema.PluginGroup{}).
		Where("yak_script_name IN (?) AND is_poc_built_in = ?", bundledNames, true).
		Unscoped().Delete(&schema.PluginGroup{}).Error; err != nil {
		return utils.Wrap(err, "delete stale bundled POC groups failed")
	}
	for _, scriptName := range bundledNames {
		var count int
		if err := db.Model(&schema.YakScript{}).Where("script_name = ?", scriptName).Count(&count).Error; err != nil {
			return utils.Wrapf(err, "check bundled POC [%s] failed", scriptName)
		}
		if count == 0 {
			continue
		}
		for _, group := range pluginbundle.BundledPocGroups(scriptName) {
			saveData := &schema.PluginGroup{YakScriptName: scriptName, Group: group.Name, IsPocBuiltIn: true}
			saveData.Hash = saveData.CalcHash()
			if err := CreateOrUpdatePluginGroup(db, saveData.Hash, saveData); err != nil {
				return utils.Wrapf(err, "save bundled YakScriptGroup [%s] [%s] failed", scriptName, group.Name)
			}
		}
	}
	scriptDB := db.Model(&schema.YakScript{})
	for group, keywords := range pocBuiltInGroups {
		filterDB := FilterYakScript(scriptDB, &ypb.QueryYakScriptRequest{Keyword: keywords})
		yakScripts := bizhelper.YieldModel[*schema.YakScript](context.Background(), filterDB)
		for yakScript := range yakScripts {
			if yakScript == nil || yakScript.ScriptName == "" || pluginbundle.IsBundledPoc(yakScript.ScriptName) {
				continue
			}
			saveData := &schema.PluginGroup{
				YakScriptName: yakScript.ScriptName,
				Group:         group,
				IsPocBuiltIn:  true,
			}
			saveData.Hash = saveData.CalcHash()
			if err := CreateOrUpdatePluginGroup(db, saveData.Hash, saveData); err != nil {
				return utils.Wrapf(err, "save YakScriptGroup [%s] [%s] failed", yakScript.ScriptName, group)
			}
		}
	}
	return nil
}

func migrateOfflineImportGroupsFromTags(db *gorm.DB) error {
	var offlineGroups []*schema.PluginGroup
	if err := db.Model(&schema.PluginGroup{}).
		Where("`group` = ?", pluginbundle.OfflineImportGroup).
		Find(&offlineGroups).Error; err != nil {
		return utils.Wrap(err, "query offline import groups failed")
	}

	for _, offlineGroup := range offlineGroups {
		if offlineGroup == nil || offlineGroup.YakScriptName == "" {
			continue
		}
		var script schema.YakScript
		if err := db.Where("script_name = ?", offlineGroup.YakScriptName).First(&script).Error; err != nil {
			if gorm.IsRecordNotFoundError(err) {
				continue
			}
			return utils.Wrapf(err, "query offline plugin [%s] failed", offlineGroup.YakScriptName)
		}
		if !pluginbundle.IsPocPluginType(script.Type) {
			continue
		}
		tagGroups := pluginbundle.GroupsFromTags(script.Tags)
		if len(tagGroups) == 0 {
			continue
		}
		for _, group := range tagGroups {
			saveData := &schema.PluginGroup{
				YakScriptName: script.ScriptName,
				Group:         group.Name,
				IsPocBuiltIn:  true,
			}
			saveData.Hash = saveData.CalcHash()
			if err := CreateOrUpdatePluginGroup(db, saveData.Hash, saveData); err != nil {
				return utils.Wrapf(err, "migrate offline plugin group [%s] [%s] failed", script.ScriptName, group.Name)
			}
		}
		if err := db.Model(&schema.PluginGroup{}).
			Where("id = ?", offlineGroup.ID).
			Unscoped().Delete(&schema.PluginGroup{}).Error; err != nil {
			return utils.Wrapf(err, "delete offline import group [%s] failed", script.ScriptName)
		}
	}
	return nil
}

func CreateOrUpdatePluginGroup(db *gorm.DB, hash string, i interface{}) error {
	yakScriptOpLock.Lock()
	db = db.Model(&schema.PluginGroup{})
	if db := db.Where("hash = ?", hash).Assign(i).FirstOrCreate(&schema.PluginGroup{}); db.Error != nil {
		return utils.Errorf("create/update PluginGroup failed: %s", db.Error)
	}
	yakScriptOpLock.Unlock()
	return nil
}

func DeletePluginGroupByHash(db *gorm.DB, hash string) error {
	db = db.Model(&schema.PluginGroup{}).Where("hash = ?", hash).Unscoped().Delete(&schema.PluginGroup{})
	if db.Error != nil {
		return db.Error
	}
	return nil
}

func DeletePluginGroupsWithNonEmptyTemporaryId(db *gorm.DB) error {
	db = db.Model(&schema.PluginGroup{}).Where("temporary_id != ''").Unscoped().Delete(&schema.PluginGroup{})
	if db.Error != nil {
		return db.Error
	}
	return nil
}

func GetPluginByGroup(db *gorm.DB, group string) (req []*schema.PluginGroup, err error) {
	db = db.Model(&schema.PluginGroup{}).Where("`group` = ?", group).Scan(&req)
	if db.Error != nil {
		return nil, db.Error
	}
	return req, nil
}

func DeletePluginGroup(db *gorm.DB, group string) error {
	db = db.Model(&schema.PluginGroup{})
	if group != "" {
		db = db.Where(" `group` = ?", group)
	}
	db = db.Unscoped().Delete(&schema.PluginGroup{})
	if db.Error != nil {
		return db.Error
	}
	return nil
}

func GroupCount(db *gorm.DB) (req []*TagAndTypeValue, err error) {
	db = db.Model(&schema.PluginGroup{}).Select(" `group` as value, count(*) as count, `temporary_id` as temporary_id, `is_poc_built_in` as is_poc_built_in")
	db = db.Joins("INNER JOIN yak_scripts Y on Y.script_name = plugin_groups.yak_script_name ")
	//db = db.Where("yak_script_name IN (SELECT DISTINCT(script_name) FROM yak_scripts)")
	db = db.Group(" `group`,`temporary_id`,`is_poc_built_in` ").Order(`count desc`).Scan(&req)
	if db.Error != nil {
		return nil, utils.Wrap(db.Error, "GroupCount failed")
	}

	return req, nil
}

func GetGroup(db *gorm.DB, scriptNames []string) (req []*schema.PluginGroup, err error) {
	db = db.Model(&schema.PluginGroup{}).Select(" DISTINCT(`group`)")
	if len(scriptNames) > 0 {
		db = db.Joins("inner join yak_scripts Y on Y.script_name = plugin_groups.yak_script_name ")
		db = bizhelper.ExactQueryStringArrayOr(db, "plugin_groups.yak_script_name", scriptNames)
		db = db.Where("is_poc_built_in = false")
		db = db.Group(" `group` ").Having("COUNT(DISTINCT yak_script_name) = ?", len(scriptNames))
		db = db.Scan(&req)
	}
	if db.Error != nil {
		return nil, utils.Errorf("GetGroup failed: %s", db.Error)
	}

	return req, nil
}

func DeletePluginGroupByScriptName(db *gorm.DB, scriptName []string) error {
	db = db.Model(&schema.PluginGroup{})
	db = bizhelper.ExactQueryStringArrayOr(db, "yak_script_name", scriptName).Unscoped().Delete(&schema.PluginGroup{})
	if db.Error != nil {
		return db.Error
	}
	return nil
}

func QueryGroupCount(db *gorm.DB, excludeType []string, isMITMParamPlugins int64) (req []*TagAndTypeValue, err error) {
	db = db.Model(&schema.PluginGroup{}).Select(" `group` as value, COUNT(Y.script_name) as count, `temporary_id` as temporary_id, `is_poc_built_in` as is_poc_built_in")
	db = db.Joins("LEFT JOIN yak_scripts Y on Y.script_name = plugin_groups.yak_script_name ")
	db = db.Where("plugin_groups.`group` NOT IN (?)", pluginbundle.LegacyGenericPocGroups())
	db = bizhelper.ExactQueryExcludeStringArrayOr(db, "Y.type", excludeType)
	switch isMITMParamPlugins {
	case 1:
		db = db.Where(mitmHasParamsCondition("Y.params"))
	case 2:
		db = db.Where("(" + mitmEmptyParamsCondition("Y.params") + ") or Y.type!='mitm'")
	}
	db = db.Group(" `group`,`temporary_id`,`is_poc_built_in` ").Order(`count desc`).Scan(&req)
	if db.Error != nil {
		return nil, utils.Wrap(db.Error, "GroupCount failed")
	}

	return req, nil
}
