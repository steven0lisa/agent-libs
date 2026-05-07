"""Skill loader — discovers and caches SKILL.md files from disk."""

from __future__ import annotations

import os
from pathlib import Path

from .types import SkillInfo, SkillMetadata, SkillLoaderOptions
from .yaml import parse_skill_file


class SkillLoader:
    """Discovers SKILL.md files from configured directories.

    Skills are cached after the first ``discover_all()`` call.
    User skills directory is scanned first, so duplicates from
    project skills are skipped.
    """

    def __init__(self, options: SkillLoaderOptions | None = None) -> None:
        default_dir = os.path.join(
            os.path.expanduser("~"), ".claude", "skills"
        )
        opts = options or SkillLoaderOptions()
        self._skills_dir = opts.skills_dir or default_dir
        self._include_project = opts.include_project_skills
        self._project_dir = opts.project_dir or os.getcwd()
        self._cache: list[SkillInfo] | None = None

    def discover_all(self) -> list[SkillInfo]:
        """Discover all skills from configured directories.

        Results are cached after the first call. Call ``clear_cache()``
        to force a re-scan.

        Returns:
            List of discovered ``SkillInfo`` objects.
        """
        if self._cache is not None:
            return self._cache

        skills: list[SkillInfo] = []
        scanned_dirs: set[str] = set()

        # User skills dir (higher priority — scanned first so duplicates
        # are skipped when scanning project skills)
        self._scan_directory(self._skills_dir, skills, scanned_dirs)

        # Project skills dir (optional)
        if self._include_project:
            project_skills_dir = os.path.join(
                os.path.abspath(self._project_dir), ".claude", "skills"
            )
            if project_skills_dir != self._skills_dir:
                self._scan_directory(
                    project_skills_dir, skills, scanned_dirs
                )

        self._cache = skills
        return skills

    def find_by_name(self, name: str) -> SkillInfo | None:
        """Find a skill by its metadata name.

        Args:
            name: The skill name to look up.

        Returns:
            ``SkillInfo`` if found, ``None`` otherwise.
        """
        skills = self.discover_all()
        for s in skills:
            if s.metadata.name == name:
                return s
        return None

    def clear_cache(self) -> None:
        """Clear the cached skill list so the next call re-scans."""
        self._cache = None

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _scan_directory(
        self,
        directory: str,
        result: list[SkillInfo],
        scanned_dirs: set[str],
    ) -> None:
        if not os.path.isdir(directory):
            return

        try:
            entries = sorted(
                os.scandir(directory), key=lambda e: e.name
            )
        except PermissionError:
            return

        for entry in entries:
            if not entry.is_dir(follow_symlinks=False):
                continue

            skill_dir = os.path.abspath(entry.path)
            skill_file = os.path.join(skill_dir, "SKILL.md")

            if not os.path.isfile(skill_file):
                continue
            if skill_dir in scanned_dirs:
                continue
            scanned_dirs.add(skill_dir)

            try:
                metadata_dict, content = parse_skill_file(skill_file)
                name = metadata_dict.pop("name", entry.name) or entry.name
                if not name:
                    continue

                meta = SkillMetadata(name=name, **metadata_dict)
                result.append(
                    SkillInfo(
                        metadata=meta,
                        content=content,
                        file_path=skill_file,
                        dir_path=skill_dir,
                    )
                )
            except Exception:
                # Silently skip invalid skills
                pass
