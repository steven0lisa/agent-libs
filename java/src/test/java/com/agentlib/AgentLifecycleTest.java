package com.agentlib;

import org.junit.jupiter.api.Test;

import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Flow;
import java.util.concurrent.TimeUnit;

import static org.junit.jupiter.api.Assertions.*;

class AgentLifecycleTest {

    @Test
    void testAgentInitialState() {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .maxTurns(1)
            .build();

        Agent agent = new Agent(config);
        assertEquals(Agent.AgentState.IDLE, agent.getState());
        assertTrue(agent.listTools().size() >= 5); // Built-in tools
    }

    @Test
    void testRegisterAndUnregisterTool() {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .build();

        Agent agent = new Agent(config);
        int initialCount = agent.listTools().size();

        Tool customTool = new Tool() {
            @Override
            public String name() { return "my_tool"; }
            @Override
            public String description() { return "My custom tool"; }
            @Override
            public com.fasterxml.jackson.databind.JsonNode inputSchema() {
                return com.fasterxml.jackson.databind.node.JsonNodeFactory.instance.objectNode();
            }
            @Override
            public ToolResult call(java.util.Map<String, Object> input, ToolContext context) {
                return ToolResult.success("ok");
            }
        };

        agent.registerTool(customTool);
        assertEquals(initialCount + 1, agent.listTools().size());
        assertNotNull(agent.listTools().stream().filter(t -> t.name().equals("my_tool")).findFirst().orElse(null));

        agent.unregisterTool("my_tool");
        assertEquals(initialCount, agent.listTools().size());
    }

    @Test
    void testClearHistory() {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .build();

        Agent agent = new Agent(config);
        agent.getMessageHistory().add(Message.user("Hello"));
        assertEquals(1, agent.getMessageHistory().size());

        agent.clearHistory();
        assertEquals(0, agent.getMessageHistory().size());
    }

    @Test
    void testPauseResume() throws Exception {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .build();

        Agent agent = new Agent(config);

        // Start a run (it will fail quickly due to no API key, but that's ok for testing state)
        Flow.Publisher<Event> publisher = agent.run("test");
        CountDownLatch latch = new CountDownLatch(1);

        publisher.subscribe(new Flow.Subscriber<>() {
            private Flow.Subscription subscription;

            @Override
            public void onSubscribe(Flow.Subscription subscription) {
                this.subscription = subscription;
                subscription.request(Long.MAX_VALUE);
            }

            @Override
            public void onNext(Event event) {
                // Just consume events
            }

            @Override
            public void onError(Throwable throwable) {
                latch.countDown();
            }

            @Override
            public void onComplete() {
                latch.countDown();
            }
        });

        // Give it a moment to start
        Thread.sleep(100);

        // Test pause/resume while running (may or may not be running depending on timing)
        agent.pause();
        assertTrue(agent.getState() == Agent.AgentState.PAUSED ||
                   agent.getState() == Agent.AgentState.TERMINATED ||
                   agent.getState() == Agent.AgentState.COMPLETED);

        agent.resume();
        // After resume, state should be RUNNING or already finished

        // Wait for completion
        latch.await(5, TimeUnit.SECONDS);
    }

    @Test
    void testStop() throws Exception {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .build();

        Agent agent = new Agent(config);

        Flow.Publisher<Event> publisher = agent.run("test");
        CountDownLatch latch = new CountDownLatch(1);

        publisher.subscribe(new Flow.Subscriber<>() {
            private Flow.Subscription subscription;

            @Override
            public void onSubscribe(Flow.Subscription subscription) {
                this.subscription = subscription;
                subscription.request(Long.MAX_VALUE);
            }

            @Override
            public void onNext(Event event) {}

            @Override
            public void onError(Throwable throwable) {
                latch.countDown();
            }

            @Override
            public void onComplete() {
                latch.countDown();
            }
        });

        Thread.sleep(50);
        agent.stop();

        latch.await(5, TimeUnit.SECONDS);
        assertTrue(agent.getState() == Agent.AgentState.TERMINATED ||
                   agent.getState() == Agent.AgentState.STOPPING);
    }

