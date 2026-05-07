import asyncio
import os
import sys
from pathlib import Path


async def main() -> None:
    work_dir = Path(__file__).resolve().parent.parent
    repo_root = work_dir.parent.parent
    sys.path.insert(0, str(repo_root / "python" / "src"))

    from agentlib import Agent, AgentConfig

    prompt_path = work_dir / "prompt.txt"
    prompt = prompt_path.read_text(encoding="utf-8")
    print("=== Prompt ===")
    print(prompt)
    print()

    data_path = work_dir / "data.txt"
    data = data_path.read_text(encoding="utf-8")
    print("=== Data Preview ===")
    print("\n".join(data.split("\n")[:25]))
    print("...\n")

    api_key = os.environ.get("ANTHROPIC_AUTH_TOKEN") or os.environ.get(
        "ANTHROPIC_API_KEY", ""
    )
    if not api_key:
        print("ERROR: ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY not set")
        return

    config = AgentConfig(
        api_key=api_key,
        base_url=os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com"),
        model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
        work_dir=str(work_dir),
        max_turns=10,
    )

    agent = Agent(config)
    print("=== Agent Output ===\n")

    async for event in agent.run(prompt):
        if event.type.value == "message_delta":
            print(event.data["text"], end="", flush=True)
        elif event.type.value == "tool_use_start":
            print(f"\n[Tool call: {event.data['name']}] input={event.data['input']}")
        elif event.type.value == "complete":
            print("\n\n=== Complete ===")
            print(event.data["final_content"])
        elif event.type.value == "error":
            print(f"\n[Error] {event.data['message']}")

    print(f"\n\n[Final state] {agent.state.value}")


if __name__ == "__main__":
    asyncio.run(main())
