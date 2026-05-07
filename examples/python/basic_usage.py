"""Basic usage example: Run the agent with a simple prompt."""

import asyncio
import os

from agentlib import Agent, AgentConfig


async def main():
    # Configure the agent with API credentials from environment variables.
    # The caller must set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY.
    config = AgentConfig(
        api_key=os.environ.get("ANTHROPIC_AUTH_TOKEN", ""),
        base_url=os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com"),
        model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
        work_dir=os.getcwd(),
    )

    agent = Agent(config)

    # Run the agent with a user prompt.
    async for event in agent.run("Please list the files in the current directory"):
        if event.type.value == "message_delta":
            print(event.data["text"], end="", flush=True)
        elif event.type.value == "complete":
            print("\n\n[Complete] Final content:")
            print(event.data["final_content"])
        elif event.type.value == "error":
            print(f"\n[Error] {event.data['message']}")


if __name__ == "__main__":
    asyncio.run(main())
