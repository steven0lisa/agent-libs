package com.agentlib.examples;

import com.agentlib.*;
import java.util.List;
import java.util.concurrent.Flow;

/**
 * Example: Use whitelist/blacklist security policies for bash and curl.
 */
public class SecurityPolicy {
    public static void main(String[] args) {
        AgentConfig config = AgentConfig.builder()
            .apiKey(System.getenv("ANTHROPIC_AUTH_TOKEN"))
            .bashWhitelist(List.of(
                new Pattern("git *", PatternType.WILDCARD),
                new Pattern("ls *", PatternType.WILDCARD)
            ))
            .bashBlacklist(List.of(
                new Pattern("rm *", PatternType.WILDCARD),
                new Pattern("^dd\\s", PatternType.REGEX)
            ))
            .curlWhitelist(List.of(
                new Pattern("*.example.com/*", PatternType.WILDCARD)
            ))
            .curlBlacklist(List.of(
                new Pattern("evil\\.com|malicious\\.org", PatternType.REGEX)
            ))
            .build();

        Agent agent = new Agent(config);

        agent.run("Show me the git status of this repository").subscribe(new Flow.Subscriber<Event>() {
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
