package yakgrpc

import (
	"archive/zip"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path"
	"testing"

	"github.com/google/uuid"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"github.com/tidwall/gjson"
	"github.com/yaklang/yaklang/common/consts"
	"github.com/yaklang/yaklang/common/log"
	"github.com/yaklang/yaklang/common/schema"
	"github.com/yaklang/yaklang/common/utils"
	"github.com/yaklang/yaklang/common/yakgrpc/yakit"
	"github.com/yaklang/yaklang/common/yakgrpc/ypb"
)

func TestServerExportsPlugins(t *testing.T) {
	client, _ := NewLocalClient()
	uid := uuid.New().String()

	name1, clearFunc, err := yakit.CreateAndClearTemporaryYakScript("yak", "hello 1; "+uid, uid)
	require.NoError(t, err)
	defer clearFunc()
	name2, clearFunc2, err := yakit.CreateAndClearTemporaryYakScript("yak", "hello 2; "+uid, uid)
	require.NoError(t, err)
	defer clearFunc2()
	stream, err := client.ExportYakScriptStream(
		context.Background(),
		&ypb.ExportYakScriptStreamRequest{
			Filter: &ypb.QueryYakScriptRequest{
				Keyword:  uid,
				IsIgnore: true,
			},
			OutputFilename: "",
			Password:       "",
		},
	)
	if err != nil {
		t.Fatal(err)
	}

	outputFile := ""
	for {
		client, err := stream.Recv()
		if err != nil {
			break
		}
		if client.IsMessage {
			data := gjson.ParseBytes(client.Message).Get("content").Get("data")
			pathName := gjson.Parse(data.Str).Get("path").Str
			if pathName != "" {
				outputFile = pathName
			}
		}
	}
	if outputFile == "" {
		t.Fatal("output file is empty")
	}
	if utils.GetFirstExistedFile(outputFile) == "" {
		t.Fatal("output file not found")
	}

	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name1)
	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name2)

	stream2, err := client.ImportYakScriptStream(context.Background(), &ypb.ImportYakScriptStreamRequest{
		Filename: outputFile,
	})
	for {
		client, err := stream2.Recv()
		if err != nil {
			break
		}
		if client.IsMessage {
			log.Infof("message: %s", client.Message)
		}
	}
	t1, _ := yakit.GetYakScriptByName(consts.GetGormProfileDatabase(), name1)
	t2, _ := yakit.GetYakScriptByName(consts.GetGormProfileDatabase(), name2)
	assert.NotNil(t, t1)
	assert.NotNil(t, t2)

	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name1)
	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name2)
}

func TestServerExportsPlugins_PreservesPocGroups(t *testing.T) {
	client, _ := NewLocalClient()
	uid := uuid.NewString()
	name, clearFunc, err := yakit.CreateAndClearTemporaryYakScript("mitm", "mirrorHTTPFlow() // "+uid, uid)
	require.NoError(t, err)
	t.Cleanup(clearFunc)

	db := consts.GetGormProfileDatabase()
	group := &schema.PluginGroup{
		YakScriptName: name,
		Group:         "offline-import-" + uid,
		IsPocBuiltIn:  true,
	}
	group.Hash = group.CalcHash()
	require.NoError(t, yakit.CreateOrUpdatePluginGroup(db, group.Hash, group))
	t.Cleanup(func() {
		db.Unscoped().Where("yak_script_name = ?", name).Delete(&schema.PluginGroup{})
	})

	stream, err := client.ExportYakScriptStream(context.Background(), &ypb.ExportYakScriptStreamRequest{
		Filter: &ypb.QueryYakScriptRequest{
			IncludedScriptNames: []string{name},
			IsIgnore:            true,
		},
		OutputPluginDir: t.TempDir(),
	})
	require.NoError(t, err)

	var outputFile string
	for {
		message, recvErr := stream.Recv()
		if recvErr != nil {
			break
		}
		if message.IsMessage {
			data := gjson.ParseBytes(message.Message).Get("content").Get("data")
			if pathName := gjson.Parse(data.Str).Get("path").Str; pathName != "" {
				outputFile = pathName
			}
		}
	}
	require.FileExists(t, outputFile)

	require.NoError(t, yakit.DeleteYakScriptByName(db, name))
	require.NoError(t, yakit.DeletePluginGroupByScriptName(db, []string{name}))

	importStream, err := client.ImportYakScriptStream(context.Background(), &ypb.ImportYakScriptStreamRequest{
		Filename: outputFile,
	})
	require.NoError(t, err)
	for {
		_, recvErr := importStream.Recv()
		if recvErr != nil {
			break
		}
	}

	var importedGroup schema.PluginGroup
	err = db.Where("yak_script_name = ? AND `group` = ?", name, group.Group).First(&importedGroup).Error
	require.NoError(t, err)
	require.True(t, importedGroup.IsPocBuiltIn)
}

