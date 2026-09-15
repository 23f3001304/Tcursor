export function engineDisplayName(id: string): string {
  const trimmed = id.trim();
  if (!trimmed) return trimmed;

  const lastSegment = trimmed.split("/").pop() || trimmed;
  const [base, tag] = lastSegment.split(":");
  const name = base.replace(/-GGUF$/i, "");

  const nameTokens = name.split(/[-_]/).filter(Boolean);
  const tagTokens = tag ? tag.split(/[-_]/).filter(Boolean) : [];
  const family = nameTokens[0] || name;

  const isSize = (t: string) => /^\d+(\.\d+)?[bmk]$/i.test(t);
  const isQuant = (t: string) => /^q\d/i.test(t);
  const rest = [...nameTokens.slice(1), ...tagTokens];
  const sizeToken = rest.find(isSize);
  const quantToken = rest.find(isQuant);

  let short = sizeToken ? `${family} ${sizeToken.toUpperCase()}` : name;
  if (quantToken) short += ` (${quantToken.toUpperCase()})`;
  return short;
}
