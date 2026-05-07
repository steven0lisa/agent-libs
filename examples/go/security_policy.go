// Example: Use whitelist/blacklist security policies for bash and curl.
package main

import (
	"context"
	"fmt"
	"os"

	"github.com/steven0lisa/agent-libs/go"
)

func main() {
	cfg := agentlib.DefaultConfig()
	cfg.APIKey = os.Getenv("ANTHROPIC_AUTH_TOKEN")

	// Bash: allow git and ls; block rm.
	cfg.BashWhitelist = []agentlib.Pattern{
		{Pattern: "git *", Type: "wildcard"},
		{Pattern: "ls *", Type: "wildcard"},
	}
	cfg.BashBlacklist = []agentlib.Pattern{
		{Pattern: "rm *", Type: "wildcard"},
		{Pattern: "^dd\\s", Type: "regex"},
	}

	// Curl: allow example.com; block known bad domains.
	cfg.CurlWhitelist = []agentlib.Pattern{
		{Pattern: "*.example.com/*", Type: "wildcard"},
	}
	cfg.CurlBlacklist = []agentlib.Pattern{
		{Pattern: "evil\\.com|malicious\\.org", Type: "regex"},
	}

	agent := agentlib.NewAgent(cfg)
	ctx := context.Background()

	for event := range agent.Run(ctx, "Show me the git status of this repository") {
		switch event.Type {
		case agentlib.EventMessageDelta:
			fmt.Print(event.Data["text"])
		case agentlib.EventComplete:
			fmt.Printf("\n\n[Complete] %s\n", event.Data["final_content"])
		case agentlib.EventError:
			fmt.Printf("\n[Error] %s\n", event.Data["message"])
		}
	}
}