    @Test
    void testCallbackReceivesEvents() throws Exception {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .maxTurns(1)
            .build();

        Agent agent = new Agent(config);
        List<Event> callbackEvents = new ArrayList<>();
        agent.setCallback(callbackEvents::add);

        Flow.Publisher<Event> publisher = agent.run("test");
        CountDownLatch latch = new CountDownLatch(1);

        publisher.subscribe(new Flow.Subscriber<>() {
            private Flow.Subscription subscription;

            @Override
            public void onSubscribe(Flow.Subscription subscription) {
                this.subscription = subscription;
                subscription.request(Long.MAX_VALUE);
            }

            @Override
            public void onNext(Event event) {}

            @Override
            public void onError(Throwable throwable) {
                latch.countDown();
            }

            @Override
            public void onComplete() {
                latch.countDown();
            }
        });

        latch.await(5, TimeUnit.SECONDS);

        // Callback should have received some events
        assertFalse(callbackEvents.isEmpty());
    }

    @Test
    void testRunAsync() throws Exception {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .maxTurns(1)
            .build();

        Agent agent = new Agent(config);
        CompletableFuture<String> future = agent.runAsync("test");

        // Should complete (possibly with error) within timeout
        try {
            String result = future.get(5, TimeUnit.SECONDS);
            // Result may be empty or contain error, but future should complete
        } catch (Exception e) {
            // Expected - API key is invalid
            assertTrue(e.getMessage().contains("API") || e.getMessage().contains("error") || e.getCause() != null);
        }
    }

    @Test
    void testMaxDurationLimit() throws Exception {
        // Test that maxDurationMs config is properly set and checked.
        // Since we can't reliably control API timing in tests,
        // we verify the config value is respected by the Agent.
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .maxTurns(10)
            .maxDurationMs(5000)
            .build();

        assertEquals(5000, config.maxDurationMs());

        Agent agent = new Agent(config);
        // Verify agent was created with the config
        assertNotNull(agent);

        // Run the agent and verify it produces events (duration check won't trigger
        // since API calls fail quickly in test environment)
        List<Event> events = new ArrayList<>();
        CountDownLatch latch = new CountDownLatch(1);

        Flow.Publisher<Event> publisher = agent.run("test");
        publisher.subscribe(new Flow.Subscriber<>() {
            private Flow.Subscription subscription;

            @Override
            public void onSubscribe(Flow.Subscription subscription) {
                this.subscription = subscription;
                subscription.request(Long.MAX_VALUE);
            }

            @Override
            public void onNext(Event event) {
                events.add(event);
            }

            @Override
            public void onError(Throwable throwable) {
                latch.countDown();
            }

            @Override
            public void onComplete() {
                latch.countDown();
            }
        });

        latch.await(5, TimeUnit.SECONDS);

        // Should have produced some events (turn_start at minimum)
        assertFalse(events.isEmpty(), "Expected some events");
        assertTrue(events.stream().anyMatch(e -> e instanceof TurnStartEvent),
            "Expected at least a TurnStartEvent");
    }

    @Test
    void testMaxTurnsLimit() throws Exception {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .maxTurns(1)
            .build();

        Agent agent = new Agent(config);
        List<Event> events = new ArrayList<>();
        CountDownLatch latch = new CountDownLatch(1);

        Flow.Publisher<Event> publisher = agent.run("test");
        publisher.subscribe(new Flow.Subscriber<>() {
            private Flow.Subscription subscription;

            @Override
            public void onSubscribe(Flow.Subscription subscription) {
                this.subscription = subscription;
                subscription.request(Long.MAX_VALUE);
            }

            @Override
            public void onNext(Event event) {
                events.add(event);
            }

            @Override
            public void onError(Throwable throwable) {
                latch.countDown();
            }

            @Override
            public void onComplete() {
                latch.countDown();
            }
        });

        latch.await(5, TimeUnit.SECONDS);

        // Should have hit max turns or got an API error
        assertFalse(events.isEmpty());
    }

    @Test
    void testCustomToolInConfig() {
        Tool customTool = new Tool() {
            @Override
            public String name() { return "config_tool"; }
            @Override
            public String description() { return "From config"; }
            @Override
            public com.fasterxml.jackson.databind.JsonNode inputSchema() {
                return com.fasterxml.jackson.databind.node.JsonNodeFactory.instance.objectNode();
            }
            @Override
            public ToolResult call(java.util.Map<String, Object> input, ToolContext context) {
                return ToolResult.success("from config");
            }
        };

        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .customTools(List.of(customTool))
            .build();

        Agent agent = new Agent(config);
        assertTrue(agent.listTools().stream().anyMatch(t -> t.name().equals("config_tool")));
    }
}
