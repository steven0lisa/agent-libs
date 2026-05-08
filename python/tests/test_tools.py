"""Tests for built-in tools."""

import os
import tempfile

import pytest

from agentlib import (
    AgentConfig,
    BashTool,
    CurlTool,
    Pattern,
    ReadFileTool,
    ToolContext,
    ToolResult,
    UpdateFileTool,
    WriteFileTool,
)


class TestReadFileTool:
    @pytest.fixture
    def tool(self):
        return ReadFileTool()

    @pytest.fixture
    def context(self):
        return ToolContext(work_dir="/tmp", message_history=[])

    @pytest.mark.asyncio
    async def test_read_existing_file(self, tool, context):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
            f.write("hello world")
            path = f.name

        try:
            ctx = ToolContext(work_dir=os.path.dirname(path), message_history=[])
            result = await tool.call({"file_path": os.path.basename(path)}, ctx)
            assert result.is_error is False
            assert result.content == "hello world"
        finally:
            os.unlink(path)

    @pytest.mark.asyncio
    async def test_read_nonexistent_file(self, tool):
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"file_path": "nonexistent_file_12345.txt"}, ctx)
        assert result.is_error is True
        assert "Failed to read" in result.content

    @pytest.mark.asyncio
    async def test_path_escape_blocked(self, tool):
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"file_path": "../etc/passwd"}, ctx)
        assert result.is_error is True
        assert "escapes" in result.content

    @pytest.mark.asyncio
    async def test_missing_file_path(self, tool, context):
        result = await tool.call({}, context)
        assert result.is_error is True

    def test_is_read_only(self, tool):
        assert tool.is_read_only is True


class TestWriteFileTool:
    @pytest.fixture
    def tool(self):
        return WriteFileTool()

    @pytest.mark.asyncio
    async def test_write_new_file(self, tool):
        with tempfile.TemporaryDirectory() as tmpdir:
            ctx = ToolContext(work_dir=tmpdir, message_history=[])
            result = await tool.call(
                {"file_path": "test.txt", "content": "hello"}, ctx
            )
            assert result.is_error is False
            assert os.path.exists(os.path.join(tmpdir, "test.txt"))
            with open(os.path.join(tmpdir, "test.txt")) as f:
                assert f.read() == "hello"

    @pytest.mark.asyncio
    async def test_create_nested_directories(self, tool):
        with tempfile.TemporaryDirectory() as tmpdir:
            ctx = ToolContext(work_dir=tmpdir, message_history=[])
            result = await tool.call(
                {"file_path": "a/b/c/test.txt", "content": "nested"}, ctx
            )
            assert result.is_error is False
            assert os.path.exists(os.path.join(tmpdir, "a", "b", "c", "test.txt"))

    @pytest.mark.asyncio
    async def test_path_escape_blocked(self, tool):
        with tempfile.TemporaryDirectory() as tmpdir:
            ctx = ToolContext(work_dir=tmpdir, message_history=[])
            result = await tool.call(
                {"file_path": "../outside.txt", "content": "x"}, ctx
            )
            assert result.is_error is True
            assert "escapes" in result.content

    def test_is_not_read_only(self, tool):
        assert tool.is_read_only is False


class TestUpdateFileTool:
    @pytest.fixture
    def tool(self):
        return UpdateFileTool()

    @pytest.mark.asyncio
    async def test_replace_single_occurrence(self, tool):
        with tempfile.TemporaryDirectory() as tmpdir:
            filepath = os.path.join(tmpdir, "test.txt")
            with open(filepath, "w") as f:
                f.write("hello world hello")

            ctx = ToolContext(work_dir=tmpdir, message_history=[])
            result = await tool.call(
                {
                    "file_path": "test.txt",
                    "old_string": "world",
                    "new_string": "universe",
                },
                ctx,
            )
            assert result.is_error is False
            with open(filepath) as f:
                assert f.read() == "hello universe hello"

    @pytest.mark.asyncio
    async def test_replace_all_occurrences(self, tool):
        with tempfile.TemporaryDirectory() as tmpdir:
            filepath = os.path.join(tmpdir, "test.txt")
            with open(filepath, "w") as f:
                f.write("hello hello hello")

            ctx = ToolContext(work_dir=tmpdir, message_history=[])
            result = await tool.call(
                {
                    "file_path": "test.txt",
                    "old_string": "hello",
                    "new_string": "hi",
                    "replace_all": True,
                },
                ctx,
            )
            assert result.is_error is False
            with open(filepath) as f:
                assert f.read() == "hi hi hi"

    @pytest.mark.asyncio
    async def test_old_string_not_found(self, tool):
        with tempfile.TemporaryDirectory() as tmpdir:
            filepath = os.path.join(tmpdir, "test.txt")
            with open(filepath, "w") as f:
                f.write("hello world")

            ctx = ToolContext(work_dir=tmpdir, message_history=[])
            result = await tool.call(
                {
                    "file_path": "test.txt",
                    "old_string": "notfound",
                    "new_string": "x",
                },
                ctx,
            )
            assert result.is_error is True
            assert "not found" in result.content

    @pytest.mark.asyncio
    async def test_path_escape_blocked(self, tool):
        with tempfile.TemporaryDirectory() as tmpdir:
            ctx = ToolContext(work_dir=tmpdir, message_history=[])
            result = await tool.call(
                {
                    "file_path": "../etc/passwd",
                    "old_string": "x",
                    "new_string": "y",
                },
                ctx,
            )
            assert result.is_error is True
            assert "escapes" in result.content


