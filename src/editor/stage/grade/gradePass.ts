import type { GradeSettings } from "../../../hud/settings/settings";
import { paramsOf } from "./gradeParams";
import { gradeFragmentSource, VERTEX_SOURCE } from "./gradeShader";

interface Ctx {
  canvas: HTMLCanvasElement;
  gl: WebGL2RenderingContext;
  tex: WebGLTexture;
  u: Record<string, WebGLUniformLocation | null>;
}

let held: Ctx | null = null;
let failed = false;

function compile(gl: WebGL2RenderingContext, kind: number, src: string): WebGLShader | null {
  const s = gl.createShader(kind);
  if (!s) return null;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
    if (import.meta.env.DEV) console.warn("grade shader", gl.getShaderInfoLog(s));
    return null;
  }
  return s;
}

function giveUp(why: string): false {
  held = null;
  failed = true;
  if (import.meta.env.DEV) {
    console.info(`grade: ${why}, the preview is ungraded; the export still grades`);
  }
  return false;
}

function build(): Ctx | null {
  const canvas = document.createElement("canvas");
  const gl = canvas.getContext("webgl2", { premultipliedAlpha: false, antialias: false });
  if (!gl) return null;
  canvas.addEventListener("webglcontextlost", (e) => {
    e.preventDefault();
    held = null;
  });
  const vs = compile(gl, gl.VERTEX_SHADER, VERTEX_SOURCE);
  const fs = compile(gl, gl.FRAGMENT_SHADER, gradeFragmentSource());
  const prog = vs && fs ? gl.createProgram() : null;
  if (!vs || !fs || !prog) return null;
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    if (import.meta.env.DEV) console.warn("grade program", gl.getProgramInfoLog(prog));
    return null;
  }
  gl.useProgram(prog);
  const tex = gl.createTexture();
  if (!tex) return null;
  gl.bindTexture(gl.TEXTURE_2D, tex);
  for (const p of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T])
    gl.texParameteri(gl.TEXTURE_2D, p, gl.CLAMP_TO_EDGE);
  for (const p of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER])
    gl.texParameteri(gl.TEXTURE_2D, p, gl.NEAREST);
  const at = (n: string) => gl.getUniformLocation(prog, n);
  const u = {
    tex: at("tex"),
    dims: at("dims"),
    gA: at("gA"),
    gB: at("gB"),
    lift: at("lift"),
    gam: at("gam"),
    gain: at("gain"),
  };
  gl.uniform1i(u.tex, 0);
  return { canvas, gl, tex, u };
}

export function gradeCanvas(ctx: CanvasRenderingContext2D, c: HTMLCanvasElement, s: GradeSettings): boolean {
  const p = paramsOf(s);
  if (!p) return false;
  if (failed) return false;
  if (!held) {
    held = build();
    if (!held) return giveUp("WebGL2 unavailable");
  }
  const { canvas, gl, tex, u } = held;
  if (gl.isContextLost()) return giveUp("the WebGL context was lost");
  if (canvas.width !== c.width || canvas.height !== c.height) {
    canvas.width = c.width;
    canvas.height = c.height;
  }
  gl.viewport(0, 0, c.width, c.height);
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, c);
  gl.uniform2f(u.dims, c.width, c.height);
  gl.uniform4f(u.gA, p.exposure, p.contrast, p.saturation, p.vignette);
  gl.uniform4f(u.gB, p.temp, p.tint, 1, 0);
  gl.uniform3f(u.lift, p.lift[0], p.lift[1], p.lift[2]);
  gl.uniform3f(u.gam, p.gamma[0], p.gamma[1], p.gamma[2]);
  gl.uniform3f(u.gain, p.gain[0], p.gain[1], p.gain[2]);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  ctx.save();
  ctx.globalCompositeOperation = "copy";
  ctx.drawImage(canvas, 0, 0);
  ctx.restore();
  return true;
}
