package com.agentlib;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class AgentExceptionTest {

    @Test
    void testBasicException() {
        AgentException ex = new AgentException("Something went wrong");
        assertEquals("Something went wrong", ex.getMessage());
    }

    @Test
    void testExceptionWithCause() {
        Throwable cause = new RuntimeException("Root cause");
        AgentException ex = new AgentException("Wrapped", cause);
        assertEquals("Wrapped", ex.getMessage());
        assertEquals(cause, ex.getCause());
    }

    @Test
    void testToolNotFoundException() {
        AgentException.ToolNotFoundException ex = new AgentException.ToolNotFoundException("my_tool");
        assertTrue(ex.getMessage().contains("my_tool"));
        assertTrue(ex.getMessage().contains("not found"));
    }

    @Test
    void testSecurityException() {
        AgentException.SecurityException ex = new AgentException.SecurityException("path escape");
        assertTrue(ex.getMessage().contains("Security violation"));
        assertTrue(ex.getMessage().contains("path escape"));
    }

    @Test
    void testMaxTurnsExceededException() {
        AgentException.MaxTurnsExceededException ex = new AgentException.MaxTurnsExceededException(100);
        assertTrue(ex.getMessage().contains("100"));
    }

    @Test
    void testMaxDurationExceededException() {
        AgentException.MaxDurationExceededException ex = new AgentException.MaxDurationExceededException(5000);
        assertTrue(ex.getMessage().contains("5000ms"));
    }

    @Test
    void testApiException() {
        AgentException.ApiException ex = new AgentException.ApiException("HTTP 500");
        assertTrue(ex.getMessage().contains("API error"));
        assertTrue(ex.getMessage().contains("HTTP 500"));
    }

    @Test
    void testApiExceptionWithCause() {
        Throwable cause = new java.io.IOException("Connection refused");
        AgentException.ApiException ex = new AgentException.ApiException("Failed", cause);
        assertEquals(cause, ex.getCause());
    }
}
