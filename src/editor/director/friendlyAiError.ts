export function friendlyAiError(raw: string): { title: string; hint: string | null } {
  if (raw.includes("No Ollama models")) {
    return { title: "No local model installed", hint: "Run: ollama pull llama3.2" };
  }
  if (/could not reach ollama|is it running|refused|timed? ?out/i.test(raw)) {
    return { title: "Ollama isn't running", hint: "Start the Ollama app, then try again" };
  }
  return { title: raw, hint: null };
}
