package yakgrpc

import (
	"context"
	"strings"

	"github.com/yaklang/yaklang/common/consts"
	"github.com/yaklang/yaklang/common/utils"
	"github.com/yaklang/yaklang/common/yakgrpc/ypb"
)

func (s *Server) Version(ctx context.Context, _ *ypb.Empty) (*ypb.VersionResponse, error) {
	return &ypb.VersionResponse{Version: consts.GetPalmVersion()}, nil
}

func (s *Server) YakVersionAtLeast(ctx context.Context, req *ypb.YakVersionAtLeastRequest) (*ypb.GeneralResponse, error) {
	version := req.GetYakVersion()
	atLeastVersion := req.GetAtLeastVersion()
	if version == "" {
		version = consts.GetYakVersion()
	}
	if strings.HasPrefix(version, "v") {
		version = version[1:]
	}
	if strings.HasPrefix(atLeastVersion, "v") {
		atLeastVersion = atLeastVersion[1:]
	}

	ok := false
	// 本地/CI 构建的引擎版本形如 dev-<shorthash>，无法参与 semver 比较，视为开发版直接放行
	if version == "" || strings.HasPrefix(version, "dev") {
		ok = true
	} else {
		ok = utils.VersionGreaterEqual(version, atLeastVersion)
	}

	return &ypb.GeneralResponse{Ok: ok}, nil
}

func (s *Server) GetMachineID(ctx context.Context, _ *ypb.Empty) (*ypb.GetMachineIDResponse, error) {
	return &ypb.GetMachineIDResponse{MachineID: utils.GetMachineCode()}, nil
}
