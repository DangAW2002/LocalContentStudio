import { describe, expect, it } from "vitest";
import {
  aggregateFileChanges,
  type FileChangeItem,
} from "../../../src/views/project/thread/ResultCard";

describe("aggregateFileChanges", () => {
  it("merges multiple operations on the same path where last op wins", () => {
    const changes: FileChangeItem[] = [
      { path: "src/script.md", op: "create" },
      { path: "src/script.md", op: "modify" },
      { path: "assets/logo.png", op: "create" },
      { path: "temp.txt", op: "create" },
      { path: "temp.txt", op: "delete" },
    ];

    const result = aggregateFileChanges(changes);
    const script = result.find((r) => r.path === "src/script.md");
    expect(script?.op).toBe("modify");

    const temp = result.find((r) => r.path === "temp.txt");
    expect(temp?.op).toBe("delete");
  });

  it("sorts image files before other files", () => {
    const changes: FileChangeItem[] = [
      { path: "src/main.rs", op: "modify" },
      { path: "docs/readme.md", op: "create" },
      { path: "agy-artifacts/banner.webp", op: "create" },
      { path: "video.mp4", op: "create" },
      { path: "agy-artifacts/thumbnail.png", op: "create" },
    ];

    const result = aggregateFileChanges(changes);
    expect(result.map((r) => r.fileName)).toEqual([
      "banner.webp",
      "thumbnail.png",
      "main.rs",
      "readme.md",
      "video.mp4",
    ]);
    expect(result[0].category).toBe("image");
    expect(result[1].category).toBe("image");
  });

  it("extracts fileName and dirPath properly for Unix and Windows paths", () => {
    const changes: FileChangeItem[] = [
      { path: "root.txt", op: "create" },
      { path: "src/components/Header.tsx", op: "modify" },
      { path: "agy-artifacts\\subfolder\\image.png", op: "create" },
    ];

    const result = aggregateFileChanges(changes);
    const root = result.find((r) => r.fileName === "root.txt");
    expect(root?.dirPath).toBe("");

    const header = result.find((r) => r.fileName === "Header.tsx");
    expect(header?.dirPath).toBe("src/components");

    const img = result.find((r) => r.fileName === "image.png");
    expect(img?.dirPath).toBe("agy-artifacts/subfolder");
  });

  it("classifies file extensions correctly", () => {
    const changes: FileChangeItem[] = [
      { path: "photo.jpg", op: "create" },
      { path: "clip.mp4", op: "create" },
      { path: "voice.mp3", op: "create" },
      { path: "code.ts", op: "modify" },
      { path: "archive.zip", op: "create" },
    ];

    const result = aggregateFileChanges(changes);
    expect(result.find((r) => r.fileName === "photo.jpg")?.category).toBe("image");
    expect(result.find((r) => r.fileName === "clip.mp4")?.category).toBe("video");
    expect(result.find((r) => r.fileName === "voice.mp3")?.category).toBe("audio");
    expect(result.find((r) => r.fileName === "code.ts")?.category).toBe("text");
    expect(result.find((r) => r.fileName === "archive.zip")?.category).toBe("other");
  });

  it("handles empty or blank paths gracefully", () => {
    const changes: FileChangeItem[] = [
      { path: "", op: "create" },
      { path: "valid.md", op: "create" },
    ];
    const result = aggregateFileChanges(changes);
    expect(result).toHaveLength(1);
    expect(result[0].fileName).toBe("valid.md");
  });
});
