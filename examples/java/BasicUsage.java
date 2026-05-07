package com.agentlib.examples;

import com.agentlib.*;
import java.util.concurrent.Flow;

/**
 * Basic usage example: Run the agent with a simple prompt.
 */
public class BasicUsage {
    public static void main(String[] args) {
        AgentConfig config = AgentConfig.builder()
            .apiKey(System.getenv("ANTHROPIC_AUTH_TOKEN"))
            .baseUrl(System.getenv().getOrDefault("ANTHROPIC_BASE_URL", "https://api.anthropic.com"))
            .model(System.getenv().getOrDefault("ANTHROPIC_MODEL", "claude-sonnet-4-6"))
            .build();

        Agent agent = new Agent(config);
        agent.run("Please list the files in the current directory")
            .subscribe(new Flow.Subscriber<Event>() {
                private Flow.Subscription subscription;

                @Override public void onSubscribe(Flow.Subscription s) {
                    this.subscription = s;
                    s.request(Long.MAX_VALUE);
                }

                @Override public void onNext(Event event) {
                    switch (event) {
                        case MessageDeltaEvent e -> System.out.print(e.getText());
                        case CompleteEvent e -> System.out.println("\n\n[Complete] " + e.getFinalContent());
                        case ErrorEvent e -> System.err.println("\n[Error] " + e.getMessage());
                        default -> {}
                    }
                }

                @Override public void onError(Throwable t) { t.printStackTrace(); }
                @Override public void onComplete() {}
            });
    }
}
