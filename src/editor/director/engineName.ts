/** Ollama model ids can be very long - especially HF GGUF proxies like
 *  "hf.co/empero-ai/Qwythos-9B-Claude-Mythos-5-1M-GGUF:Q8_0" (ux audit #16/#17: this wraps over
 *  two lines in the Engine dropdown, and prints in full on the running-status pill). This derives
 *  a short "Family Size (Quant)" label - e.g. "Qwythos 9B (Q8)" - for display; callers keep the
 *  raw id as the real value and put it in a `title` attribute so it's still there on hover, not
 *  lost. Pure/best-effort: any id shape it doesn't recognize just falls through with the host/org
 *  prefix stripped, never throws, never returns empty for a non-empty input. */
export function engineDisplayName(id: string): string {
  const trimmed = id.trim();
  if (!trimmed) return trimmed;

  // Drop a host/org prefix ("hf.co/empero-ai/…") - the last path segment is the real model name.
  const lastSegment = trimmed.split("/").pop() || trimmed;
  // The Ollama "name:tag" convention - the tag often carries size/quant info the name doesn't
  // (plain Ollama pulls like "llama3.1:8b-instruct-q4_0"), so both sides get scanned below.
  const [base, tag] = lastSegment.split(":");
  const name = base.replace(/-GGUF$/i, "");

  const nameTokens = name.split(/[-_]/).filter(Boolean);
  const tagTokens = tag ? tag.split(/[-_]/).filter(Boolean) : [];
  const family = nameTokens[0] || name;

  const isSize = (t: string) => /^\d+(\.\d+)?[bmk]$/i.test(t);
  const isQuant = (t: string) => /^q\d/i.test(t);
  // Scan the model-name tokens (after the family) first, then the tag tokens - either side can
  // carry the size/quant info depending on how the id was built.
  const rest = [...nameTokens.slice(1), ...tagTokens];
  const sizeToken = rest.find(isSize);
  const quantToken = rest.find(isQuant);

  // Only collapse down to "Family Size" once a real size token turns up - otherwise the rest of
  // the name might be all that distinguishes two installed variants (e.g. "Phi-3-Mini" vs
  // "Phi-3-Medium"), so keep it whole rather than guessing which word is disposable.
  let short = sizeToken ? `${family} ${sizeToken.toUpperCase()}` : name;
  if (quantToken) short += ` (${quantToken.toUpperCase()})`;
  return short;
}
