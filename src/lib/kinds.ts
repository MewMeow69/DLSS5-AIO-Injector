const COMPONENT_KIND: Record<string, string> = {
  "renodx-rhi": "renodx",
  "optiscaler-imported": "optiscaler",
};

/** Some components own more than one artifact pool (DLL + its installer). */
const EXTRA_KINDS: Record<string, string[]> = {
  "dlss-enabler": ["dlss-enabler-installer"],
};

export function artifactKind(componentId: string) {
  return COMPONENT_KIND[componentId] ?? componentId;
}

export function artifactKinds(componentId: string): string[] {
  const kind = artifactKind(componentId);
  return [kind, ...(EXTRA_KINDS[kind] ?? [])];
}

export function artifactsFor(componentId: string, byKind: Record<string, import("./types").Artifact[]>) {
  return artifactKinds(componentId).flatMap((k) => byKind[k] ?? []);
}

export const FORK_LABEL: Record<string, string> = {
  "optiscaler-wilsjo2": "wilsjo2 fork",
  "optiscaler-janblade": "janblade fork",
  "optiscaler-nightly": "upstream nightly",
  optiscaler: "imported",
};
