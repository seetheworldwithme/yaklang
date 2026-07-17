package crep

import (
	"crypto/x509"
	"encoding/pem"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yaklang/yaklang/common/utils/tlsutils"
)

func TestInitMITMCertMigratesLegacyRootCAName(t *testing.T) {
	tempDir := t.TempDir()
	legacyCA, legacyKey, err := tlsutils.GenerateSelfSignedCertKeyWithCommonName(
		"Yakit MITM Root CA",
		"mitmserver",
		nil,
		nil,
	)
	if err != nil {
		t.Fatalf("generate legacy MITM root CA: %v", err)
	}

	originalCAFile, originalKeyFile := defaultCAFile, defaultKeyFile
	originalGMCAFile, originalGMKeyFile := defaultGMCAFile, defaultGMKeyFile
	originalCA, originalKey := defaultCA, defaultKey
	originalGMCA, originalGMKey := defaultGMCA, defaultGMKey
	t.Cleanup(func() {
		defaultCAFile, defaultKeyFile = originalCAFile, originalKeyFile
		defaultGMCAFile, defaultGMKeyFile = originalGMCAFile, originalGMKeyFile
		defaultCA, defaultKey = originalCA, originalKey
		defaultGMCA, defaultGMKey = originalGMCA, originalGMKey
	})

	defaultCAFile = filepath.Join(tempDir, "yak-mitm-ca.crt")
	defaultKeyFile = filepath.Join(tempDir, "yak-mitm-ca.key")
	defaultGMCAFile = filepath.Join(tempDir, "yak-mitm-gm-ca.crt")
	defaultGMKeyFile = filepath.Join(tempDir, "yak-mitm-gm-ca.key")
	defaultCA, defaultKey, defaultGMCA, defaultGMKey = nil, nil, nil, nil

	if err := os.WriteFile(defaultCAFile, legacyCA, 0o600); err != nil {
		t.Fatalf("write legacy MITM root CA: %v", err)
	}
	if err := os.WriteFile(defaultKeyFile, legacyKey, 0o600); err != nil {
		t.Fatalf("write legacy MITM root key: %v", err)
	}

	InitMITMCert()

	block, _ := pem.Decode(defaultCA)
	if block == nil {
		t.Fatal("decode migrated MITM root CA: no PEM block")
	}
	cert, err := x509.ParseCertificate(block.Bytes)
	if err != nil {
		t.Fatalf("parse migrated MITM root CA: %v", err)
	}
	if cert.Subject.CommonName != "MITM Root CA" {
		t.Fatalf("unexpected MITM root CA common name: got %q, want %q", cert.Subject.CommonName, "MITM Root CA")
	}
	if strings.Contains(cert.Subject.String(), "Yakit") || strings.Contains(cert.Issuer.String(), "Yakit") {
		t.Fatalf("legacy product name remains in migrated certificate: subject=%q issuer=%q", cert.Subject, cert.Issuer)
	}
}
