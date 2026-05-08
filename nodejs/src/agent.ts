/** Agent core with lifecycle management. */

import { AnthropicClient, ToolDefinition } from './client.js';
import {
  AgentConfig, ITool, resolveConfig, ToolContext,
} from './config.js';
import { autoCompactIfNeeded, estimateTokens } from './compact.js';
import { buildSystemPrompt } from './prompt.js';
import {
  ContentBlock, Event, isToolUse, Message,
  userMessage, assistantMessage, messageStartEvent, messageEndEvent,
  messageDeltaEvent, thinkingDeltaEvent, turnStartEvent, errorEvent, completeEvent,
  compactEvent,
} from './types.js';
import { ReadFileTool, WriteFileTool, UpdateFileTool, BashTool, CurlTool, GlobTool, GrepTool, SubAgentTool } from './tools/index.js';
import { SkillLoader, SkillTool } from './skills/index.js';
import type { SkillInfo } from './skills/types.js';

type ToolUseBlock = Extract<ContentBlock, { type: 'tool_use' }>;
type TextBlock = Extract<ContentBlock, { type: 'text' }>;

export enum AgentState {
  IDLE = 'idle',
  RUNNING = 'running',
  PAUSED = 'paused',
  STOPPING = 'stopping',
  TERMINATED = 'terminated',
  COMPLETED = 'completed',
}

export class Agent {
  private config: Required<AgentConfig>;
  private tools: Map<string, ITool> = new Map();
  private messageHistory: Message[] = [];
  private turnCount = 0;
  private startTime: number | null = null;
  private skillLoader: SkillLoader | null = null;
  private loadedSkills: SkillInfo[] = [];

  private _state = AgentState.IDLE;
  private pauseResolve: (() => void) | null = null;
  private pausePromise: Promise<void> | null = null;
  private stopped = false;

  constructor(config: AgentConfig) {
    this.config = resolveConfig(config);
    this.initTools();
  }

  private initTools() {
    this.registerTool(new ReadFileTool());
    this.registerTool(new WriteFileTool());
    this.registerTool(new UpdateFileTool());
    this.registerTool(new BashTool(this.config.bashWhitelist, this.config.bashBlacklist));
    this.registerTool(new CurlTool(this.config.curlWhitelist, this.config.curlBlacklist));
    this.registerTool(new GlobTool());
    this.registerTool(new GrepTool());
    for (const tool of this.config.tools) this.registerTool(tool);

    if (this.config.enableSubagent) {
      this.registerTool(new SubAgentTool(this.config, this.messageHistory, this.tools));
    }

    if (this.config.enableSkills) {
      this.skillLoader = new SkillLoader({
        skillsDir: this.config.skillsDir,
        includeProjectSkills: this.config.includeProjectSkills,
        projectDir: this.config.skillsProjectDir,
      });
      this.registerTool(new SkillTool(this.skillLoader));
    }
  }

  get state(): AgentState { return this._state; }

  pause(): void {
    if (this._state === AgentState.RUNNING) {
      this._state = AgentState.PAUSED;
      this.pausePromise = new Promise((resolve) => { this.pauseResolve = resolve; });
    }
  }

  resume(): void {
    if (this._state === AgentState.PAUSED) {
      this._state = AgentState.RUNNING;
      this.pauseResolve?.();
      this.pauseResolve = null;
      this.pausePromise = null;
    }
  }

  stop(): void {
    this._state = AgentState.STOPPING;
    this.stopped = true;
    this.pauseResolve?.();
    this.pauseResolve = null;
    this.pausePromise = null;
  }

  private checkState(): boolean {
    return this._state !== AgentState.STOPPING && this._state !== AgentState.TERMINATED;
  }

  private async waitIfPaused(): Promise<void> {
    if (this._state === AgentState.PAUSED && this.pausePromise) {
      await this.pausePromise;
    }
  }

  registerTool(tool: ITool): void { this.tools.set(tool.name, tool); }
  unregisterTool(name: string): void { this.tools.delete(name); }
  listTools(): ITool[] { return Array.from(this.tools.values()); }
  getMessageHistory(): Message[] { return [...this.messageHistory]; }
  clearHistory(): void { this.messageHistory = []; this.turnCount = 0; }

  async *run(input: string): AsyncGenerator<Event> {
    this.messageHistory.push(userMessage(input));
    yield* this.runLoop();
  }

  async *chat(messages: Message[]): AsyncGenerator<Event> {
    this.messageHistory.push(...messages);
    yield* this.runLoop();
  }

