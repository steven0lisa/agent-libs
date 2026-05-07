package com.agentlib.examples;

import com.agentlib.*;
import java.util.concurrent.Flow;

/**
 * Example: Enable subagent support and let the model delegate tasks.
 */
public class Subagent {
    public static void main(String[] args) {
        AgentConfig config = AgentConfig.builder()
            .apiKey(System.getenv("ANTHROPIC_AUTH_TOKEN"))
            .enableSubagent(true)
            .subagentMaxTurns(20)
            .build();

        Agent agent = new Agent(config);

        agent.run("Explore this codebase using subagents to parallelize exploration")
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
