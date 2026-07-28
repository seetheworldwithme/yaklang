package yakgrpc

import (
	"archive/zip"
	"bytes"
	"encoding/json"
	"io"
	"os"
	"path/filepath"

	"github.com/google/uuid"
	"github.com/yaklang/yaklang/common/consts"
	"github.com/yaklang/yaklang/common/log"
	"github.com/yaklang/yaklang/common/schema"
	"github.com/yaklang/yaklang/common/utils"
	"github.com/yaklang/yaklang/common/yak/pluginbundle"
	"github.com/yaklang/yaklang/common/yak/yaklib"
	"github.com/yaklang/yaklang/common/yak/yaklib/codec"
	"github.com/yaklang/yaklang/common/yakgrpc/yakit"
	"github.com/yaklang/yaklang/common/yakgrpc/ypb"
)

func (s *Server) ImportYakScriptStream(
	req *ypb.ImportYakScriptStreamRequest,
	stream ypb.Yak_ImportYakScriptStreamServer,
) error {
	var err error

	data := req.GetData()
	if len(data) <= 0 {
		data, err = os.ReadFile(req.GetFilename())
		if err != nil {
			return utils.Wrapf(err, "read file failed: %v", req.GetFilename())
		}
	}

	var zipReader *zip.Reader
	if req.GetPassword() == "" {
		zipReader, err = zip.NewReader(bytes.NewReader(data), int64(len(data)))
		if err != nil {
			return utils.Wrap(err, "create zip reader failed, do we need password maybe!")
		}
	} else {
		results, err := codec.SM4DecryptCBCWithPKCSPadding(
			codec.PKCS7Padding([]byte(req.GetPassword())),
			data,
			codec.PKCS7Padding([]byte(req.GetPassword())),
		)
		if err != nil {
			return utils.Wrapf(err, "decrypt file failed: %v", req.GetFilename())
		}
		zipReader, err = zip.NewReader(bytes.NewReader(results), int64(len(results)))
		if err != nil {
			return utils.Wrap(err, "create zip reader failed, file is decrypted but broken")
		}
	}

	if zipReader == nil {
		return utils.Errorf("zip reader is nil")
	}

	metaReader, err := zipReader.Open("meta.json")
	if err != nil {
		return utils.Wrap(err, "open meta.json failed")
	}
	var results []pluginbundle.Metadata
	if err := json.NewDecoder(metaReader).Decode(&results); err != nil {
		return utils.Wrap(err, "decode meta.json failed")
	}
	metaReader.Close()

	client := yaklib.NewVirtualYakitClient(stream.Send)
	_ = client

	db := consts.GetGormProfileDatabase()
	tx := db.Begin()
	if tx.Error != nil {
		return utils.Wrap(tx.Error, "begin import yakit plugin transaction failed")
	}
	defer tx.Rollback()

	for _, metadata := range results {
		if metadata.Filename == "" {
			continue
		}
		fp, err := zipReader.Open(metadata.Filename)
		if err != nil {
			return utils.Wrapf(err, "open file failed: %v", metadata.Filename)
		}
		raw, _ := io.ReadAll(fp)
		fp.Close()
		var script schema.YakScript
		if err := json.Unmarshal(raw, &script); err != nil {
			return utils.Wrapf(err, "unmarshal yakit script failed: %v", metadata.Filename)
		}
		if script.ScriptName == "" {
			log.Warnf("yakit script name is empty: %v", metadata.Filename)
			continue
		}

		var sourceGroups []pluginbundle.Group
		if metadata.Groups == nil {
			var existingGroups []*schema.PluginGroup
			if query := tx.Where("yak_script_name = ?", script.ScriptName).Find(&existingGroups); query.Error != nil {
				return utils.Wrapf(query.Error, "query existing plugin groups failed: %v", script.ScriptName)
			}
			sourceGroups = pluginbundle.FromSchema(existingGroups)
		} else {
			sourceGroups = *metadata.Groups
			if err := yakit.DeletePluginGroupByScriptName(tx, []string{script.ScriptName}); err != nil {
				return utils.Wrapf(err, "delete old plugin groups failed: %v", script.ScriptName)
			}
		}

		err = yakit.CreateOrUpdateYakScriptByName(tx, script.ScriptName, &script)
		if err != nil {
			return utils.Wrapf(err, "create or update yakit script failed: %v", script.ScriptName)
		}
		for _, group := range pluginbundle.NormalizeGroups(&script, sourceGroups, pluginbundle.OfflineImportGroup) {
			if err := yakit.CreateOrUpdatePluginGroup(tx, group.Hash, group); err != nil {
				return utils.Wrapf(err, "create or update yakit plugin group failed: %v", script.ScriptName)
			}
		}
	}
	if err := tx.Commit().Error; err != nil {
		return utils.Wrap(err, "commit import yakit plugin transaction failed")
	}
	return nil
}

