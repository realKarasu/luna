import type { HelpItem } from "../lib/help";
import { Icon } from "./Icon";

/** Collapsible questions: each one opens on click, Enter or Space. */
export function HelpList({ items }: { items: HelpItem[] }) {
  return (
    <div className="faq">
      {items.map(([q, a]) => (
        <details key={q} className="faq-item">
          <summary>
            <span>{q}</span>
            <Icon name="chevron" size={14} />
          </summary>
          <p className="muted small">{a}</p>
        </details>
      ))}
    </div>
  );
}
