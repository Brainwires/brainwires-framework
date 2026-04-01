"use client";

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import { oneLight } from "react-syntax-highlighter/dist/esm/styles/prism";
import { useTheme } from "next-themes";
import { ExternalLink, Copy, Check } from "lucide-react";
import { useState } from "react";
import { cn } from "@/lib/utils";
import type { Components } from "react-markdown";

function slugify(text: string): string {
  return text.toLowerCase().replace(/[^\w\s-]/g, "").trim().replace(/\s+/g, "-");
}

function CopyButton({ code }: { code: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      onClick={() => { navigator.clipboard.writeText(code); setCopied(true); setTimeout(() => setCopied(false), 2000); }}
      className="absolute right-2 top-2 rounded p-1 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-foreground"
      aria-label="Copy code"
    >
      {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
    </button>
  );
}

function CodeBlock({ language, code, isDark }: { language: string; code: string; isDark: boolean }) {
  return (
    <div className="group relative my-4 rounded-lg overflow-hidden border text-sm">
      {language && (
        <div className="flex items-center border-b bg-muted/50 px-4 py-1.5 text-xs text-muted-foreground">
          <span>{language}</span>
        </div>
      )}
      <CopyButton code={code} />
      <SyntaxHighlighter
        language={language || "text"}
        style={isDark ? oneDark : oneLight}
        customStyle={{ margin: 0, borderRadius: 0, background: "transparent", fontSize: "0.85em" }}
        PreTag="div"
      >
        {code}
      </SyntaxHighlighter>
    </div>
  );
}

export function Markdown({ content }: { content: string }) {
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === "dark";

  const components: Components = {
    h1: ({ children }) => { const id = slugify(String(children)); return <h1 id={id} className="scroll-mt-16 text-3xl font-bold mt-8 mb-4 first:mt-0">{children}</h1>; },
    h2: ({ children }) => { const id = slugify(String(children)); return <h2 id={id} className="scroll-mt-16 text-2xl font-semibold mt-10 mb-4 border-b pb-2">{children}</h2>; },
    h3: ({ children }) => { const id = slugify(String(children)); return <h3 id={id} className="scroll-mt-16 text-xl font-semibold mt-8 mb-3">{children}</h3>; },
    h4: ({ children }) => <h4 className="text-lg font-semibold mt-6 mb-2">{children}</h4>,
    p: ({ children }) => <p className="leading-7 mb-4">{children}</p>,
    a: ({ href, children }) => {
      const isExternal = href?.startsWith("http");
      return (
        <a href={href} target={isExternal ? "_blank" : undefined} rel={isExternal ? "noopener noreferrer" : undefined}
          className="text-primary underline underline-offset-4 hover:text-primary/80 inline-flex items-center gap-0.5">
          {children}{isExternal && <ExternalLink className="size-3 shrink-0" />}
        </a>
      );
    },
    ul: ({ children }) => <ul className="my-4 ml-6 list-disc space-y-1 [&>li]:mt-1">{children}</ul>,
    ol: ({ children }) => <ol className="my-4 ml-6 list-decimal space-y-1 [&>li]:mt-1">{children}</ol>,
    li: ({ children }) => <li className="leading-7">{children}</li>,
    blockquote: ({ children }) => <blockquote className="border-l-4 border-primary/30 pl-4 my-4 italic text-muted-foreground">{children}</blockquote>,
    table: ({ children }) => <div className="my-4 overflow-x-auto rounded-lg border"><table className="w-full text-sm">{children}</table></div>,
    thead: ({ children }) => <thead className="bg-muted/50">{children}</thead>,
    th: ({ children }) => <th className="border-b px-4 py-2 text-left font-semibold">{children}</th>,
    td: ({ children }) => <td className="border-b px-4 py-2 last:border-0">{children}</td>,
    tr: ({ children }) => <tr className="hover:bg-muted/30 transition-colors">{children}</tr>,
    hr: () => <hr className="my-8 border-border" />,
    code: ({ className, children }) => {
      const match = /language-(\w+)/.exec(className ?? "");
      const isInline = !match && !String(children).includes("\n");
      if (isInline) return <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[0.85em]">{children}</code>;
      return <CodeBlock language={match?.[1] ?? ""} code={String(children).replace(/\n$/, "")} isDark={isDark} />;
    },
    pre: ({ children }) => <>{children}</>,
    img: ({ src, alt }) => (
      // eslint-disable-next-line @next/next/no-img-element
      <img src={src} alt={alt ?? ""} className="my-4 max-w-full rounded-lg border" />
    ),
  };

  return (
    <div className={cn("prose-neutral max-w-none", "prose prose-sm sm:prose-base", "dark:prose-invert")}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>{content}</ReactMarkdown>
    </div>
  );
}
