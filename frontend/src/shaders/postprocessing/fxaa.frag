/**
 * Fast Approximate Anti-Aliasing (FXAA) Fragment Shader
 * Edge-aware antialiasing for smooth edges without full MSAA
 */

precision highp float;

uniform sampler2D tColor;
uniform vec2 u_resolution;
uniform float u_fxaaQualitySubpix;   // 0.75
uniform float u_fxaaQualityEdgeThreshold;  // 0.166
uniform float u_fxaaQualityEdgeThresholdMin; // 0.0625
uniform bool u_fxaaEnabled;

varying vec2 vUv;

#include ../utils/sampling.glsl

float fxaaLuma(vec3 rgb) {
  return rgb.y * (0.587/0.299) + rgb.x;
}

vec3 fxaaAdvanced(sampler2D tex, vec2 pos, vec2 fxaaRcpFrame) {
  vec2 posM = pos;
  vec4 rgbyM = texture2D(tex, posM);
  vec3 rgbM = rgbyM.rgb;
  float lumaM = rgbyM.a;
  
  float lumaS = fxaaLuma(texture2D(tex, posM + vec2( 0.0,  1.0) * fxaaRcpFrame).rgb);
  float lumaE = fxaaLuma(texture2D(tex, posM + vec2( 1.0,  0.0) * fxaaRcpFrame).rgb);
  float lumaN = fxaaLuma(texture2D(tex, posM + vec2( 0.0, -1.0) * fxaaRcpFrame).rgb);
  float lumaW = fxaaLuma(texture2D(tex, posM + vec2(-1.0,  0.0) * fxaaRcpFrame).rgb);
  
  float maxSM = max(lumaS, lumaM);
  float minSM = min(lumaS, lumaM);
  float maxESM = max(lumaE, maxSM);
  float minESM = min(lumaE, minSM);
  float maxWN = max(lumaN, lumaW);
  float minWN = min(lumaN, lumaW);
  float rangeMax = max(maxWN, maxESM);
  float rangeMin = min(minWN, minESM);
  float rangeMaxScaled = rangeMax * u_fxaaQualityEdgeThreshold;
  float range = rangeMax - rangeMin;
  float rangeMaxClamped = max(u_fxaaQualityEdgeThresholdMin, rangeMaxScaled);
  
  bool earlyExit = range < rangeMaxClamped;
  if(earlyExit) return rgbM;
  
  float lumaNW = fxaaLuma(texture2D(tex, posM + vec2(-1.0, -1.0) * fxaaRcpFrame).rgb);
  float lumaSE = fxaaLuma(texture2D(tex, posM + vec2( 1.0,  1.0) * fxaaRcpFrame).rgb);
  float lumaNE = fxaaLuma(texture2D(tex, posM + vec2( 1.0, -1.0) * fxaaRcpFrame).rgb);
  float lumaSW = fxaaLuma(texture2D(tex, posM + vec2(-1.0,  1.0) * fxaaRcpFrame).rgb);
  
  float lumaNS = lumaN + lumaS;
  float lumaWE = lumaW + lumaE;
  float subpixRcpRange = 1.0/range;
  float subpixNSWE = lumaNS + lumaWE;
  float edgeHorz1 = (-2.0 * lumaM) + lumaNS;
  float edgeVert1 = (-2.0 * lumaM) + lumaWE;
  
  float lumaNESE = lumaNE + lumaSE;
  float lumaNWNE = lumaNW + lumaNE;
  float edgeHorz2 = (-2.0 * lumaE) + lumaNESE;
  float edgeVert2 = (-2.0 * lumaN) + lumaNWNE;
  
  float lumaNWSW = lumaNW + lumaSW;
  float lumaSWSE = lumaSW + lumaSE;
  float edgeHorz4 = (abs(edgeHorz1) * 2.0) + abs(edgeHorz2);
  float edgeVert4 = (abs(edgeVert1) * 2.0) + abs(edgeVert2);
  float edgeHorz3 = (-2.0 * lumaW) + lumaNWSW;
  float edgeVert3 = (-2.0 * lumaS) + lumaSWSE;
  float edgeHorz = abs(edgeHorz3) + edgeHorz4;
  float edgeVert = abs(edgeVert3) + edgeVert4;
  
  float subpixNWSWNESE = lumaNWSW + lumaNESE;
  float lengthSign = fxaaRcpFrame.x;
  bool horzSpan = edgeHorz >= edgeVert;
  float subpixA = subpixNSWE * 2.0 + subpixNWSWNESE;
  
  if(!horzSpan) lumaN = lumaW;
  if(!horzSpan) lumaS = lumaE;
  if(horzSpan) lengthSign = fxaaRcpFrame.y;
  float subpixB = (subpixA * (1.0/12.0)) - lumaM;
  
  float gradientN = lumaN - lumaM;
  float gradientS = lumaS - lumaM;
  float lumaNN = lumaN + lumaM;
  float lumaSS = lumaS + lumaM;
  bool pairN = abs(gradientN) >= abs(gradientS);
  float gradient = max(abs(gradientN), abs(gradientS));
  if(pairN) lengthSign = -lengthSign;
  float subpixC = clamp(abs(subpixB) * subpixRcpRange, 0.0, 1.0);
  
  vec2 posB = posM;
  vec2 offNP = vec2(0.0, 0.0);
  if(!horzSpan) offNP.x = fxaaRcpFrame.x;
  if( horzSpan) offNP.y = fxaaRcpFrame.y;
  if(!horzSpan) posB.x += lengthSign * 0.5;
  if( horzSpan) posB.y += lengthSign * 0.5;
  
  vec2 posN = posB - offNP;
  vec2 posP = posB + offNP;
  float subpixD = ((-2.0)*subpixC) + 3.0;
  float lumaEndN = fxaaLuma(texture2D(tex, posN).rgb);
  float subpixE = subpixC * subpixC;
  float lumaEndP = fxaaLuma(texture2D(tex, posP).rgb);
  
  if(!pairN) lumaNN = lumaSS;
  float gradientScaled = gradient * 1.0/4.0;
  float lumaMM = lumaM - lumaNN * 0.5;
  float subpixF = subpixD * subpixE;
  bool lumaMLTZero = lumaMM < 0.0;
  
  lumaEndN -= lumaNN * 0.5;
  lumaEndP -= lumaNN * 0.5;
  bool doneN = abs(lumaEndN) >= gradientScaled;
  bool doneP = abs(lumaEndP) >= gradientScaled;
  if(!doneN) posN.x -= offNP.x * 1.5;
  if(!doneN) posN.y -= offNP.y * 1.5;
  bool doneNP = (!doneN) || (!doneP);
  if(!doneP) posP.x += offNP.x * 1.5;
  if(!doneP) posP.y += offNP.y * 1.5;
  
  if(doneNP) {
    if(!doneN) lumaEndN = fxaaLuma(texture2D(tex, posN.xy).rgb);
    if(!doneP) lumaEndP = fxaaLuma(texture2D(tex, posP.xy).rgb);
    if(!doneN) lumaEndN = lumaEndN - lumaNN * 0.5;
    if(!doneP) lumaEndP = lumaEndP - lumaNN * 0.5;
    doneN = abs(lumaEndN) >= gradientScaled;
    doneP = abs(lumaEndP) >= gradientScaled;
    if(!doneN) posN.x -= offNP.x * 2.0;
    if(!doneN) posN.y -= offNP.y * 2.0;
    doneNP = (!doneN) || (!doneP);
    if(!doneP) posP.x += offNP.x * 2.0;
    if(!doneP) posP.y += offNP.y * 2.0;
  }
  
  float dstN = posM.x - posN.x;
  float dstP = posP.x - posM.x;
  if(!horzSpan) dstN = posM.y - posN.y;
  if(!horzSpan) dstP = posP.y - posM.y;
  
  bool goodSpanN = (lumaEndN < 0.0) != lumaMLTZero;
  float spanLength = (dstP + dstN);
  bool goodSpanP = (lumaEndP < 0.0) != lumaMLTZero;
  float spanLengthRcp = 1.0/spanLength;
  
  bool directionN = dstN < dstP;
  float dst = min(dstN, dstP);
  bool goodSpan = directionN ? goodSpanN : goodSpanP;
  float subpixG = subpixF * subpixF;
  float pixelOffset = (dst * (-spanLengthRcp)) + 0.5;
  float subpixH = subpixG * u_fxaaQualitySubpix;
  
  float pixelOffsetGood = goodSpan ? pixelOffset : 0.0;
  float pixelOffsetSubpix = max(pixelOffsetGood, subpixH);
  if(!horzSpan) posM.x += pixelOffsetSubpix * lengthSign;
  if( horzSpan) posM.y += pixelOffsetSubpix * lengthSign;
  
  return texture2D(tex, posM).rgb;
}

void main() {
  if (!u_fxaaEnabled) {
    gl_FragColor = vec4(texture2D(tColor, vUv).rgb, 1.0);
    return;
  }
  
  vec2 texelSize = 1.0 / u_resolution;
  vec3 antialiasedColor = fxaaAdvanced(tColor, vUv, texelSize);
  
  gl_FragColor = vec4(antialiasedColor, 1.0);
}
