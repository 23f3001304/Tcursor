/** Turns a raw AI-director failure string (`String(e)` off the `ai_plan` IPC rejection) into a
 *  short title + an actionable hint, for the two failure modes an Ollama setup actually hits day
 *  to day - matches the exact wording `build_plan` (`src-tauri/src/ai/commands.rs`) and
 *  `ollama::chat` (`src-tauri/src/ai/backend/ollama.rs`) produce. Anything else falls through
 *  unchanged (`hint: null`) - same as before this task, just wrapped in one shape so `AiPanel`
 *  only needs one render path. */
export function friendlyAiError(raw: string): { title: string; hint: string | null } {
  if (raw.includes("No Ollama models")) {
    return { title: "No local model installed", hint: "Run: ollama pull llama3.2" };
  }
  if (/could not reach ollama|is it running|refused|timed? ?out/i.test(raw)) {
    return { title: "Ollama isn't running", hint: "Start the Ollama app, then try again" };
  }
  return { title: raw, hint: null };
}
