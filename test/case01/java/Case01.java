import com.agentlib.Agent;
import com.agentlib.AgentConfig;
import com.agentlib.CompleteEvent;
import com.agentlib.ErrorEvent;
import com.agentlib.Event;
import com.agentlib.MessageDeltaEvent;
import com.agentlib.ToolUseStartEvent;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Flow;
import java.util.concurrent.TimeUnit;

public class Case01 {
    public static void main(String[] args) throws Exception {
        Path workDir = findWorkDir();
        Path promptPath = workDir.resolve("prompt.txt");
        Path dataPath = workDir.resolve("data.txt");

        String prompt = Files.readString(promptPath);
        String data = Files.readString(dataPath);

        System.out.println("=== Prompt ===");
        System.out.println(prompt);
        System.out.println();

        System.out.println("=== Data Preview ===");
        var lines = data.lines().toList();
        for (int i = 0; i < Math.min(25, lines.size()); i++) {
            System.out.println(lines.get(i));
        }
        System.out.println("...\n");

        String apiKey = System.getenv("ANTHROPIC_AUTH_TOKEN");
        if (apiKey == null || apiKey.isEmpty()) {
            apiKey = System.getenv("ANTHROPIC_API_KEY");
        }
        if (apiKey == null || apiKey.isEmpty()) {
            System.out.println("ERROR: ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY not set");
            return;
        }

        AgentConfig config = AgentConfig.builder()
            .apiKey(apiKey)
            .baseUrl(System.getenv().getOrDefault("ANTHROPIC_BASE_URL", "https://api.anthropic.com"))
            .model(System.getenv().getOrDefault("ANTHROPIC_MODEL", "claude-sonnet-4-6"))
            .workDir(workDir)
            .maxTurns(10)
            .build();

        Agent agent = new Agent(config);
        System.out.println("=== Agent Output ===\n");

        CountDownLatch latch = new CountDownLatch(1);
        agent.run(prompt).subscribe(new Flow.Subscriber<>() {
            @Override
            public void onSubscribe(Flow.Subscription subscription) {
                subscription.request(Long.MAX_VALUE);
            }

            @Override
            public void onNext(Event event) {
                if (event instanceof MessageDeltaEvent e) {
                    System.out.print(e.text());
                } else if (event instanceof ToolUseStartEvent e) {
                    System.out.println("\n[Tool call: " + e.name() + "] input=" + e.input());
                } else if (event instanceof CompleteEvent e) {
                    System.out.println("\n\n=== Complete ===");
                    System.out.println(e.finalContent());
                    latch.countDown();
                } else if (event instanceof ErrorEvent e) {
                    System.out.println("\n[Error] " + e.message());
                    latch.countDown();
                }
            }

            @Override
            public void onError(Throwable throwable) {
                throwable.printStackTrace();
                latch.countDown();
            }

            @Override
            public void onComplete() {
                latch.countDown();
            }
        });

        latch.await(60, TimeUnit.SECONDS);
        System.out.println("\n[Final state] " + agent.getState());
    }

    private static Path findWorkDir() {
        Path cwd = Paths.get("").toAbsolutePath();
        for (int i = 0; i < 8; i++) {
            Path candidate = cwd.resolve("test").resolve("case01");
            if (Files.exists(candidate.resolve("prompt.txt")) && Files.exists(candidate.resolve("data.txt"))) {
                return candidate;
            }
            cwd = cwd.getParent();
            if (cwd == null) {
                break;
            }
        }
        throw new IllegalStateException("Cannot locate test/case01 directory from current working directory");
    }
}
