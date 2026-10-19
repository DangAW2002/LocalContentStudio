/**
 * Command registry and MRU history for Command Palette.
 * Supports context-sensitive `when` conditions, keybinding mappings, and MRU persistence.
 */

export interface CommandContext {
  activeArea?: string;
  hasSelectedArtifact?: boolean;
  hasRunningTurn?: boolean;
  hasSelectedFile?: boolean;
  currentSlug?: string;
  activeThreadId?: string;
  [key: string]: any;
}

export interface CommandDefinition {
  id: string;
  title: string;
  category: string;
  icon?: string;
  keybinding?: string;
  description?: string;
  when?: (ctx: CommandContext) => boolean;
  run: (ctx: CommandContext) => void | Promise<void>;
}

const MRU_STORAGE_KEY = "lcs.commands.mru";
const MAX_MRU_COUNT = 10;

export class CommandRegistry {
  private commands = new Map<string, CommandDefinition>();

  register(command: CommandDefinition): () => void {
    this.commands.set(command.id, command);
    return () => {
      this.commands.delete(command.id);
    };
  }

  registerMany(cmds: CommandDefinition[]): () => void {
    for (const cmd of cmds) {
      this.commands.set(cmd.id, cmd);
    }
    return () => {
      for (const cmd of cmds) {
        this.commands.delete(cmd.id);
      }
    };
  }

  get(id: string): CommandDefinition | undefined {
    return this.commands.get(id);
  }

  getAll(): CommandDefinition[] {
    return Array.from(this.commands.values());
  }

  getVisible(ctx: CommandContext): CommandDefinition[] {
    return this.getAll().filter((cmd) => {
      if (!cmd.when) return true;
      try {
        return cmd.when(ctx);
      } catch {
        return false;
      }
    });
  }

  // ==========================================
  // MRU (Most Recently Used) Handling
  // ==========================================

  getMruIds(): string[] {
    if (typeof window === "undefined" || !window.localStorage) return [];
    try {
      const data = localStorage.getItem(MRU_STORAGE_KEY);
      if (!data) return [];
      const parsed = JSON.parse(data);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }

  recordUsage(commandId: string): void {
    if (typeof window === "undefined" || !window.localStorage) return;
    try {
      const current = this.getMruIds().filter((id) => id !== commandId);
      current.unshift(commandId);
      const trimmed = current.slice(0, MAX_MRU_COUNT);
      localStorage.setItem(MRU_STORAGE_KEY, JSON.stringify(trimmed));
    } catch {
      // ignore
    }
  }

  getMruCommands(ctx: CommandContext): CommandDefinition[] {
    const ids = this.getMruIds();
    const visible = new Map(this.getVisible(ctx).map((c) => [c.id, c]));
    const result: CommandDefinition[] = [];
    for (const id of ids) {
      const cmd = visible.get(id);
      if (cmd) {
        result.push(cmd);
      }
    }
    return result;
  }
}

export const globalCommandRegistry = new CommandRegistry();
