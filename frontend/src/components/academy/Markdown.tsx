"use client";

import katex from "katex";
import "katex/dist/katex.min.css";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";

function Math({ tex, display }: { tex: string; display: boolean }) {
  // KaTeX escapes its input; `trust: false` blocks \href and similar commands.
  const html = katex.renderToString(tex, { displayMode: display, throwOnError: false, trust: false, strict: "ignore" });
  return display ? (
    <div className="my-3 overflow-x-auto" dangerouslySetInnerHTML={{ __html: html }} />
  ) : (
    <span dangerouslySetInnerHTML={{ __html: html }} />
  );
}

/** Lesson Markdown: GitHub-flavoured Markdown plus $inline$ and $$display$$ math (KaTeX). */
export function Markdown({ text }: { text: string }) {
  return (
    <div className="academy-prose text-sm leading-relaxed text-fg">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMath]}
        components={{
          code({ className, children }) {
            const value = String(children).replace(/\n$/, "");
            if (className?.includes("math-display")) return <Math tex={value} display />;
            if (className?.includes("math-inline")) return <Math tex={value} display={false} />;
            return <code className="rounded bg-surface-3 px-1 font-mono text-[0.9em]">{value}</code>;
          },
          pre({ children }) {
            return <pre className="my-2 overflow-x-auto rounded bg-surface-3 p-2 text-xs">{children}</pre>;
          },
          a({ href, children }) {
            const external = href?.startsWith("http");
            return (
              <a
                href={href}
                className="text-accent underline-offset-2 hover:underline"
                {...(external ? { target: "_blank", rel: "noreferrer" } : {})}
              >
                {children}
              </a>
            );
          },
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
