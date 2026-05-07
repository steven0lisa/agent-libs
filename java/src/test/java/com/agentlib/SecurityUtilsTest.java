package com.agentlib;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class SecurityUtilsTest {

    @TempDir
    Path tempDir;

    @Test
    void testResolveSafePathWithinWorkDir() {
        Path result = SecurityUtils.resolveSafePath("file.txt", tempDir);
        assertEquals(tempDir.resolve("file.txt").normalize(), result);
    }

    @Test
    void testResolveSafePathSubdirectory() {
        Path result = SecurityUtils.resolveSafePath("sub/dir/file.txt", tempDir);
        assertEquals(tempDir.resolve("sub/dir/file.txt").normalize(), result);
    }

    @Test
    void testResolveSafePathEscapesWorkDir() {
        AgentException.SecurityException ex = assertThrows(
            AgentException.SecurityException.class,
            () -> SecurityUtils.resolveSafePath("../escape.txt", tempDir)
        );
        assertTrue(ex.getMessage().contains("escapes working directory"));
    }

    @Test
    void testResolveSafePathDoubleDotEscapes() {
        assertThrows(
            AgentException.SecurityException.class,
            () -> SecurityUtils.resolveSafePath("a/../../escape.txt", tempDir)
        );
    }

    @Test
    void testCheckSecurityPolicyWhitelistFirst() {
        List<Pattern> whitelist = List.of(new Pattern("ls *"));
        List<Pattern> blacklist = List.of(new Pattern("ls -la"));

        // Whitelist takes priority
        SecurityUtils.SecurityCheckResult result = SecurityUtils.checkSecurityPolicy(
            "ls -la", whitelist, blacklist, true
        );
        assertTrue(result.allowed());
        assertTrue(result.reason().contains("whitelist"));
    }

    @Test
    void testCheckSecurityPolicyBlacklistBlocks() {
        List<Pattern> whitelist = List.of();
        List<Pattern> blacklist = List.of(new Pattern("rm *"));

        SecurityUtils.SecurityCheckResult result = SecurityUtils.checkSecurityPolicy(
            "rm -rf /", whitelist, blacklist, true
        );
        assertFalse(result.allowed());
        assertTrue(result.reason().contains("blacklist"));
    }

    @Test
    void testCheckSecurityPolicyDefaultAllow() {
        List<Pattern> whitelist = List.of();
        List<Pattern> blacklist = List.of();

        SecurityUtils.SecurityCheckResult result = SecurityUtils.checkSecurityPolicy(
            "echo hello", whitelist, blacklist, true
        );
        assertTrue(result.allowed());
        assertEquals("Default allow", result.reason());
    }

    @Test
    void testCheckSecurityPolicyDefaultDeny() {
        List<Pattern> whitelist = List.of();
        List<Pattern> blacklist = List.of();

        SecurityUtils.SecurityCheckResult result = SecurityUtils.checkSecurityPolicy(
            "echo hello", whitelist, blacklist, false
        );
        assertFalse(result.allowed());
        assertEquals("Default deny", result.reason());
    }

    @Test
    void testCheckSecurityPolicyRegexBlacklist() {
        List<Pattern> whitelist = List.of();
        List<Pattern> blacklist = List.of(new Pattern("rm.*-rf.*", "regex"));

        SecurityUtils.SecurityCheckResult result = SecurityUtils.checkSecurityPolicy(
            "rm -rf /tmp", whitelist, blacklist, true
        );
        assertFalse(result.allowed());
    }
}
