// Example: Manage agent lifecycle - pause, resume, and stop.
package main

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/steven0lisa/agent-libs/go"
)

func main() {
	cfg := agentlib.DefaultConfig()
	cfg.APIKey = os.Getenv("ANTHROPIC_AUTH_TOKEN")
	agent := agentlib.NewAgent(cfg)

	// Schedule lifecycle changes in background.
	go func() {
		time.Sleep(3 * time.Second)
		fmt.Println("\n[Lifecycle] Pausing agent...")
		agent.Pause()

		time.Sleep(2 * time.Second)
		fmt.Println("\n[Lifecycle] Resuming agent...")
		agent.Resume()

		time.Sleep(5 * time.Second)
		fmt.Println("\n[Lifecycle] Stopping agent...")
		agent.Stop()
	}()

	ctx := context.Background()
	for event := range agent.Run(ctx, "Explore the codebase and tell me about it") {
		switch event.Type {
		case agentlib.EventTurnStart:
			fmt.Printf("\n[Turn %d started]\n", event.Data["turn"])
		case agentlib.EventMessageDelta:
			fmt.Print(event.Data["text"])
		case agentlib.EventComplete:
			fmt.Printf("\n\n[Complete] %s\n", event.Data["final_content"])
		case agentlib.EventError:
			fmt.Printf("\n[Error] %s\n", event.Data["message"])
		}
	}

	fmt.Printf("\n[Final state] %s\n", agent.State())
}
