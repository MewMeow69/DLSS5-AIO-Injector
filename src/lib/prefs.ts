const BETA_KEY = "dlss5aio.beta";
const LEGACY_BETA_KEY = "neurodeck.beta";

export function getAllowBeta(): boolean {
  const v = localStorage.getItem(BETA_KEY) ?? localStorage.getItem(LEGACY_BETA_KEY);
  return v === "1";
}

export function setAllowBeta(on: boolean) {
  localStorage.setItem(BETA_KEY, on ? "1" : "0");
}
