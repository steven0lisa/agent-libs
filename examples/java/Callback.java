package com.agentlib.examples;

import com.agentlib.*;
import java.util.function.Consumer;
import java.util.concurrent.Flow;

/**
 * Example: Use a callback to log every step of the agent's execution.
 */
public class Callback {
    public static void main(String[] args) {
        Consumer<Event> onEvent = event -> {
            switch (event) {
                case ToolUseStartEvent e ->
                    System.out.println("[Callback] Tool '" + e.getName() + "' called");
                case ToolUseEndEvent e -> {
                    String status = e.getResult().isError() ? "ERROR" : "OK";
                    System.out.println("[Callback] Tool '" + e.getName() + "' finished: " + status);
                }
                case ErrorEvent e ->
                    System.out.println("[Callback] Error: " + e.getMessage());
                default -> {}
            }
        };

        AgentConfig config = AgentConfig.builder()
            .apiKey(System.getenv("ANTHROPIC_AUTH_TOKEN"))
            .callback(onEvent)
            .build();

        Agent agent = new Agent(config);

        agent.run("Read README.md and summarize it").subscribe(new Flow.Subscriber<Event>() {
            private Flow.Subscription subscription;

            @Override public void onSubscribe(Flow.Subscription s) {
                this.subscription = s;
                s.request(Long.MAX_VALUE);
            }

            @Override public void onNext(Event event) {
                switch (event) {
                    case MessageDeltaEvent e -> System.out.print(e.getText());
                    case CompleteEvent e -> System.out.println("\n\n[Complete] " + e.getFinalContent());
                    default -> {}
                }
            }

            @Override public void onError(Throwable t) { t.printStackTrace(); }
            @Override public void onComplete() {}
        });
    }
}
