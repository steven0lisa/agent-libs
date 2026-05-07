package com.agentlib.examples;

import com.agentlib.*;
import java.util.concurrent.Flow;

/**
 * Example: Manage agent lifecycle - pause, resume, and stop.
 */
public class Lifecycle {
    public static void main(String[] args) {
        AgentConfig config = AgentConfig.builder()
            .apiKey(System.getenv("ANTHROPIC_AUTH_TOKEN"))
            .build();

        Agent agent = new Agent(config);

        // Schedule lifecycle changes.
        new Thread(() -> {
            try {
                Thread.sleep(3000);
                System.out.println("\n[Lifecycle] Pausing agent...");
                agent.pause();

                Thread.sleep(2000);
                System.out.println("\n[Lifecycle] Resuming agent...");
                agent.resume();

                Thread.sleep(5000);
                System.out.println("\n[Lifecycle] Stopping agent...");
                agent.stop();
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
        }).start();

        agent.run("Explore the codebase and tell me about it").subscribe(new Flow.Subscriber<Event>() {
            private Flow.Subscription subscription;

            @Override public void onSubscribe(Flow.Subscription s) {
                this.subscription = s;
                s.request(Long.MAX_VALUE);
            }

            @Override public void onNext(Event event) {
                switch (event) {
                    case TurnStartEvent e -> System.out.println("\n[Turn " + e.getTurn() + " started]");
                    case MessageDeltaEvent e -> System.out.print(e.getText());
                    case CompleteEvent e -> System.out.println("\n\n[Complete] " + e.getFinalContent());
                    case ErrorEvent e -> System.err.println("\n[Error] " + e.getMessage());
                    default -> {}
                }
            }

            @Override public void onError(Throwable t) { t.printStackTrace(); }
            @Override public void onComplete() {}
        });

        System.out.println("\n[Final state] " + agent.getState());
    }
}
