"""Example: Register a custom tool with the agent."""

import asyncio
import os

from agentlib import Agent, AgentConfig, Tool, ToolContext, ToolResult


class CalculatorTool(Tool):
    """A custom tool that performs basic arithmetic."""

    @property
    def name(self) -> str:
        return "calculator"

    @property
    def description(self) -> str:
        return "Perform basic arithmetic: add, subtract, multiply, divide."

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                },
                "a": {"type": "number"},
                "b": {"type": "number"},
            },
            "required": ["operation", "a", "b"],
        }

    @property
    def is_read_only(self) -> bool:
        return True

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        operation = input.get("operation")
        a = input.get("a", 0)
        b = input.get("b", 0)

        try:
            if operation == "add":
                result = a + b
            elif operation == "subtract":
                result = a - b
            elif operation == "multiply":
                result = a * b
            elif operation == "divide":
                if b == 0:
                    return ToolResult.error("Cannot divide by zero")
                result = a / b
            else:
                return ToolResult.error(f"Unknown operation: {operation}")

            return ToolResult.success(str(result))
        except Exception as e:
            return ToolResult.error(str(e))


async def main():
    config = AgentConfig(
        api_key=os.environ.get("ANTHROPIC_AUTH_TOKEN", ""),
        model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
    )

    agent = Agent(config)
    agent.register_tool(CalculatorTool())

    async for event in agent.run("What is 123 multiplied by 456?"):
        if event.type.value == "message_delta":
            print(event.data["text"], end="", flush=True)
        elif event.type.value == "complete":
            print(f"\n\n[Complete] {event.data['final_content']}")
        elif event.type.value == "error":
            print(f"\n[Error] {event.data['message']}")


if __name__ == "__main__":
    asyncio.run(main())
