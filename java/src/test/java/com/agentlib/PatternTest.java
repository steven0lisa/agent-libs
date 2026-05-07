package com.agentlib;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class PatternTest {

    @Test
    void testWildcardExactMatch() {
        Pattern p = new Pattern("hello");
        assertTrue(p.matches("hello"));
        assertFalse(p.matches("hello world"));
        assertFalse(p.matches("hell"));
    }

    @Test
    void testWildcardStar() {
        Pattern p = new Pattern("hello*");
        assertTrue(p.matches("hello"));
        assertTrue(p.matches("hello world"));
        assertTrue(p.matches("helloworld"));
        assertFalse(p.matches("hi hello"));
    }

    @Test
    void testWildcardStarPrefix() {
        Pattern p = new Pattern("*.txt");
        assertTrue(p.matches("file.txt"));
        assertTrue(p.matches("a.txt"));
        assertFalse(p.matches("file.pdf"));
    }

    @Test
    void testWildcardStarBothSides() {
        Pattern p = new Pattern("*hello*");
        assertTrue(p.matches("hello"));
        assertTrue(p.matches("say hello"));
        assertTrue(p.matches("hello world"));
        assertTrue(p.matches("say hello world"));
        assertFalse(p.matches("hell"));
    }

    @Test
    void testWildcardQuestionMark() {
        Pattern p = new Pattern("h?llo");
        assertTrue(p.matches("hello"));
        assertTrue(p.matches("hallo"));
        assertFalse(p.matches("hllo"));
        assertFalse(p.matches("heello"));
    }

    @Test
    void testRegexPattern() {
        Pattern p = new Pattern("rm -rf /", "regex");
        assertTrue(p.matches("rm -rf /"));
        assertFalse(p.matches("rm -rf /tmp"));
    }

    @Test
    void testRegexPatternWithWildcard() {
        Pattern p = new Pattern("rm.*-rf.*", "regex");
        assertTrue(p.matches("rm -rf /"));
        assertTrue(p.matches("rm -rf /tmp"));
        assertFalse(p.matches("ls -la"));
    }

    @Test
    void testNullText() {
        Pattern p = new Pattern("hello");
        assertFalse(p.matches(null));
    }

    @Test
    void testDefaultTypeIsWildcard() {
        Pattern p1 = new Pattern("test*");
        Pattern p2 = new Pattern("test*", "wildcard");
        assertEquals(p1.type(), p2.type());
        assertTrue(p1.matches("testing"));
        assertTrue(p2.matches("testing"));
    }
}