func (s *Server) ExportYakScriptStream(
	req *ypb.ExportYakScriptStreamRequest,
	stream ypb.Yak_ExportYakScriptStreamServer,
) error {
	outputDir := req.GetOutputPluginDir()
	if outputDir == "" {
		outputDir = consts.GetDefaultYakitProjectsDir()
	}
	tempFilename := req.GetOutputFilename()
	if utils.StringContainsAnyOfSubString(tempFilename, []string{
		"\\", "|", "/",
	}) {
		return utils.Errorf("output filename contains invalid characters: %v (not contains \\, |, / )", tempFilename)
	}

	db := consts.GetGormProfileDatabase().Model(&schema.YakScript{})
	db = yakit.FilterYakScript(db, req.GetFilter())

	client := yaklib.NewVirtualYakitClient(stream.Send)
	client.YakitSetProgress(0.1)

	var total int64
	if err := db.Count(&total).Error; err != nil {
		return err
	}

	if total <= 0 {
		return utils.Error("no yakit script found")
	}

	step := 0.8 / float64(total)
	var buf bytes.Buffer
	zipWriter := zip.NewWriter(&buf)
	var output = make([]pluginbundle.Metadata, 0, 64)
	for script := range yakit.YieldYakScripts(db, stream.Context()) {
		select {
		case <-stream.Context().Done():
			return nil
		default:
		}
		ruid := uuid.New().String()
		filename := ruid + ".json"

		script.ID = 0
		scriptRaw, err := json.Marshal(script)
		if err != nil {
			return utils.Wrapf(err, "marshal yakit script failed: %v", script.ScriptName)
		}

		fileSaver, err := zipWriter.Create(filename)
		if err != nil {
			return err
		}
		_, err = fileSaver.Write(scriptRaw)
		if err != nil {
			log.Warnf("write yakit script failed: %v", script.ScriptName)
			return err
		} else if err := zipWriter.Flush(); err != nil {
			log.Warnf("flush yakit script failed: %v", script.ScriptName)
			return err
		}
		var storedGroups []*schema.PluginGroup
		if query := consts.GetGormProfileDatabase().Where("yak_script_name = ?", script.ScriptName).Find(&storedGroups); query.Error != nil {
			return utils.Wrapf(query.Error, "query yakit plugin groups failed: %v", script.ScriptName)
		}
		groups := pluginbundle.FromSchema(storedGroups)
		output = append(output, pluginbundle.Metadata{
			Filename:   filename,
			ScriptName: script.ScriptName,
			Groups:     &groups,
		})
		client.YakitSetProgress(step + 0.1)
	}
	err := zipWriter.Flush()
	if err != nil {
		return err
	}
	writer, err := zipWriter.Create("meta.json")
	if err != nil {
		return utils.Wrapf(err, "create yakit plugin meta.json")
	}
	raw, err := json.Marshal(output)
	if err != nil {
		return utils.Wrapf(err, "marshal yakit plugin meta.json")
	}
	_, err = writer.Write(raw)
	if err != nil {
		return utils.Wrapf(err, "write yakit plugin meta.json")
	}
	zipWriter.Close()
	defer func() {
		client.YakitSetProgress(1.0)
	}()
	if req.OutputFilename == "" {
		req.OutputFilename = "yakit_plugins_" + utils.DatetimePretty2() + ".zip"
	}

	if filepath.Ext(req.OutputFilename) != ".zip" { // try fix extension
		req.OutputFilename += ".zip"
	}

	var results []byte = buf.Bytes()
	if req.Password != "" {
		req.OutputFilename += ".enc"
		results, err = codec.SM4EncryptCBCWithPKCSPadding(
			codec.PKCS7Padding([]byte(req.Password)),
			results, codec.PKCS7Padding([]byte(req.Password)),
		)
		if err != nil {
			return err
		}
	}

	finalFilename := filepath.Join(outputDir, req.OutputFilename)
	fp, err := os.Create(finalFilename)
	if err != nil {
		return err
	}
	defer fp.Close()
	fp.Write(results)

	if req.Password == "" {
		client.YakitFile(finalFilename, "Yakit Plugin Output", "Empty Password")
	} else {
		client.YakitFile(finalFilename, "Yakit Plugin Output", "Encrypted with SM4")
	}

	return nil
}
