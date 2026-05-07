package utils

import (
	"os"
	"path/filepath"
	"testing"
)

func TestResolveSafePath(t *testing.T) {
	workDir := t.TempDir()

	tests := []struct {
		name    string
		path    string
		wantErr bool
	}{
		{"simple file", "test.txt", false},
		{"nested file", "subdir/file.txt", false},
		{"absolute in workdir", filepath.Join(workDir, "file.txt"), false},
		{"escape with dots", "../escape.txt", true},
		{"escape nested", "foo/../../escape.txt", true},
		{"dots in middle", "foo/../bar/file.txt", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resolved, err := ResolveSafePath(tt.path, workDir)
			if tt.wantErr {
				if err == nil {
					t.Errorf("expected error for path %q, got resolved path %q", tt.path, resolved)
				}
				return
			}
			if err != nil {
				t.Errorf("unexpected error for path %q: %v", tt.path, err)
			}
			if resolved == "" {
				t.Error("expected non-empty resolved path")
			}
		})
	}
}

func TestResolveSafePathWithAbsoluteEscape(t *testing.T) {
	workDir := t.TempDir()
	outsideDir := t.TempDir()

	// Try to use an absolute path outside the working directory
	outsideFile := filepath.Join(outsideDir, "secret.txt")
	_, err := ResolveSafePath(outsideFile, workDir)
	if err == nil {
		t.Error("expected error for absolute path outside working directory")
	}
}

func TestCheckSecurityPolicyWhitelist(t *testing.T) {
	whitelist := []Pattern{
		{Pattern: "ls*", Type: "wildcard"},
		{Pattern: "cat*", Type: "wildcard"},
	}
	blacklist := []Pattern{
		{Pattern: "rm*", Type: "wildcard"},
	}

	// Whitelisted command should be allowed even if it looks like blacklist
	allowed, reason := CheckSecurityPolicy("ls -la", whitelist, blacklist, true)
	if !allowed {
		t.Errorf("expected 'ls -la' to be allowed, got: %s", reason)
	}

	// Blacklisted command should be denied
	allowed, reason = CheckSecurityPolicy("rm -rf /", whitelist, blacklist, true)
	if allowed {
		t.Errorf("expected 'rm -rf /' to be denied, got: %s", reason)
	}

	// Neither whitelist nor blacklist - default allow
	allowed, reason = CheckSecurityPolicy("echo hello", whitelist, blacklist, true)
	if !allowed {
		t.Errorf("expected 'echo hello' to be allowed by default, got: %s", reason)
	}
}

func TestCheckSecurityPolicyDefaultDeny(t *testing.T) {
	whitelist := []Pattern{}
	blacklist := []Pattern{}

	// Default deny
	allowed, _ := CheckSecurityPolicy("anything", whitelist, blacklist, false)
	if allowed {
		t.Error("expected default deny to block everything")
	}
}

func TestCheckSecurityPolicyWhitelistPriority(t *testing.T) {
	// Whitelist should take priority over blacklist
	whitelist := []Pattern{
		{Pattern: "rm *", Type: "wildcard"},
	}
	blacklist := []Pattern{
		{Pattern: "rm *", Type: "wildcard"},
	}

	allowed, reason := CheckSecurityPolicy("rm file.txt", whitelist, blacklist, true)
	if !allowed {
		t.Errorf("expected whitelist to take priority, but got: %s", reason)
	}
}

func TestPatternMatchesWildcard(t *testing.T) {
	tests := []struct {
		pattern string
		text    string
		want    bool
	}{
		{"ls*", "ls -la", true},
		{"ls*", "ls /tmp", true},
		{"ls*", "cat file.txt", false},
		{"*secret*", "mysecretfile", true},
		{"*secret*", "public", false},
		{"grep", "grep hello", true},
		{"grep", "ag hello", false},
	}

	for _, tt := range tests {
		p := Pattern{Pattern: tt.pattern, Type: "wildcard"}
		got := p.Matches(tt.text)
		if got != tt.want {
			t.Errorf("Pattern(%q).Matches(%q) = %v, want %v", tt.pattern, tt.text, got, tt.want)
		}
	}
}

func TestPatternMatchesRegex(t *testing.T) {
	tests := []struct {
		pattern string
		text    string
		want    bool
	}{
		{"^ls", "ls -la", true},
		{"^ls", "cat file", false},
		{"rm$", "rm", true},
		{"rm$", "rm -rf", false},
		{"exact", "exact", true},
	}

	for _, tt := range tests {
		p := Pattern{Pattern: tt.pattern, Type: "regex"}
		got := p.Matches(tt.text)
		if got != tt.want {
			t.Errorf("Pattern(%q).Matches(%q) = %v, want %v", tt.pattern, tt.text, got, tt.want)
		}
	}
}

func TestEnsureDir(t *testing.T) {
	tmpDir := t.TempDir()
	testPath := filepath.Join(tmpDir, "a", "b", "c", "file.txt")

	err := EnsureDir(testPath)
	if err != nil {
		t.Fatalf("EnsureDir failed: %v", err)
	}

	// Check that the directory was created
	_, err = os.Stat(filepath.Join(tmpDir, "a", "b", "c"))
	if os.IsNotExist(err) {
		t.Error("expected directory to be created")
	}
}

func TestWriteFile(t *testing.T) {
	tmpDir := t.TempDir()
	testPath := filepath.Join(tmpDir, "test.txt")
	data := []byte("hello world")

	err := WriteFile(testPath, data)
	if err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	read, err := os.ReadFile(testPath)
	if err != nil {
		t.Fatalf("failed to read file: %v", err)
	}
	if string(read) != "hello world" {
		t.Errorf("expected 'hello world', got %s", string(read))
	}
}

func TestPatternString(t *testing.T) {
	p1 := Pattern{Pattern: "test", Type: "wildcard"}
	if p1.String() != "wildcard(test)" {
		t.Errorf("expected 'wildcard(test)', got %s", p1.String())
	}

	p2 := Pattern{Pattern: "test", Type: ""}
	if p2.String() != "test" {
		t.Errorf("expected 'test', got %s", p2.String())
	}
}
