package main

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"

	agentlib "github.com/steven0lisa/agent-libs/go"
)

func main() {
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		fmt.Println("ERROR: cannot resolve current file path")
		return
	}

	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(filename), "..", "..", ".."))
	workDir := filepath.Join(repoRoot, "test", "case01")

	promptPath := filepath.Join(workDir, "prompt.txt")
	promptBytes, err := os.ReadFile(promptPath)
	if err != nil {
		fmt.Printf("Error reading prompt: %v\n", err)
		return
	}
	prompt := string(promptBytes)

	fmt.Println("=== Prompt ===")
	fmt.Println(prompt)
	fmt.Println()

	dataPath := filepath.Join(workDir, "data.txt")
	dataBytes, err := os.ReadFile(dataPath)
	if err != nil {
		fmt.Printf("Error reading data: %v\n", err)
		return
	}
	data := string(dataBytes)
	fmt.Println("=== Data Preview ===")
	lines := strings.Split(data, "\n")
	for i := 0; i < 25 && i < len(lines); i++ {
		fmt.Println(lines[i])
	}
	fmt.Println("...")
	fmt.Println()

	apiKey := os.Getenv("ANTHROPIC_AUTH_TOKEN")
	if apiKey == "" {
		apiKey = os.Getenv("ANTHROPIC_API_KEY")
	}
	if apiKey == "" {
		fmt.Println("ERROR: ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY not set")
		return
	}

	cfg := agentlib.DefaultConfig()
	cfg.APIKey = apiKey
	if baseURL := os.Getenv("ANTHROPIC_BASE_URL"); baseURL != "" {
		cfg.BaseURL = baseURL
	}
	if model := os.Getenv("ANTHROPIC_MODEL"); model != "" {
		cfg.Model = model
	}
	cfg.WorkDir = workDir
	cfg.MaxTurns = 10

	agent := agentlib.NewAgent(cfg)
	fmt.Println("=== Agent Output ===")
	fmt.Println()

	ctx := context.Background()
	events, err := agent.Run(ctx, prompt)
	if err != nil {
		fmt.Printf("Error starting agent: %v\n", err)
		return
	}

	for event := range events {
		switch event.Type {
		case agentlib.EventMessageDelta:
			fmt.Print(event.Data["text"])
		case agentlib.EventToolUseStart:
			fmt.Printf("\n[Tool call: %s] input=%v\n", event.Data["name"], event.Data["input"])
		case agentlib.EventComplete:
			fmt.Printf("\n\n=== Complete ===\n%v\n", event.Data["final_content"])
		case agentlib.EventError:
			fmt.Printf("\n[Error] %v\n", event.Data["message"])
		}
	}

	fmt.Printf("\n[Final state] %s\n", agent.State())
}
