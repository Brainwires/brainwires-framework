import { type ReactNode } from "react";

export type BubbleKind = "user" | "assistant" | "tool" | "error";

export interface MessageBubbleProps {
  kind: BubbleKind;
  children: ReactNode;
  meta?: string;
}

export function MessageBubble({ kind, children, meta }: MessageBubbleProps) {
  const palette: Record<BubbleKind, string> = {
    user: "self-end bg-bw-user text-white",
    assistant: "self-start bg-bw-assistant text-neutral-100",
    tool: "self-start bg-neutral-800 text-amber-200 italic text-sm",
    error: "self-start bg-red-900/40 border border-red-800 text-red-100 text-sm",
  };

  return (
    <div className={`max-w-[80%] rounded-xl px-4 py-2 ${palette[kind]}`}>
      {meta ? (
        <div className="mb-1 text-[11px] uppercase tracking-wide opacity-60">
          {meta}
        </div>
      ) : null}
      <div className="whitespace-pre-wrap break-words text-sm leading-relaxed">
        {children}
      </div>
    </div>
  );
}
