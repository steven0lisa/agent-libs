"""Security utilities for path validation and command filtering."""

from __future__ import annotations

import os
from pathlib import Path

from ..config import Pattern


def resolve_safe_path(file_path: str, work_dir: str) -> Path:
    """Resolve a path and ensure it stays within the working directory.

    Returns the resolved path if valid.
    Raises ValueError if the path escapes the working directory.
    """
    work_abs = os.path.abspath(work_dir)
    target_abs = os.path.normpath(os.path.join(work_abs, file_path))

    work_real = os.path.realpath(work_abs)
    target_real = os.path.realpath(target_abs)
    if os.path.commonpath([work_real, target_real]) != work_real:
        raise ValueError(f"Path '{file_path}' escapes working directory '{work_dir}'")

    return Path(target_abs)


def check_security_policy(
    text: str,
    whitelist: list[Pattern],
    blacklist: list[Pattern],
    default_allow: bool = True,
) -> tuple[bool, str]:
    """Check if text passes the whitelist/blacklist policy.

    Returns (allowed, reason).
    """
    # Check whitelist first - if matched, allow
    for pattern in whitelist:
        if pattern.matches(text):
            return True, f"Matched whitelist pattern: {pattern.pattern}"

    # Check blacklist - if matched, deny
    for pattern in blacklist:
        if pattern.matches(text):
            return False, f"Matched blacklist pattern: {pattern.pattern}"

    # Default action
    if default_allow:
        return True, "Default allow"
    return False, "Default deny"
