"""Variable substitution for skill content.

Supported substitutions:
    - $ARGUMENTS           → user-provided arguments string
    - ${CLAUDE_SKILL_DIR}  → skill directory path
    - ${ENV:VAR_NAME}      → environment variable value (empty string if unset)
"""

from __future__ import annotations

import os
import re


def substitute_variables(
    content: str,
    args: str | None = None,
    skill_dir: str | None = None,
) -> str:
    """Replace variable placeholders in skill content with actual values.

    Args:
        content: Raw skill content with variable placeholders.
        args: Optional user-provided argument string for ``$ARGUMENTS``.
        skill_dir: Optional skill directory path for ``${CLAUDE_SKILL_DIR}``.

    Returns:
        Content with all known variables substituted.
    """
    result = content

    if args is not None:
        result = re.sub(r"\$ARGUMENTS", args, result)

    if skill_dir is not None:
        result = re.sub(r"\$\{CLAUDE_SKILL_DIR\}", skill_dir, result)

    def _replace_env(match: re.Match) -> str:
        var_name = match.group(1)
        return os.environ.get(var_name, "")

    result = re.sub(r"\$\{ENV:([^}]+)\}", _replace_env, result)

    return result
