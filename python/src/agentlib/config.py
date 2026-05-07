"""Agent configuration."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional, Callable
from enum import Enum
import os

from .types import Event


class OutputFormat(str, Enum):
    TEXT = "text"
    JSON = "json"


@dataclass
class Pattern:
    """A pattern for whitelist/blacklist matching.

    Supports wildcard and regex patterns.
    """

    pattern: str
    type: str = "wildcard"  # "wildcard" or "regex"

    def matches(self, text: str) -> bool:
        import fnmatch
        import re

        if self.type == "wildcard":
            return fnmatch.fnmatch(text, self.pattern)
        elif self.type == "regex":
            return bool(re.search(self.pattern, text))
        return False


@dataclass
class AgentConfig:
    api_key: str = ""
    base_url: str = "https://api.anthropic.com"
    model: str = "claude-sonnet-4-6"
    work_dir: str = field(default_factory=lambda: os.getcwd())
    max_tokens: int = 8192
    max_turns: int = 100
    max_duration_ms: int = 0  # 0 = no limit
    system_prompt: Optional[str] = None
    timeout_ms: int = 120_000
    stream: bool = True
    tools: list = field(default_factory=list)
    output_format: OutputFormat = OutputFormat.TEXT
    callback: Optional[Callable[[Event], None]] = None
    enable_subagent: bool = False
    subagent_max_turns: int = 50
    enable_skills: bool = False
    skills_dir: str = ""
    include_project_skills: bool = False
    skills_project_dir: str = ""
    bash_whitelist: list[Pattern] = field(default_factory=list)
    bash_blacklist: list[Pattern] = field(default_factory=list)
    curl_whitelist: list[Pattern] = field(default_factory=list)
    curl_blacklist: list[Pattern] = field(default_factory=list)

    # All configuration must be provided explicitly by the caller.
    # The library does not read environment variables, to support
    # multi-tenant scenarios with different credentials per instance.
