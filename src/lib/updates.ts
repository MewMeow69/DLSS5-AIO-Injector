import type { Artifact, Component } from "./types";
import { artifactKind } from "./kinds";
import { cmpVersion } from "./version";



export function updatesAvailable(
  components: Component[],
  artifacts: Artifact[],
  allowBeta: boolean,
): { count: number; items: string[] } {
  const items: string[] = [];
  const skip = new Set(["renodx", "lumenite", "streamline", "installer-feeder"]);
  for (const c of components) {
    if (c.channel === "nightly" || c.channel === "local") continue;
    const rel = allowBeta ? c.betaLatest ?? c.latest : c.latest;
    if (!rel) continue;
    const kind = artifactKind(c.id);
    if (skip.has(kind)) continue;
    const localBest = artifacts
      .filter((a) => a.kind === kind)
      .reduce<string | null>((acc, a) => (acc === null || cmpVersion(a.version, acc) > 0 ? a.version : acc), null);
    if (!localBest || cmpVersion(rel.version, localBest) > 0) {
      items.push(`${c.label}: ${localBest ?? "none"} → ${rel.version}`);
    }
  }
  return { count: items.length, items };
}