func TestServerExportsPlugins_CustomDir(t *testing.T) {
	client, _ := NewLocalClient()
	uid := uuid.New().String()

	_, clearFunc, err := yakit.CreateAndClearTemporaryYakScript("yak", "hello 1; "+uid, uid)
	require.NoError(t, err)
	defer clearFunc()
	tmpdir := t.TempDir()
	fileName := uuid.New().String()
	stream, err := client.ExportYakScriptStream(
		context.Background(),
		&ypb.ExportYakScriptStreamRequest{
			Filter: &ypb.QueryYakScriptRequest{
				Keyword:  uid,
				IsIgnore: true,
			},
			OutputFilename:  fileName,
			Password:        "",
			OutputPluginDir: tmpdir,
		},
	)
	if err != nil {
		t.Fatal(err)
	}

	outputFile := ""
	for {
		client, err := stream.Recv()
		if err != nil {
			break
		}
		if client.IsMessage {
			data := gjson.ParseBytes(client.Message).Get("content").Get("data")
			pathName := gjson.Parse(data.Str).Get("path").Str
			if pathName != "" {
				outputFile = pathName
			}
		}
	}
	if outputFile == "" {
		t.Fatal("output file is empty")
	}
	if utils.GetFirstExistedFile(outputFile) == "" {
		t.Fatal("output file not found")
	}
	require.Equal(t, path.Join(tmpdir, fileName)+".zip", outputFile)
	require.FileExists(t, outputFile)
}

func TestServerExportsPlugins_Enc(t *testing.T) {
	client, _ := NewLocalClient()
	uid := uuid.New().String()

	name1, clearFunc, err := yakit.CreateAndClearTemporaryYakScript("yak", "hello 1; "+uid, uid)
	require.NoError(t, err)
	defer clearFunc()
	name2, clearFunc2, err := yakit.CreateAndClearTemporaryYakScript("yak", "hello 2; "+uid, uid)
	require.NoError(t, err)
	defer clearFunc2()
	assert.NotEmpty(t, name1)
	assert.NotEmpty(t, name2)

	password := utils.RandSecret(6)

	stream, err := client.ExportYakScriptStream(
		context.Background(),
		&ypb.ExportYakScriptStreamRequest{
			Filter: &ypb.QueryYakScriptRequest{
				Keyword:  uid,
				IsIgnore: true,
			},
			OutputFilename: "",
			Password:       password,
		},
	)
	if err != nil {
		t.Fatal(err)
	}

	outputFile := ""
	for {
		client, err := stream.Recv()
		if err != nil {
			break
		}
		if client.IsMessage {
			fmt.Println(string(client.Message))
			data := gjson.ParseBytes(client.Message).Get("content").Get("data")
			pathName := gjson.Parse(data.Str).Get("path").Str
			if pathName != "" {
				outputFile = pathName
			}
		}
	}
	if outputFile == "" {
		t.Fatal("output file is empty")
	}
	if utils.GetFirstExistedFile(outputFile) == "" {
		t.Fatal("output file not found")
	}

	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name1)
	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name2)

	stream2, _ := client.ImportYakScriptStream(context.Background(), &ypb.ImportYakScriptStreamRequest{
		Filename: outputFile,
		Password: password,
	})
	for {
		client, err := stream2.Recv()
		if err != nil {
			break
		}
		if client.IsMessage {
			log.Infof("message: %s", client.Message)
		}
	}
	t1, _ := yakit.GetYakScriptByName(consts.GetGormProfileDatabase(), name1)
	t2, _ := yakit.GetYakScriptByName(consts.GetGormProfileDatabase(), name2)
	assert.NotNil(t, t1)
	assert.NotNil(t, t2)

	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name1)
	yakit.DeleteYakScriptByName(consts.GetGormProfileDatabase(), name2)
}

func TestServerImportsPlugins(t *testing.T) {
	client, _ := NewLocalClient()

	content := "hello 1; " + uuid.NewString()
	name, clearFunc, err := yakit.CreateAndClearTemporaryYakScript("yak", content)
	t.Cleanup(clearFunc)

	createYakOutputZip := func() (string, string) {
		newContent := uuid.NewString()
		script, err := yakit.GetYakScriptByName(consts.GetGormProfileDatabase(), name)
		require.NoError(t, err)
		script.ID = 0
		require.Contains(t, script.Content, content)
		script.Content = newContent

		scriptRaw, err := json.Marshal(script)
		require.NoError(t, err)
		// create output plugin file
		path := t.TempDir() + "/final.zip"
		fp, err := os.Create(path)
		defer fp.Close()

		zipWriter := zip.NewWriter(fp)
		fileName := uuid.NewString() + ".json"
		fileSaver, err := zipWriter.Create(fileName)
		require.NoError(t, err)
		_, err = fileSaver.Write(scriptRaw)
		require.NoError(t, err)
		var output = make([]map[string]interface{}, 0, 64)
		output = append(output, map[string]any{
			"filename":    fileName,
			"script_name": script.ScriptName,
		})
		err = zipWriter.Flush()
		require.NoError(t, err)
		writer, err := zipWriter.Create("meta.json")
		require.NoError(t, err)
		raw, err := json.Marshal(output)
		require.NoError(t, err)
		_, err = writer.Write(raw)
		require.NoError(t, err)
		err = zipWriter.Close()
		require.NoError(t, err)
		return path, newContent
	}
	outputFile, newContent := createYakOutputZip()
	importStream, err := client.ImportYakScriptStream(context.Background(), &ypb.ImportYakScriptStreamRequest{
		Filename: outputFile,
	})
	require.NoError(t, err)
	for {
		client, err := importStream.Recv()
		if err != nil {
			break
		}
		if client.IsMessage {
			log.Infof("message: %s", client.Message)
		}
	}

	newScript, _ := yakit.GetYakScriptByName(consts.GetGormProfileDatabase(), name)
	require.NotNil(t, newScript)
	require.Contains(t, newScript.Content, newContent)
	require.NotContains(t, newScript.Content, content)
}
