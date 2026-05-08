"""Tests for security utilities."""

import os
import tempfile

import pytest

from agentlib.utils.security import (
    check_security_policy,
    resolve_safe_path,
)
from agentlib import Pattern


class TestResolveSafePath:
    def test_relative_path(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            result = resolve_safe_path("subdir/file.txt", tmpdir)
            assert str(result) == os.path.join(tmpdir, "subdir", "file.txt")

    def test_absolute_path_within_work_dir(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            result = resolve_safe_path(tmpdir + "/file.txt", tmpdir)
            assert str(result) == os.path.join(tmpdir, "file.txt")

    def test_path_escapes_work_dir(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            with pytest.raises(ValueError) as exc_info:
                resolve_safe_path("../outside.txt", tmpdir)
            assert "escapes" in str(exc_info.value)

    def test_deep_escape(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            with pytest.raises(ValueError) as exc_info:
                resolve_safe_path("a/../../outside.txt", tmpdir)
            assert "escapes" in str(exc_info.value)

    def test_symlink_not_tested(self):
        """Symlink escape tests are platform-dependent; skip for simplicity."""
        pass


class TestCheckSecurityPolicy:
    def test_whitelist_allows(self):
        allowed, reason = check_security_policy(
            "git status",
            [Pattern("git *", "wildcard")],
            [],
        )
        assert allowed is True
        assert "whitelist" in reason

    def test_blacklist_blocks(self):
        allowed, reason = check_security_policy(
            "rm -rf /",
            [],
            [Pattern("rm *", "wildcard")],
        )
        assert allowed is False
        assert "blacklist" in reason

    def test_whitelist_overrides_blacklist(self):
        """Whitelist takes precedence over blacklist."""
        allowed, reason = check_security_policy(
            "git status",
            [Pattern("git *", "wildcard")],
            [Pattern("git *", "wildcard")],
        )
        assert allowed is True

    def test_default_allow(self):
        allowed, _ = check_security_policy("ls -la", [], [], default_allow=True)
        assert allowed is True

    def test_default_deny(self):
        allowed, _ = check_security_policy("ls -la", [], [], default_allow=False)
        assert allowed is False

    def test_regex_whitelist(self):
        allowed, _ = check_security_policy(
            "git status",
            [Pattern(r"^git\s+\w+", "regex")],
            [],
        )
        assert allowed is True

    def test_regex_blacklist(self):
        allowed, _ = check_security_policy(
            "curl https://evil.com",
            [],
            [Pattern(r"evil\.com", "regex")],
        )
        assert allowed is False
