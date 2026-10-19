import React from "react";
import type { ProjectFileEntry, ScopeItem } from "../../../api/types";
import type { MentionPickerItem } from "./MentionPicker";

export interface UseMentionPickerOptions {
  projectFiles: ProjectFileEntry[];
  playbooks: ScopeItem[];
  inputPrompt: string;
  setInputPrompt: React.Dispatch<React.SetStateAction<string>>;
  textareaRef: React.RefObject<HTMLTextAreaElement>;
}

export function useMentionPicker({
  projectFiles,
  playbooks,
  inputPrompt,
  setInputPrompt,
  textareaRef,
}: UseMentionPickerOptions) {
  const [pickerMode, setPickerMode] = React.useState<"@" | "/" | null>(null);
  const [pickerIndex, setPickerIndex] = React.useState(0);
  const [pickerStart, setPickerStart] = React.useState(0);
  const pickerListRef = React.useRef<HTMLDivElement>(null);

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    const caret = e.target.selectionStart ?? val.length;
    setInputPrompt(val);

    const typed = val.slice(caret - 1, caret);
    const before = val.slice(caret - 2, caret - 1);
    if ((typed === "@" || typed === "/") && (caret === 1 || /\s/.test(before))) {
      setPickerMode(typed);
      setPickerStart(caret - 1);
      setPickerIndex(0);
      return;
    }
    if (pickerMode) {
      const query = val.slice(pickerStart + 1, caret);
      if (val[pickerStart] !== pickerMode || caret <= pickerStart || /\s/.test(query)) {
        setPickerMode(null);
      } else {
        setPickerIndex(0);
      }
    }
  };

  const pickerQuery = pickerMode ? inputPrompt.slice(pickerStart + 1).split(/\s/)[0].toLowerCase() : "";
  const pickerItems: MentionPickerItem[] = React.useMemo(() => {
    if (!pickerMode) return [];
    const all =
      pickerMode === "@"
        ? projectFiles.map((f) => ({ key: f.path, insert: f.path, label: f.path, meta: "" }))
        : playbooks.map((pb) => ({ key: pb.id, insert: pb.name, label: pb.name, meta: pb.layer }));
    if (!pickerQuery) return all.slice(0, 50);
    const matches = all.filter((it) => it.label.toLowerCase().includes(pickerQuery));
    matches.sort(
      (x, y) =>
        Number(!x.label.toLowerCase().startsWith(pickerQuery)) - Number(!y.label.toLowerCase().startsWith(pickerQuery))
    );
    return matches.slice(0, 50);
  }, [pickerMode, pickerQuery, projectFiles, playbooks]);

  React.useEffect(() => {
    const el = pickerListRef.current?.querySelector<HTMLElement>(`[data-index="${pickerIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [pickerIndex]);

  const handleSelectPickerItem = (itemText: string) => {
    if (!pickerMode) return;
    const queryEnd = pickerStart + 1 + (inputPrompt.slice(pickerStart + 1).match(/^\S*/)?.[0].length ?? 0);
    const head = inputPrompt.slice(0, pickerStart) + pickerMode + itemText + " ";
    const next = head + inputPrompt.slice(queryEnd).replace(/^ /, "");
    setInputPrompt(next);
    setPickerMode(null);
    requestAnimationFrame(() => {
      const ta = textareaRef.current;
      if (!ta) return;
      ta.focus();
      ta.selectionStart = ta.selectionEnd = head.length;
    });
  };

  const handlePickerKey = (e: React.KeyboardEvent<HTMLTextAreaElement>): boolean => {
    if (!pickerMode) return false;
    if (e.key === "Escape") {
      e.preventDefault();
      setPickerMode(null);
      return true;
    }
    if (pickerItems.length === 0) return false;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const delta = e.key === "ArrowDown" ? 1 : -1;
      setPickerIndex((i) => (i + delta + pickerItems.length) % pickerItems.length);
      return true;
    }
    if ((e.key === "Enter" && !e.shiftKey) || e.key === "Tab") {
      e.preventDefault();
      const item = pickerItems[Math.min(pickerIndex, pickerItems.length - 1)];
      if (item) handleSelectPickerItem(item.insert);
      return true;
    }
    return false;
  };

  return {
    pickerMode,
    setPickerMode,
    pickerIndex,
    setPickerIndex,
    pickerItems,
    pickerQuery,
    pickerListRef,
    handleInputChange,
    handlePickerKey,
    handleSelectPickerItem,
  };
}
