import type { ReactNode } from "react";

type Status = "ok" | "warn" | "danger" | "neutral";

export function StatusPill({ status, children }: { status: Status; children: ReactNode }) {
  return <span className={`pill pill-${status}`}>{children}</span>;
}
