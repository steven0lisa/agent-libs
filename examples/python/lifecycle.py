"""Example: Manage agent lifecycle - pause, resume, and stop."""

import asyncio
import os

from agentlib import Agent, AgentConfig


async def main():
    config = AgentConfig(
        api_key=os.environ.get("ANTHROPIC_AUTH_TOKEN", ""),
        model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
    )

    agent = Agent(config)

    # Schedule a pause after 3 seconds, then resume after 5 seconds.
    async def lifecycle_manager():
        await asyncio.sleep(3)
        print("\n[Lifecycle] Pausing agent...")
        agent.pause()

        await asyncio.sleep(2)
        print("\n[Lifecycle] Resuming agent...")
        agent.resume()

        # Stop after 10 seconds total.
        await asyncio.sleep(5)
        print("\n[Lifecycle] Stopping agent...")
        agent.stop()

    asyncio.create_task(lifecycle_manager())

    async for event in agent.run("Please explore the codebase and tell me about it"):
        if event.type.value == "turn_start":
            print(f"\n[Turn {event.data['turn']} started]")
        elif event.type.value == "message_delta":
            print(event.data["text"], end="", flush=True)
        elif event.type.value == "complete":
            print(f"\n\n[Complete] {event.data['final_content']}")
        elif event.type.value == "error":
            print(f"\n[Error] {event.data['message']}")

    print(f"\n[Final state] {agent.state.value}")


if __name__ == "__main__":
    asyncio.run(main())
