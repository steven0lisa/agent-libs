"""Security utilities for path validation and command filtering."""

from __future__ import annotations

import os
from pathlib import Path

from ..config import Pattern


def resolve_safe_path(
    file_path: str,
    work_dir: str,
    allowed_dirs: list[str] | None = None,
) -> Path:
    """Resolve a path and ensure it stays within allowed directories.

    If allowed_dirs is empty or None, only work_dir is allowed.
    work_dir is always implicitly allowed.

    Returns the resolved path if valid.
    Raises ValueError if the path escapes all allowed directories.
    """
    work_abs = os.path.abspath(work_dir)
    target_abs = os.path.normpath(os.path.join(work_abs, file_path))

    # Collect all allowed directories (work_dir is always allowed)
    all_allowed = [os.path.realpath(work_abs)]
    if allowed_dirs:
        for d in allowed_dirs:
            all_allowed.append(os.path.realpath(os.path.abspath(d)))

    target_real = os.path.realpath(target_abs)

    # Check if target is under any allowed directory
    for allowed_dir in all_allowed:
        try:
            if os.path.commonpath([allowed_dir, target_real]) == allowed_dir:
                return Path(target_abs)
        except ValueError:
            # Different drives on Windows
            continue

    raise ValueError(
        f"Path '{file_path}' escapes allowed directories. "
        f"Allowed: {[str(d) for d in all_allowed]}"
    )


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
