/** Variable substitution for skill content. */

export interface SubstitutionContext {
  args?: string;
  skillDir?: string;
}

/**
 * Substitute variables in skill content:
 * - $ARGUMENTS       → user-provided arguments
 * - ${CLAUDE_SKILL_DIR} → skill directory path
 * - ${ENV:VAR_NAME}  → environment variable
 */
export function substituteVariables(content: string, ctx: SubstitutionContext): string {
  let result = content;

  if (ctx.args !== undefined) {
    result = result.replace(/\$ARGUMENTS/g, ctx.args);
  }

  if (ctx.skillDir !== undefined) {
    result = result.replace(/\$\{CLAUDE_SKILL_DIR\}/g, ctx.skillDir);
  }

  result = result.replace(/\$\{ENV:([^}]+)\}/g, (_, name: string) => {
    return process.env[name] || '';
  });

  return result;
}