class TestBashTool:
    @pytest.fixture
    def tool(self):
        return BashTool()

    @pytest.mark.asyncio
    async def test_echo_command(self, tool):
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"command": "echo hello"}, ctx)
        assert result.is_error is False
        assert "hello" in result.content

    @pytest.mark.asyncio
    async def test_command_with_stderr(self, tool):
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"command": "echo error >&2"}, ctx)
        # Command succeeds but stderr contains text
        assert result.is_error is False
        assert "[stderr]" in result.content or "error" in result.content

    @pytest.mark.asyncio
    async def test_failing_command(self, tool):
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"command": "exit 1"}, ctx)
        assert result.is_error is True

    @pytest.mark.asyncio
    async def test_timeout(self, tool):
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call(
            {"command": "sleep 10", "timeout": 100}, ctx
        )
        assert result.is_error is True
        assert "timed out" in result.content

    @pytest.mark.asyncio
    async def test_whitelist_allows(self):
        tool = BashTool(whitelist=[Pattern("echo *", "wildcard")])
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"command": "echo hello"}, ctx)
        assert result.is_error is False
        assert "hello" in result.content

    @pytest.mark.asyncio
    async def test_blacklist_blocks(self):
        tool = BashTool(blacklist=[Pattern("rm *", "wildcard")])
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"command": "rm -rf /"}, ctx)
        assert result.is_error is True
        assert "blocked by security policy" in result.content

    @pytest.mark.asyncio
    async def test_whitelist_overrides_blacklist(self):
        tool = BashTool(
            whitelist=[Pattern("git *", "wildcard")],
            blacklist=[Pattern("git *", "wildcard")],
        )
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"command": "git status"}, ctx)
        # Whitelist should take precedence
        assert result.is_error is False

    @pytest.mark.asyncio
    async def test_regex_blacklist(self):
        tool = BashTool(blacklist=[Pattern(r"rm\s+-rf\s+\/?", "regex")])
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call({"command": "rm -rf /"}, ctx)
        assert result.is_error is True

    def test_is_not_read_only(self, tool):
        # BashTool does not declare itself read-only
        assert tool.is_read_only is False


class TestCurlTool:
    @pytest.fixture
    def tool(self):
        return CurlTool()

    @pytest.mark.asyncio
    async def test_get_request(self, tool):
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call(
            {"url": "https://httpbin.org/get"}, ctx
        )
        # This is a real network call; in unit tests we'd mock
        # For now just check it doesn't crash
        assert isinstance(result.content, str)

    def test_is_read_only(self, tool):
        assert tool.is_read_only is True

    @pytest.mark.asyncio
    async def test_whitelist_allows(self):
        tool = CurlTool(whitelist=[Pattern("*httpbin.org*", "wildcard")])
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call(
            {"url": "https://httpbin.org/get"}, ctx
        )
        assert "blocked by security policy" not in result.content

    @pytest.mark.asyncio
    async def test_blacklist_blocks(self):
        tool = CurlTool(blacklist=[Pattern("*evil.com*", "wildcard")])
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call(
            {"url": "https://evil.com/data"}, ctx
        )
        assert result.is_error is True
        assert "blocked by security policy" in result.content

    @pytest.mark.asyncio
    async def test_regex_url_blacklist(self):
        tool = CurlTool(
            blacklist=[Pattern(r"evil\.com|malicious\.org", "regex")]
        )
        ctx = ToolContext(work_dir="/tmp", message_history=[])
        result = await tool.call(
            {"url": "https://api.evil.com/data"}, ctx
        )
        assert result.is_error is True
