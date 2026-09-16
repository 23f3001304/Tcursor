import { CORNER, LUMA, TEMP_GAIN, TINT_GAIN, VIGN_IN } from "./gradeParams";

export const VERTEX_SOURCE = `#version 300 es
void main() {
  vec2 p = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

export function gradeFragmentSource(): string {
  return `#version 300 es
precision highp float;
uniform sampler2D tex;
uniform vec2 dims;
uniform vec4 gA;
uniform vec4 gB;
uniform vec3 lift;
uniform vec3 gam;
uniform vec3 gain;
out vec4 frag;
const vec3 luma = vec3(${LUMA[0]}, ${LUMA[1]}, ${LUMA[2]});
const float vignIn = ${VIGN_IN};
const float corner = ${CORNER};
const float tempGain = ${TEMP_GAIN};
const float tintGain = ${TINT_GAIN};
void main() {
  vec2 px = gl_FragCoord.xy;
  vec2 uvTex = vec2(px.x / dims.x, 1.0 - px.y / dims.y);
  vec4 src = texture(tex, uvTex);
  float e = exp2(gA.x);
  vec3 c = vec3(src.r * e * (1.0 + tempGain * gB.x),
                src.g * e * (1.0 + tintGain * gB.y),
                src.b * e * (1.0 - tempGain * gB.x));
  c = clamp(lift + (gain - lift) * c, 0.0, 1.0);
  c = pow(c, 1.0 / max(gam, vec3(0.001)));
  float contrast = gA.y;
  c = (c - 0.5) * contrast + 0.5;
  float l = dot(c, luma);
  c = vec3(l) + (c - vec3(l)) * gA.z;
  vec2 vignette = px / dims - 0.5;
  float d = length(vignette) / corner;
  float k = clamp((d - vignIn) / (1.0 - vignIn), 0.0, 1.0);
  frag = vec4(clamp(c * (1.0 - gA.w * k * k), 0.0, 1.0), src.a);
}`;
}
