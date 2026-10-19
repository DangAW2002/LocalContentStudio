// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from "vitest";
import { CommandRegistry, type CommandDefinition } from "../../src/runtime/commands";

describe("commands.ts", () => {
  let registry: CommandRegistry;

  beforeEach(() => {
    registry = new CommandRegistry();
    if (typeof window !== "undefined") {
      localStorage.clear();
    }
  });

  it("registers and unregisters commands", () => {
    const unregister = registry.register({
      id: "test.cmd1",
      title: "Test Command 1",
      category: "Test",
      run: () => {},
    });

    expect(registry.get("test.cmd1")).toBeDefined();
    expect(registry.getAll().length).toBe(1);

    unregister();
    expect(registry.get("test.cmd1")).toBeUndefined();
    expect(registry.getAll().length).toBe(0);
  });

  it("filters commands using when condition", () => {
    registry.register({
      id: "cmd.artifact.render",
      title: "Render Artifact",
      category: "Artifact",
      when: (ctx) => Boolean(ctx.hasSelectedArtifact),
      run: () => {},
    });

    registry.register({
      id: "cmd.home",
      title: "Về Home",
      category: "Điều hướng",
      run: () => {},
    });

    const withoutArtifact = registry.getVisible({ hasSelectedArtifact: false });
    expect(withoutArtifact.map((c) => c.id)).toEqual(["cmd.home"]);

    const withArtifact = registry.getVisible({ hasSelectedArtifact: true });
    expect(withArtifact.map((c) => c.id)).toEqual(["cmd.artifact.render", "cmd.home"]);
  });

  it("records and retrieves MRU items in order", () => {
    registry.registerMany([
      { id: "cmd.1", title: "Cmd 1", category: "Test", run: () => {} },
      { id: "cmd.2", title: "Cmd 2", category: "Test", run: () => {} },
      { id: "cmd.3", title: "Cmd 3", category: "Test", run: () => {} },
    ]);

    registry.recordUsage("cmd.1");
    registry.recordUsage("cmd.2");
    registry.recordUsage("cmd.1"); // cmd.1 used again, should move to front

    const mru = registry.getMruCommands({});
    expect(mru.map((c) => c.id)).toEqual(["cmd.1", "cmd.2"]);
  });

  it("filters out invisible commands from MRU list", () => {
    registry.registerMany([
      {
        id: "cmd.conditional",
        title: "Conditional",
        category: "Test",
        when: (ctx) => Boolean(ctx.hasSelectedArtifact),
        run: () => {},
      },
      { id: "cmd.always", title: "Always", category: "Test", run: () => {} },
    ]);

    registry.recordUsage("cmd.conditional");
    registry.recordUsage("cmd.always");

    const mruHidden = registry.getMruCommands({ hasSelectedArtifact: false });
    expect(mruHidden.map((c) => c.id)).toEqual(["cmd.always"]);

    const mruVisible = registry.getMruCommands({ hasSelectedArtifact: true });
    expect(mruVisible.map((c) => c.id)).toEqual(["cmd.always", "cmd.conditional"]);
  });
});