  private async *runLoop(): AsyncGenerator<Event> {
    this._state = AgentState.RUNNING;
    this.startTime = Date.now();
    this.stopped = false;

    // Load skills lazily on first run loop
    if (this.config.enableSkills && this.skillLoader && this.loadedSkills.length === 0) {
      this.loadedSkills = this.skillLoader.discoverAll();
    }

    const systemPrompt = buildSystemPrompt(
      this.tools,
      this.config.systemPrompt,
      this.config.enableSubagent,
      this.config.subagentMaxTurns,
      this.loadedSkills,
    );
    const client = new AnthropicClient(this.config);
    let consecutiveCompactFailures = 0;

    try {
      while (this.turnCount < this.config.maxTurns) {
        if (!this.checkState()) break;
        await this.waitIfPaused();
        if (!this.checkState()) break;

        this.turnCount++;
        yield turnStartEvent(this.turnCount);
        this.config.callback?.(turnStartEvent(this.turnCount));

        if (this.config.maxDurationMs > 0 && this.startTime) {
          const elapsed = Date.now() - this.startTime;
          if (elapsed > this.config.maxDurationMs) {
            yield errorEvent('Max duration exceeded');
            break;
          }
        }

        const toolDefs: ToolDefinition[] = Array.from(this.tools.values()).map(t => ({
          name: t.name,
          description: t.description,
          input_schema: t.inputSchema,
        }));

        // Auto compact
        if (this.config.autoCompact) {
          const result = await autoCompactIfNeeded(this.config, client, this.messageHistory, consecutiveCompactFailures);
          if (result.history.length !== this.messageHistory.length) {
            this.messageHistory = result.history;
            const estimated = estimateTokens(this.messageHistory);
            const e = compactEvent(this.messageHistory.length, estimated);
            yield e;
            this.config.callback?.(e);
          }
          consecutiveCompactFailures = result.failures;
        }

        await this.waitIfPaused();
        if (!this.checkState()) break;

        const assistantContent: ContentBlock[] = [];
        yield messageStartEvent();
        this.config.callback?.(messageStartEvent());

        try {
          for await (const event of client.streamMessages(this.messageHistory, systemPrompt, toolDefs)) {
            if (!this.checkState()) break;
            await this.waitIfPaused();

            const type = event.type as string;
            if (type === 'content_block_start') {
              assistantContent.push(this.parseBlock(event.content_block as Record<string, unknown>));
            } else if (type === 'content_block_delta') {
              const delta = event.delta as Record<string, unknown>;
              if (delta.type === 'text_delta') {
                const e = messageDeltaEvent(delta.text as string);
                yield e;
                this.config.callback?.(e);
              } else if (delta.type === 'thinking_delta') {
                const e = thinkingDeltaEvent(delta.thinking as string);
                yield e;
                this.config.callback?.(e);
              }
            } else if (type === 'message_stop') {
              yield messageEndEvent();
              this.config.callback?.(messageEndEvent());
            }
          }
        } catch (e) {
          yield errorEvent(`API error: ${e}`);
          break;
        }

        if (!this.checkState()) break;

        this.messageHistory.push(assistantMessage(assistantContent));
        const toolUses = assistantContent.filter(isToolUse);

        if (toolUses.length === 0) {
          const finalText = this.extractText(assistantContent);
          this._state = AgentState.COMPLETED;
          const e = completeEvent(finalText);
          yield e;
          this.config.callback?.(e);
          return;
        }

        const results = await this.executeTools(toolUses);
        if (!this.checkState()) break;
        this.messageHistory.push({ role: 'user', content: results });
      }

      if (this.turnCount >= this.config.maxTurns) {
        yield errorEvent('Max turns reached');
      }
    } finally {
      if (this._state !== AgentState.COMPLETED) {
        this._state = AgentState.TERMINATED;
      }
    }
  }

  private async executeTools(toolUses: ToolUseBlock[]): Promise<ContentBlock[]> {
    const readOnly: ToolUseBlock[] = [];
    const write: ToolUseBlock[] = [];
    for (const block of toolUses) {
      const tool = this.tools.get(block.name);
      if (tool?.isReadOnly) readOnly.push(block); else write.push(block);
    }

    const results: ContentBlock[] = [];

    if (readOnly.length > 0) {
      const readResults = await Promise.all(readOnly.map(b => this.executeSingleTool(b)));
      results.push(...readResults);
    }

    for (const block of write) {
      if (!this.checkState()) break;
      results.push(await this.executeSingleTool(block));
    }

    return results;
  }

  private async executeSingleTool(block: ToolUseBlock): Promise<ContentBlock> {
    const tool = this.tools.get(block.name);
    if (!tool) {
      return { type: 'tool_result', tool_use_id: block.id, content: `Tool not found: ${block.name}`, is_error: true };
    }

    const context: ToolContext = {
      workDir: this.config.workDir,
      messageHistory: [...this.messageHistory],
    };

    try {
      const result = await tool.call(block.input, context);
      return { type: 'tool_result', tool_use_id: block.id, content: result.content, is_error: result.isError };
    } catch (e) {
      return { type: 'tool_result', tool_use_id: block.id, content: String(e), is_error: true };
    }
  }

  private extractText(blocks: ContentBlock[]): string {
    return blocks.filter((b): b is TextBlock => b.type === 'text').map((b) => b.text).join('');
  }

  private parseBlock(data: Record<string, unknown>): ContentBlock {
    const type = data.type as string;
    if (type === 'text') return { type: 'text', text: data.text as string };
    if (type === 'tool_use') return { type: 'tool_use', name: data.name as string, id: data.id as string, input: (data.input as Record<string, unknown>) || {} };
    if (type === 'thinking') return { type: 'thinking', thinking: data.thinking as string };
    throw new Error(`Unknown block type: ${type}`);
  }
}
