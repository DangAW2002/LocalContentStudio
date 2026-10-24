// @vitest-environment jsdom
import React from "react";
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import {
  AskUserQuestionCard,
  parseOptionLabel,
  parseQuestionInput,
} from "../../../src/views/project/thread/AskUserQuestionCard";

describe("AskUserQuestionCard parsing utilities", () => {
  it("detects and strips (Recommended) and (Khuyến nghị) tags", () => {
    expect(parseOptionLabel("date-fns (Recommended)")).toEqual({
      displayLabel: "date-fns",
      isRecommended: true,
    });
    expect(parseOptionLabel("Day.js (Khuyến nghị)")).toEqual({
      displayLabel: "Day.js",
      isRecommended: true,
    });
    expect(parseOptionLabel("Moment.js")).toEqual({
      displayLabel: "Moment.js",
      isRecommended: false,
    });
  });

  it("parses raw string or object questions safely", () => {
    const raw = JSON.stringify({
      title: "Chọn cấu hình",
      questions: [
        {
          question: "Bạn chọn framework nào?",
          options: [{ label: "React (Recommended)" }, { label: "Vue" }],
        },
      ],
    });
    const parsed = parseQuestionInput(raw);
    expect(parsed.title).toBe("Chọn cấu hình");
    expect(parsed.questions).toHaveLength(1);
    expect(parsed.questions[0].question).toBe("Bạn chọn framework nào?");
    expect(parsed.questions[0].options?.[0].label).toBe("React (Recommended)");
  });
});

describe("AskUserQuestionCard UI component", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders single-select question with recommended option and submits choice", () => {
    const onRespond = vi.fn();
    const input = {
      title: "Cấu hình dự án",
      questions: [
        {
          question: "Chọn công cụ định dạng ngày?",
          options: [
            { label: "date-fns (Recommended)", description: "Nhẹ và modular" },
            { label: "Day.js", description: "API tương tự Moment" },
          ],
        },
      ],
    };

    render(
      <AskUserQuestionCard
        requestId="req-123"
        tool="AskUserQuestion"
        input={input}
        onRespond={onRespond}
      />
    );

    // Title and question
    expect(screen.getByText("Cấu hình dự án")).toBeDefined();
    expect(screen.getByText("Chọn công cụ định dạng ngày?")).toBeDefined();
    expect(screen.getByText("Khuyến nghị")).toBeDefined();
    expect(screen.getByText("date-fns")).toBeDefined();
    expect(screen.getByText("Nhẹ và modular")).toBeDefined();

    // Submit button is disabled before selection
    const submitBtn = screen.getByRole("button", { name: "Gửi câu trả lời" });
    expect(submitBtn.hasAttribute("disabled")).toBe(true);

    // Click option A
    fireEvent.click(screen.getByText("date-fns"));
    expect(submitBtn.hasAttribute("disabled")).toBe(false);

    // Click submit
    fireEvent.click(submitBtn);
    expect(onRespond).toHaveBeenCalledWith({
      "Chọn công cụ định dạng ngày?": "date-fns (Recommended)",
    });
  });

  it("supports multiSelect with multiple choices", () => {
    const onRespond = vi.fn();
    const input = {
      questions: [
        {
          question: "Chọn các tính năng cần bật?",
          multiSelect: true,
          options: [
            { label: "ESLint" },
            { label: "Prettier" },
            { label: "TypeScript" },
          ],
        },
      ],
    };

    render(
      <AskUserQuestionCard
        requestId="req-multi"
        tool="AskUserQuestion"
        input={input}
        onRespond={onRespond}
      />
    );

    expect(screen.getByText("(Có thể chọn nhiều phương án)")).toBeDefined();

    // Select ESLint and TypeScript
    fireEvent.click(screen.getByText("ESLint"));
    fireEvent.click(screen.getByText("TypeScript"));

    const submitBtn = screen.getByRole("button", { name: "Gửi câu trả lời" });
    fireEvent.click(submitBtn);

    expect(onRespond).toHaveBeenCalledWith({
      "Chọn các tính năng cần bật?": ["ESLint", "TypeScript"],
    });
  });

  it("supports 'Khác' with free-text custom answer", () => {
    const onRespond = vi.fn();
    const input = {
      questions: [
        {
          question: "Phương án triển khai?",
          options: [{ label: "Vercel" }, { label: "Netlify" }],
        },
      ],
    };

    render(
      <AskUserQuestionCard
        requestId="req-other"
        tool="AskUserQuestion"
        input={input}
        onRespond={onRespond}
      />
    );

    // Click "Khác"
    fireEvent.click(screen.getByText("Khác (nhập phương án của bạn)"));

    // Find input and type custom text
    const textInput = screen.getByPlaceholderText("Nhập nội dung lựa chọn tự do...");
    fireEvent.change(textInput, { target: { value: "Tự host trên Docker" } });

    const submitBtn = screen.getByRole("button", { name: "Gửi câu trả lời" });
    expect(submitBtn.hasAttribute("disabled")).toBe(false);

    fireEvent.click(submitBtn);
    expect(onRespond).toHaveBeenCalledWith({
      "Phương án triển khai?": "Tự host trên Docker",
    });
  });

  it("supports keyboard interaction using Enter or Space", () => {
    const onRespond = vi.fn();
    const input = {
      questions: [
        {
          question: "Bạn chọn phương án nào?",
          options: [{ label: "Phương án 1" }, { label: "Phương án 2" }],
        },
      ],
    };

    render(
      <AskUserQuestionCard
        requestId="req-kbd"
        tool="AskUserQuestion"
        input={input}
        onRespond={onRespond}
      />
    );

    const opt1 = screen.getByRole("radio", { name: /Phương án 1/i });
    fireEvent.keyDown(opt1, { key: "Enter" });

    const submitBtn = screen.getByRole("button", { name: "Gửi câu trả lời" });
    expect(submitBtn.hasAttribute("disabled")).toBe(false);

    fireEvent.click(submitBtn);
    expect(onRespond).toHaveBeenCalledWith({
      "Bạn chọn phương án nào?": "Phương án 1",
    });
  });

  it("renders read-only answered state when resolved is true", () => {
    const input = {
      questions: [
        {
          question: "Thư viện đã chọn?",
          options: [{ label: "date-fns" }, { label: "Day.js" }],
        },
      ],
    };

    render(
      <AskUserQuestionCard
        requestId="req-resolved"
        tool="AskUserQuestion"
        input={input}
        resolved={true}
        answers={{ "Thư viện đã chọn?": "date-fns" }}
      />
    );

    // Displays resolved badge
    expect(screen.getByText("Đã trả lời")).toBeDefined();
    // Displays user's chosen answer
    expect(screen.getByText("date-fns")).toBeDefined();
    // Submit button should NOT be present
    expect(screen.queryByRole("button", { name: "Gửi câu trả lời" })).toBeNull();
  });
});
