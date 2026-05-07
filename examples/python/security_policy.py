"""Example: Use whitelist/blacklist security policies for bash and curl."""

import asyncio
import os

from agentlib import Agent, AgentConfig, Pattern


async def main():
    # Only allow git commands and ls; block rm and any destructive commands.
    config = AgentConfig(
        api_key=os.environ.get("ANTHROPIC_AUTH_TOKEN", ""),
        model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
        # Bash: whitelist allows git and ls; blacklist blocks rm
        bash_whitelist=[
            Pattern("git *", "wildcard"),
            Pattern("ls *", "wildcard"),
        ],
        bash_blacklist=[
            Pattern("rm *", "wildcard"),
            Pattern(r"^dd\s", "regex"),
        ],
        # Curl: only allow example.com subdomains; block known bad domains.
        curl_whitelist=[
            Pattern("*.example.com/*", "wildcard"),
            Pattern("https://api.example.com/*", "wildcard"),
        ],
        curl_blacklist=[
            Pattern(r"evil\.com|malicious\.org", "regex"),
        ],
    )

    agent = Agent(config)

    # The model can still see and request any tool, but blocked commands
    # will return a tool_result with an error, allowing the model to adapt.
    async for event in agent.run("Show me the git status of this repository"):
        if event.type.value == "message_delta":
            print(event.data["text"], end="", flush=True)
        elif event.type.value == "complete":
            print(f"\n\n[Complete] {event.data['final_content']}")
        elif event.type.value == "error":
            print(f"\n[Error] {event.data['message']}")


if __name__ == "__main__":
    asyncio.run(main())
