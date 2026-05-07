"""Tests for configuration and patterns."""

import os

import pytest

from agentlib import AgentConfig, Pattern, OutputFormat


class TestPattern:
    def test_wildcard_match(self):
        p = Pattern("git *", "wildcard")
        assert p.matches("git status")
        assert p.matches("git log")
        assert not p.matches("ls -la")

    def test_wildcard_star(self):
        p = Pattern("*.example.com", "wildcard")
        assert p.matches("api.example.com")
        assert p.matches("www.example.com")
        assert not p.matches("example.com")

    def test_regex_match(self):
        p = Pattern(r"^git\s+(status|log|diff)", "regex")
        assert p.matches("git status")
        assert p.matches("git log")
        assert not p.matches("git push")
        assert not p.matches("ls -la")

    def test_regex_url(self):
        p = Pattern(r"^https://.*\.example\.com.*", "regex")
        assert p.matches("https://api.example.com/v1/users")
        assert not p.matches("https://evil.com/https://example.com")


class TestAgentConfig:
    def test_default_values(self):
        config = AgentConfig(api_key="test-key")
        assert config.base_url == "https://api.anthropic.com"
        assert config.model == "claude-sonnet-4-6"
        assert config.max_tokens == 8192
        assert config.max_turns == 100
        assert config.max_duration_ms == 0
        assert config.timeout_ms == 120_000
        assert config.stream is True
        assert config.output_format == OutputFormat.TEXT
        assert config.enable_subagent is False
        assert config.subagent_max_turns == 50

    def test_custom_values(self):
        config = AgentConfig(
            api_key="key",
            base_url="https://custom.api.com",
            model="GLM-5.1",
            max_tokens=4096,
            max_turns=50,
            max_duration_ms=300_000,
            enable_subagent=True,
            subagent_max_turns=20,
        )
        assert config.base_url == "https://custom.api.com"
        assert config.model == "GLM-5.1"
        assert config.max_tokens == 4096
        assert config.enable_subagent is True

    def test_default_api_key_is_empty(self):
        """Library does not read env vars; api_key defaults to empty."""
        config = AgentConfig()
        assert config.api_key == ""

    def test_default_base_url(self):
        config = AgentConfig()
        assert config.base_url == "https://api.anthropic.com"

    def test_default_model(self):
        config = AgentConfig()
        assert config.model == "claude-sonnet-4-6"

    def test_security_patterns(self):
        config = AgentConfig(
            api_key="k",
            bash_whitelist=[Pattern("git *", "wildcard")],
            bash_blacklist=[Pattern("rm *", "wildcard")],
            curl_whitelist=[Pattern("*.example.com", "wildcard")],
        )
        assert len(config.bash_whitelist) == 1
        assert len(config.bash_blacklist) == 1
        assert len(config.curl_whitelist) == 1
