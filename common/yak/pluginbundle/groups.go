package pluginbundle

import (
	"encoding/json"
	"strings"

	"github.com/yaklang/yaklang/common/schema"
)

const (
	OfflineImportGroup = "离线导入"
	// OnlineDefaultGroup and BuiltinPocGroup are legacy generic groups kept for
	// migration compatibility. New bundles must use the real classifications.
	OnlineDefaultGroup = "企业默认漏洞插件"
	BuiltinPocGroup    = "内置漏洞插件"
)

var bundledPocGroups = map[string][]string{
	"Fastjson 综合检测":          {"FastJSON", "IIS", "Java", "Spring", "远程代码执行（扫描）"},
	"HTTP请求走私":               {"安全产品"},
	"SQL注入-MySQL-ErrorBased": {"SQL注入", "安全产品"},
	"SQL注入-Path参数注入":         {"Java", "SQL注入"},
	"SQL注入-UNION注入-MD5函数":    {"Java", "SQL注入"},
	"SQL注入-堆叠注入":             {"SQL注入"},
	"SQL注入-时间盲注-Sleep":       {"SQL注入", "安全产品"},
	"SQL注入-高危Header注入":       {"Java", "SQL注入"},
	"SSRF HTTP Public":       {"安全产品", "远程代码执行（扫描）"},
	"SSTI Expr 服务器模版表达式注入":   {"Java", "PHP"},
	"Shiro 指纹识别 + 弱密码检测":     {"Java", "Shiro", "远程代码执行（扫描）"},
	"Shiro 自定义检测":            {"Java", "远程代码执行（扫描）"},
	"Swagger JSON 泄漏":        {"安全产品"},
	"基础 XSS 检测":              {"Java", "PHP", "XSS"},
	"多认证综合越权测试":              {"PHP"},
	"开放 URL 重定向漏洞":           {"远程代码执行（扫描）"},
	"文件包含":                   {"XSS"},
}

type Group struct {
	Name         string `json:"name"`
	IsPocBuiltIn bool   `json:"is_poc_built_in"`
}

type Metadata struct {
	Filename   string   `json:"filename"`
	ScriptName string   `json:"script_name"`
	Groups     *[]Group `json:"groups,omitempty"`
}

func NormalizeGroups(script *schema.YakScript, groups []Group, fallbackGroup string) []*schema.PluginGroup {
	if script == nil || strings.TrimSpace(script.ScriptName) == "" {
		return nil
	}

	isPoc := IsPocPluginType(script.Type)
	if isPoc && len(groups) > 0 {
		filtered := make([]Group, 0, len(groups))
		for _, group := range groups {
			if IsLegacyGenericPocGroup(group.Name) {
				continue
			}
			filtered = append(filtered, group)
		}
		groups = filtered
	}
	if len(groups) == 0 && isPoc {
		groups = BundledPocGroups(script.ScriptName)
	}
	if len(groups) == 0 && isPoc {
		groups = GroupsFromTags(script.Tags)
	}
	if len(groups) == 0 && isPoc && strings.TrimSpace(fallbackGroup) != "" {
		groups = []Group{{Name: fallbackGroup, IsPocBuiltIn: true}}
	}

	seen := make(map[string]struct{}, len(groups))
	result := make([]*schema.PluginGroup, 0, len(groups))
	for _, group := range groups {
		name := strings.TrimSpace(group.Name)
		if name == "" {
			continue
		}
		if _, ok := seen[name]; ok {
			continue
		}
		seen[name] = struct{}{}

		item := &schema.PluginGroup{
			YakScriptName: script.ScriptName,
			Group:         name,
			IsPocBuiltIn:  group.IsPocBuiltIn || isPoc,
		}
		item.Hash = item.CalcHash()
		result = append(result, item)
	}
	return result
}

func GroupsFromTags(rawTags string) []Group {
	rawTags = strings.TrimSpace(rawTags)
	if rawTags == "" {
		return nil
	}

	var names []string
	if err := json.Unmarshal([]byte(rawTags), &names); err != nil {
		names = strings.FieldsFunc(rawTags, func(r rune) bool {
			switch r {
			case ',', '，', ';', '；', '\n', '\r':
				return true
			default:
				return false
			}
		})
	}

	seen := make(map[string]struct{}, len(names))
	result := make([]Group, 0, len(names))
	for _, name := range names {
		name = strings.TrimSpace(name)
		if name == "" || IsLegacyGenericPocGroup(name) {
			continue
		}
		if _, ok := seen[name]; ok {
			continue
		}
		seen[name] = struct{}{}
		result = append(result, Group{Name: name, IsPocBuiltIn: true})
	}
	return result
}

func BundledPocGroups(scriptName string) []Group {
	names := bundledPocGroups[strings.TrimSpace(scriptName)]
	result := make([]Group, 0, len(names))
	for _, name := range names {
		result = append(result, Group{Name: name, IsPocBuiltIn: true})
	}
	return result
}

func BundledPocScriptNames() []string {
	result := make([]string, 0, len(bundledPocGroups))
	for name := range bundledPocGroups {
		result = append(result, name)
	}
	return result
}

func IsBundledPoc(scriptName string) bool {
	_, ok := bundledPocGroups[strings.TrimSpace(scriptName)]
	return ok
}

func LegacyGenericPocGroups() []string {
	return []string{OnlineDefaultGroup, BuiltinPocGroup}
}

func IsLegacyGenericPocGroup(name string) bool {
	switch strings.TrimSpace(name) {
	case OnlineDefaultGroup, BuiltinPocGroup:
		return true
	default:
		return false
	}
}

func IsPocPluginType(pluginType string) bool {
	switch strings.ToLower(strings.TrimSpace(pluginType)) {
	case "mitm", "nuclei", "port-scan":
		return true
	default:
		return false
	}
}

func FromSchema(groups []*schema.PluginGroup) []Group {
	result := make([]Group, 0, len(groups))
	for _, group := range groups {
		if group == nil || strings.TrimSpace(group.Group) == "" {
			continue
		}
		result = append(result, Group{
			Name:         group.Group,
			IsPocBuiltIn: group.IsPocBuiltIn,
		})
	}
	return result
}
