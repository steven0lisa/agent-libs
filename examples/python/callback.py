"""Example: Use a callback to log every step of the agent's execution."""

import asyncio
import os

from agentlib import Agent, AgentConfig, Event


async def main():
    # Callback receives every event for logging or monitoring.
    def on_event(event: Event) -> None:
        match event.type.value:
            case "turn_start":
                print(f"\n[Callback] Turn {event.data['turn']} started")
            case "tool_use_start":
                print(f"[Callback] Tool '{event.data['name']}' called (id={event.data['id']})")
            case "tool_use_end":
                result = event.data["result"]
                status = "OK" if not result.is_error else "ERROR"
                print(f"[Callback] Tool '{event.data['name']}' finished: {status}")
            case "error":
                print(f"[Callback] Error: {event.data['message']}")
            case _:
                pass

    config = AgentConfig(
        api_key=os.environ.get("ANTHROPIC_AUTH_TOKEN", ""),
        model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
        callback=on_event,
    )

    agent = Agent(config)

    async for event in agent.run("Read the file README.md and summarize it"):
        if event.type.value == "message_delta":
            print(event.data["text"], end="", flush=True)
        elif event.type.value == "complete":
            print(f"\n\n[Complete] {event.data['final_content']}")


if __name__ == "__main__":
    asyncio.run(main())
