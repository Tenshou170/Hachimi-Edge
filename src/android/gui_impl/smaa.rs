use std::sync::Arc;

use glow::HasContext;
use log::{error, info};

use crate::core::hachimi::SmaaQuality;

const AREA_TEX_BYTES: &[u8] = include_bytes!("../../../assets/smaa_area.bin");
const SEARCH_TEX_BYTES: &[u8] = include_bytes!("../../../assets/smaa_search.bin");
const OFFICIAL_SMAA_HLSL: &str = include_str!("../../../assets/SMAA.hlsl");

const FULLSCREEN_VS_BODY: &str = r#"
void FullscreenQuad(out vec4 pos, out vec2 uv) {
    uv = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    pos = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const EDGE_VS_MAIN: &str = r#"
out vec2 v_uv;
out vec4 v_offset[3];
void main() {
    FullscreenQuad(gl_Position, v_uv);
    SMAAEdgeDetectionVS(v_uv, v_offset);
}
"#;

const EDGE_PS_MAIN: &str = r#"
uniform sampler2D colorTex;
in vec2 v_uv;
in vec4 v_offset[3];
out vec4 fragColor;
void main() {
    vec2 edges = SMAAColorEdgeDetectionPS(v_uv, v_offset, colorTex);
    if (dot(edges, vec2(1.0)) == 0.0) discard;
    fragColor = vec4(edges, 0.0, 1.0);
}
"#;

const WEIGHT_VS_MAIN: &str = r#"
out vec2 v_uv;
out vec2 v_pixcoord;
out vec4 v_offset[3];
void main() {
    FullscreenQuad(gl_Position, v_uv);
    SMAABlendingWeightCalculationVS(v_uv, v_pixcoord, v_offset);
}
"#;

const WEIGHT_PS_MAIN: &str = r#"
uniform sampler2D edgesTex;
uniform sampler2D areaTex;
uniform sampler2D searchTex;
in vec2 v_uv;
in vec2 v_pixcoord;
in vec4 v_offset[3];
out vec4 fragColor;
void main() {
    fragColor = SMAABlendingWeightCalculationPS(v_uv, v_pixcoord, v_offset, edgesTex, areaTex, searchTex, vec4(0.0));
}
"#;

const BLEND_VS_MAIN: &str = r#"
out vec2 v_uv;
out vec4 v_offset;
void main() {
    FullscreenQuad(gl_Position, v_uv);
    SMAANeighborhoodBlendingVS(v_uv, v_offset);
}
"#;

const BLEND_PS_MAIN: &str = r#"
uniform sampler2D colorTex;
uniform sampler2D blendTex;
in vec2 v_uv;
in vec4 v_offset;
out vec4 fragColor;
void main() {
    fragColor = SMAANeighborhoodBlendingPS(v_uv, v_offset, colorTex, blendTex);
}
"#;

const FXAA_VS_SRC: &str = r#"#version 300 es
precision highp float;
void main() {
    vec2 uv = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const FXAA_PS_SRC: &str = r#"#version 300 es
precision mediump float;
precision mediump sampler2D;
uniform mediump sampler2D colorTex;
uniform highp vec4 rtMetrics;
out mediump vec4 fragColor;

float FxaaLuma(vec3 rgb) {
    return dot(rgb, vec3(0.299, 0.587, 0.114));
}

void main() {
    highp vec2 posM = gl_FragCoord.xy * rtMetrics.xy;
    highp vec2 rcpFrame = rtMetrics.xy;
    vec4 rgbyM = textureLod(colorTex, posM, 0.0);
    float lumaM = FxaaLuma(rgbyM.rgb);
    float lumaS = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(0, -1)).rgb);
    float lumaE = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(1, 0)).rgb);
    float lumaN = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(0, 1)).rgb);
    float lumaW = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(-1, 0)).rgb);

    float maxSM = max(lumaS, lumaM);
    float minSM = min(lumaS, lumaM);
    float maxESM = max(lumaE, maxSM);
    float minESM = min(lumaE, minSM);
    float maxWN = max(lumaN, lumaW);
    float minWN = min(lumaN, lumaW);
    float rangeMax = max(maxWN, maxESM);
    float rangeMin = min(minWN, minESM);
    float range = rangeMax - rangeMin;
    if (range < max(0.0625, rangeMax * 0.125)) {
        fragColor = rgbyM;
        return;
    }

    float lumaNW = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(-1, 1)).rgb);
    float lumaSE = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(1, -1)).rgb);
    float lumaNE = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(1, 1)).rgb);
    float lumaSW = FxaaLuma(textureLodOffset(colorTex, posM, 0.0, ivec2(-1, -1)).rgb);

    float lumaNS = lumaN + lumaS;
    float lumaWE = lumaW + lumaE;
    float subpixRcpRange = 1.0 / range;
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
    highp float lengthSign = rcpFrame.x;
    bool horzSpan = edgeHorz >= edgeVert;
    float subpixA = subpixNSWE * 2.0 + subpixNWSWNESE;

    if (!horzSpan) lumaN = lumaW;
    if (!horzSpan) lumaS = lumaE;
    if (horzSpan) lengthSign = rcpFrame.y;
    float subpixB = (subpixA * (1.0 / 12.0)) - lumaM;

    float gradientN = lumaN - lumaM;
    float gradientS = lumaS - lumaM;
    float lumaNN = lumaN + lumaM;
    float lumaSS = lumaS + lumaM;
    bool pairN = abs(gradientN) >= abs(gradientS);
    float gradient = max(abs(gradientN), abs(gradientS));
    if (pairN) lengthSign = -lengthSign;
    float subpixC = clamp(abs(subpixB) * subpixRcpRange, 0.0, 1.0);

    highp vec2 posB = posM;
    highp vec2 offNP;
    offNP.x = (!horzSpan) ? 0.0 : rcpFrame.x;
    offNP.y = (horzSpan) ? 0.0 : rcpFrame.y;
    if (!horzSpan) posB.x += lengthSign * 0.5;
    if (horzSpan) posB.y += lengthSign * 0.5;

    highp vec2 posN = posB - offNP;
    highp vec2 posP = posB + offNP;
    float subpixD = ((-2.0) * subpixC) + 3.0;
    float lumaEndN = FxaaLuma(textureLod(colorTex, posN, 0.0).rgb);
    float subpixE = subpixC * subpixC;
    float lumaEndP = FxaaLuma(textureLod(colorTex, posP, 0.0).rgb);

    if (!pairN) lumaNN = lumaSS;
    float gradientScaled = gradient * 0.25;
    bool lumaMLTZero = (lumaM - lumaNN * 0.5) < 0.0;

    float subpixF = subpixD * subpixE;
    lumaEndN -= lumaNN * 0.5;
    lumaEndP -= lumaNN * 0.5;
    bool doneN = abs(lumaEndN) >= gradientScaled;
    bool doneP = abs(lumaEndP) >= gradientScaled;
    if (!doneN) posN -= offNP * 1.5;
    if (!doneP) posP += offNP * 1.5;

    float steps[4] = float[4](2.0, 2.0, 2.0, 4.0);
    for (int s = 0; s < 4; s++) {
        if (doneN && doneP) break;
        if (!doneN) lumaEndN = FxaaLuma(textureLod(colorTex, posN, 0.0).rgb) - lumaNN * 0.5;
        if (!doneP) lumaEndP = FxaaLuma(textureLod(colorTex, posP, 0.0).rgb) - lumaNN * 0.5;
        doneN = abs(lumaEndN) >= gradientScaled;
        doneP = abs(lumaEndP) >= gradientScaled;
        if (!doneN) posN -= offNP * steps[s];
        if (!doneP) posP += offNP * steps[s];
    }

    highp float dstN = horzSpan ? (posM.x - posN.x) : (posM.y - posN.y);
    highp float dstP = horzSpan ? (posP.x - posM.x) : (posP.y - posM.y);
    bool directionN = dstN < dstP;
    highp float dst = min(dstN, dstP);
    bool goodSpan = ((directionN ? lumaEndN : lumaEndP) < 0.0) != lumaMLTZero;
    highp float spanLength = dstP + dstN;
    float pixelOffset = (dst * (-1.0 / spanLength)) + 0.5;
    float subpixOffset = subpixF * subpixF * 0.75;
    highp float finalOffset = max(goodSpan ? pixelOffset : 0.0, subpixOffset);

    if (!horzSpan) posM.x += finalOffset * lengthSign;
    if (horzSpan) posM.y += finalOffset * lengthSign;
    fragColor = vec4(textureLod(colorTex, posM, 0.0).rgb, rgbyM.a);
}
"#;

const TEMPORAL_VS_SRC: &str = r#"#version 300 es
precision highp float;
void main() {
    vec2 uv = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const TEMPORAL_PS_SRC: &str = r#"#version 300 es
precision highp float;
precision highp sampler2D;
uniform sampler2D colorTex;
uniform sampler2D historyTex;
uniform vec4 rtMetrics;
out vec4 fragColor;
void main() {
    ivec2 pxCoord = ivec2(gl_FragCoord.xy);
    ivec2 maxCoord = ivec2(rtMetrics.zw) - ivec2(1);
    vec4 curr = texelFetch(colorTex, pxCoord, 0);
    vec3 hist = texelFetch(historyTex, pxCoord, 0).rgb;

    vec3 m1 = vec3(0.0);
    vec3 m2 = vec3(0.0);
    vec3 minC = curr.rgb;
    vec3 maxC = curr.rgb;

    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            ivec2 sc = clamp(pxCoord + ivec2(x, y), ivec2(0), maxCoord);
            vec3 c = texelFetch(colorTex, sc, 0).rgb;
            m1 += c;
            m2 += c * c;
            minC = min(minC, c);
            maxC = max(maxC, c);
        }
    }

    vec3 mean = m1 / 9.0;
    vec3 stddev = sqrt(max(vec3(0.0), m2 / 9.0 - mean * mean));
    vec3 clipMin = max(minC, mean - stddev * 0.75);
    vec3 clipMax = min(maxC, mean + stddev * 0.75);
    vec3 clampedHist = clamp(hist, clipMin, clipMax);

    vec3 rangeVec = maxC - minC;
    float localRange = max(max(rangeVec.r, rangeVec.g), rangeVec.b);
    float edgeFactor = smoothstep(0.03, 0.12, localRange);

    vec3 outVec = max(vec3(0.0), max(minC - hist, hist - maxC));
    float distOutside = max(max(outVec.r, outVec.g), outVec.b);
    float motionReject = 1.0 - smoothstep(0.01, 0.10, distOutside);
    float alpha = 0.50 * edgeFactor * motionReject;
    fragColor = vec4(mix(curr.rgb, clampedHist, alpha), curr.a);
}
"#;

fn compile_program(gl: &glow::Context, vs_src: &str, fs_src: &str, name: &str) -> Option<glow::NativeProgram> {
    unsafe {
        let vs = gl.create_shader(glow::VERTEX_SHADER).ok()?;
        gl.shader_source(vs, vs_src);
        gl.compile_shader(vs);
        if !gl.get_shader_compile_status(vs) {
            error!("SMAA {} VS: {}", name, gl.get_shader_info_log(vs));
            gl.delete_shader(vs);
            return None;
        }

        let fs = gl.create_shader(glow::FRAGMENT_SHADER).ok()?;
        gl.shader_source(fs, fs_src);
        gl.compile_shader(fs);
        if !gl.get_shader_compile_status(fs) {
            error!("SMAA {} FS: {}", name, gl.get_shader_info_log(fs));
            gl.delete_shader(vs);
            gl.delete_shader(fs);
            return None;
        }

        let prog = gl.create_program().ok()?;
        gl.attach_shader(prog, vs);
        gl.attach_shader(prog, fs);
        gl.link_program(prog);
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        if !gl.get_program_link_status(prog) {
            error!("SMAA {} link: {}", name, gl.get_program_info_log(prog));
            gl.delete_program(prog);
            return None;
        }
        Some(prog)
    }
}

fn build_smaa_sources(preset_macro: &str, is_vs: bool, main_body: &str) -> String {
    let include_flags = if is_vs {
        "#define SMAA_INCLUDE_VS 1\n#define SMAA_INCLUDE_PS 0\n"
    } else {
        "#define SMAA_INCLUDE_VS 0\n#define SMAA_INCLUDE_PS 1\n"
    };
    let vs_helper = if is_vs { FULLSCREEN_VS_BODY } else { "" };
    format!(
        "#version 300 es\n\
         precision highp float;\n\
         precision highp int;\n\
         precision highp sampler2D;\n\
         uniform vec4 rtMetrics;\n\
         #define SMAA_RT_METRICS rtMetrics\n\
         #define SMAA_GLSL_3 1\n\
         {}{}{}{}{}",
        preset_macro, include_flags, OFFICIAL_SMAA_HLSL, vs_helper, main_body
    )
}

struct SmaaPrograms {
    quality: SmaaQuality,
    edge_prog: glow::NativeProgram,
    edge_rt_metrics: Option<glow::UniformLocation>,
    edge_color_tex: Option<glow::UniformLocation>,
    weight_prog: glow::NativeProgram,
    weight_rt_metrics: Option<glow::UniformLocation>,
    weight_edges_tex: Option<glow::UniformLocation>,
    weight_area_tex: Option<glow::UniformLocation>,
    weight_search_tex: Option<glow::UniformLocation>,
    blend_prog: glow::NativeProgram,
    blend_rt_metrics: Option<glow::UniformLocation>,
    blend_color_tex: Option<glow::UniformLocation>,
    blend_blend_tex: Option<glow::UniformLocation>,
}

impl SmaaPrograms {
    fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.edge_prog);
            gl.delete_program(self.weight_prog);
            gl.delete_program(self.blend_prog);
        }
    }
}

fn compile_preset(gl: &glow::Context, quality: SmaaQuality) -> Option<SmaaPrograms> {
    let preset_macro = match quality {
        SmaaQuality::Off | SmaaQuality::Fxaa => return None,
        SmaaQuality::Low => "#define SMAA_PRESET_LOW 1\n",
        SmaaQuality::Medium => "#define SMAA_PRESET_MEDIUM 1\n",
        SmaaQuality::High => "#define SMAA_PRESET_HIGH 1\n",
        SmaaQuality::Ultra => "#define SMAA_PRESET_ULTRA 1\n",
    };

    let edge_vs = build_smaa_sources(preset_macro, true, EDGE_VS_MAIN);
    let edge_fs = build_smaa_sources(preset_macro, false, EDGE_PS_MAIN);
    let weight_vs = build_smaa_sources(preset_macro, true, WEIGHT_VS_MAIN);
    let weight_fs = build_smaa_sources(preset_macro, false, WEIGHT_PS_MAIN);
    let blend_vs = build_smaa_sources(preset_macro, true, BLEND_VS_MAIN);
    let blend_fs = build_smaa_sources(preset_macro, false, BLEND_PS_MAIN);

    let edge_prog = compile_program(gl, &edge_vs, &edge_fs, "Edge")?;
    let weight_prog = compile_program(gl, &weight_vs, &weight_fs, "Weight").or_else(|| {
        unsafe { gl.delete_program(edge_prog) };
        None
    })?;
    let blend_prog = compile_program(gl, &blend_vs, &blend_fs, "Blend").or_else(|| {
        unsafe {
            gl.delete_program(edge_prog);
            gl.delete_program(weight_prog);
        }
        None
    })?;

    unsafe {
        info!("Android SMAA preset compiled");
        Some(SmaaPrograms {
            quality,
            edge_rt_metrics: gl.get_uniform_location(edge_prog, "rtMetrics"),
            edge_color_tex: gl.get_uniform_location(edge_prog, "colorTex"),
            edge_prog,
            weight_rt_metrics: gl.get_uniform_location(weight_prog, "rtMetrics"),
            weight_edges_tex: gl.get_uniform_location(weight_prog, "edgesTex"),
            weight_area_tex: gl.get_uniform_location(weight_prog, "areaTex"),
            weight_search_tex: gl.get_uniform_location(weight_prog, "searchTex"),
            weight_prog,
            blend_rt_metrics: gl.get_uniform_location(blend_prog, "rtMetrics"),
            blend_color_tex: gl.get_uniform_location(blend_prog, "colorTex"),
            blend_blend_tex: gl.get_uniform_location(blend_prog, "blendTex"),
            blend_prog,
        })
    }
}

struct RenderTargets {
    width: i32,
    height: i32,
    has_smaa_targets: bool,
    color_tex: glow::NativeTexture,
    color_fbo: glow::NativeFramebuffer,
    edges_tex: Option<glow::NativeTexture>,
    edges_fbo: Option<glow::NativeFramebuffer>,
    blend_tex: Option<glow::NativeTexture>,
    blend_fbo: Option<glow::NativeFramebuffer>,
    stencil_rb: Option<glow::NativeRenderbuffer>,
    history_tex: glow::NativeTexture,
    history_fbo: glow::NativeFramebuffer,
    has_history: bool,
}

impl RenderTargets {
    fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_framebuffer(self.color_fbo);
            if let Some(fbo) = self.edges_fbo {
                gl.delete_framebuffer(fbo);
            }
            if let Some(fbo) = self.blend_fbo {
                gl.delete_framebuffer(fbo);
            }
            gl.delete_framebuffer(self.history_fbo);
            if let Some(rb) = self.stencil_rb {
                gl.delete_renderbuffer(rb);
            }
            gl.delete_texture(self.color_tex);
            if let Some(tex) = self.edges_tex {
                gl.delete_texture(tex);
            }
            if let Some(tex) = self.blend_tex {
                gl.delete_texture(tex);
            }
            gl.delete_texture(self.history_tex);
        }
    }
}

fn create_rt_texture(gl: &glow::Context, width: i32, height: i32, internal_fmt: i32, fmt: u32, filter: i32) -> Option<glow::NativeTexture> {
    unsafe {
        let tex = gl.create_texture().ok()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            internal_fmt,
            width,
            height,
            0,
            fmt,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        Some(tex)
    }
}

fn create_static_texture(gl: &glow::Context, width: i32, height: i32, internal_fmt: i32, fmt: u32, filter: i32, bytes: &[u8]) -> Option<glow::NativeTexture> {
    unsafe {
        let prev_align = gl.get_parameter_i32(glow::UNPACK_ALIGNMENT);
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        let tex = gl.create_texture().ok()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            internal_fmt,
            width,
            height,
            0,
            fmt,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(bytes)),
        );
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, prev_align);
        Some(tex)
    }
}

fn create_fbo_for_tex(
    gl: &glow::Context,
    tex: glow::NativeTexture,
    stencil: Option<(glow::NativeRenderbuffer, u32)>,
) -> Option<glow::NativeFramebuffer> {
    unsafe {
        let fbo = gl.create_framebuffer().ok()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        if let Some((rb, attachment)) = stencil {
            gl.framebuffer_renderbuffer(glow::FRAMEBUFFER, attachment, glow::RENDERBUFFER, Some(rb));
        }
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            gl.delete_framebuffer(fbo);
            return None;
        }
        Some(fbo)
    }
}

fn create_stencil_rb(gl: &glow::Context, width: i32, height: i32, fmt: u32) -> Option<glow::NativeRenderbuffer> {
    unsafe {
        let rb = gl.create_renderbuffer().ok()?;
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
        gl.renderbuffer_storage(glow::RENDERBUFFER, fmt, width, height);
        Some(rb)
    }
}

pub struct SmaaPass {
    gl: Arc<glow::Context>,
    vao: glow::NativeVertexArray,
    area_tex: glow::NativeTexture,
    search_tex: glow::NativeTexture,
    programs: Option<SmaaPrograms>,
    fxaa_prog: Option<glow::NativeProgram>,
    fxaa_rt_metrics: Option<glow::UniformLocation>,
    fxaa_color_tex: Option<glow::UniformLocation>,
    temporal_prog: Option<glow::NativeProgram>,
    temporal_rt_metrics: Option<glow::UniformLocation>,
    temporal_color_tex: Option<glow::UniformLocation>,
    temporal_history_tex: Option<glow::UniformLocation>,
    targets: Option<RenderTargets>,
}

impl SmaaPass {
    pub fn new(gl: Arc<glow::Context>) -> Option<Self> {
        unsafe {
            let prev_active = gl.get_parameter_i32(glow::ACTIVE_TEXTURE) as u32;
            gl.active_texture(glow::TEXTURE0);
            let prev_tex = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);

            let vao = gl.create_vertex_array().ok()?;
            let area_tex = create_static_texture(
                &gl,
                160,
                560,
                glow::RG8 as i32,
                glow::RG,
                glow::LINEAR as i32,
                AREA_TEX_BYTES,
            )?;
            let search_tex = create_static_texture(
                &gl,
                64,
                16,
                glow::R8 as i32,
                glow::RED,
                glow::NEAREST as i32,
                SEARCH_TEX_BYTES,
            )?;

            gl.bind_texture(glow::TEXTURE_2D, std::num::NonZeroU32::new(prev_tex as u32).map(glow::NativeTexture));
            gl.active_texture(prev_active);

            Some(Self {
                gl,
                vao,
                area_tex,
                search_tex,
                programs: None,
                fxaa_prog: None,
                fxaa_rt_metrics: None,
                fxaa_color_tex: None,
                temporal_prog: None,
                temporal_rt_metrics: None,
                temporal_color_tex: None,
                temporal_history_tex: None,
                targets: None,
            })
        }
    }

    fn ensure_targets(&mut self, width: i32, height: i32, need_smaa_targets: bool) -> Option<&mut RenderTargets> {
        let needs_recreate = match &self.targets {
            Some(t) => t.width != width || t.height != height || (need_smaa_targets && !t.has_smaa_targets),
            None => true,
        };
        if needs_recreate {
            if let Some(old) = self.targets.take() {
                old.destroy(&self.gl);
            }
            let color_tex = create_rt_texture(&self.gl, width, height, glow::RGBA8 as i32, glow::RGBA, glow::LINEAR as i32)?;
            let color_fbo = create_fbo_for_tex(&self.gl, color_tex, None)?;
            let history_tex = create_rt_texture(&self.gl, width, height, glow::RGBA8 as i32, glow::RGBA, glow::NEAREST as i32)?;
            let history_fbo = create_fbo_for_tex(&self.gl, history_tex, None)?;

            let (edges_tex, edges_fbo, blend_tex, blend_fbo, stencil_rb) = if need_smaa_targets {
                let edges_tex = create_rt_texture(&self.gl, width, height, glow::RG8 as i32, glow::RG, glow::LINEAR as i32)?;
                let blend_tex = create_rt_texture(&self.gl, width, height, glow::RGBA8 as i32, glow::RGBA, glow::LINEAR as i32)?;

                let prev_rb = unsafe {
                    std::num::NonZeroU32::new(self.gl.get_parameter_i32(glow::RENDERBUFFER_BINDING) as u32)
                        .map(glow::NativeRenderbuffer)
                };
                let mut stencil_rb = None;
                let mut fbo_pair = None;
                for (rb_fmt, attach) in [
                    (glow::STENCIL_INDEX8, glow::STENCIL_ATTACHMENT),
                    (glow::DEPTH24_STENCIL8, glow::DEPTH_STENCIL_ATTACHMENT),
                ] {
                    if let Some(rb) = create_stencil_rb(&self.gl, width, height, rb_fmt) {
                        if let Some(ef) = create_fbo_for_tex(&self.gl, edges_tex, Some((rb, attach))) {
                            if let Some(bf) = create_fbo_for_tex(&self.gl, blend_tex, Some((rb, attach))) {
                                stencil_rb = Some(rb);
                                fbo_pair = Some((ef, bf));
                                break;
                            }
                            unsafe { self.gl.delete_framebuffer(ef) };
                        }
                        unsafe { self.gl.delete_renderbuffer(rb) };
                    }
                }
                unsafe { self.gl.bind_renderbuffer(glow::RENDERBUFFER, prev_rb) };

                let (edges_fbo, blend_fbo) = match fbo_pair {
                    Some(pair) => pair,
                    None => (
                        create_fbo_for_tex(&self.gl, edges_tex, None)?,
                        create_fbo_for_tex(&self.gl, blend_tex, None)?,
                    ),
                };
                (Some(edges_tex), Some(edges_fbo), Some(blend_tex), Some(blend_fbo), stencil_rb)
            } else {
                (None, None, None, None, None)
            };

            self.targets = Some(RenderTargets {
                width,
                height,
                has_smaa_targets: need_smaa_targets,
                color_tex,
                color_fbo,
                edges_tex,
                edges_fbo,
                blend_tex,
                blend_fbo,
                stencil_rb,
                history_tex,
                history_fbo,
                has_history: false,
            });
        }
        self.targets.as_mut()
    }

    pub fn apply(&mut self, width: i32, height: i32, quality: SmaaQuality, temporal: bool) {
        self.apply_to_fbo(None, width, height, quality, temporal);
    }

    pub fn apply_to_fbo(
        &mut self,
        target_fbo: Option<glow::NativeFramebuffer>,
        width: i32,
        height: i32,
        quality: SmaaQuality,
        temporal: bool,
    ) -> bool {
        if quality == SmaaQuality::Off {
            if let Some(t) = &mut self.targets {
                t.has_history = false;
            }
            return false;
        }
        if width <= 0 || height <= 0 {
            return false;
        }

        if quality == SmaaQuality::Fxaa && self.fxaa_prog.is_none() {
            if let Some(prog) = compile_program(&self.gl, FXAA_VS_SRC, FXAA_PS_SRC, "FXAA") {
                unsafe {
                    self.fxaa_rt_metrics = self.gl.get_uniform_location(prog, "rtMetrics");
                    self.fxaa_color_tex = self.gl.get_uniform_location(prog, "colorTex");
                }
                self.fxaa_prog = Some(prog);
                info!("Android FXAA 3.11 compiled");
            }
        }

        let need_smaa = !matches!(quality, SmaaQuality::Off | SmaaQuality::Fxaa);
        if need_smaa && self.programs.as_ref().map(|p| p.quality) != Some(quality) {
            if let Some(old) = self.programs.take() {
                old.destroy(&self.gl);
            }
            self.programs = compile_preset(&self.gl, quality);
        }

        if temporal && self.temporal_prog.is_none() {
            if let Some(prog) = compile_program(&self.gl, TEMPORAL_VS_SRC, TEMPORAL_PS_SRC, "Temporal") {
                unsafe {
                    self.temporal_rt_metrics = self.gl.get_uniform_location(prog, "rtMetrics");
                    self.temporal_color_tex = self.gl.get_uniform_location(prog, "colorTex");
                    self.temporal_history_tex = self.gl.get_uniform_location(prog, "historyTex");
                }
                self.temporal_prog = Some(prog);
            }
        }

        let gl = self.gl.clone();
        unsafe {
            // Backup GL state
            let prev_draw_fbo = std::num::NonZeroU32::new(gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32).map(glow::NativeFramebuffer);
            let prev_read_fbo = std::num::NonZeroU32::new(gl.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING) as u32).map(glow::NativeFramebuffer);
            let mut prev_vp = [0i32; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut prev_vp);
            let prev_vao = std::num::NonZeroU32::new(gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING) as u32).map(glow::NativeVertexArray);
            let prev_prog = std::num::NonZeroU32::new(gl.get_parameter_i32(glow::CURRENT_PROGRAM) as u32).map(glow::NativeProgram);
            let prev_active = gl.get_parameter_i32(glow::ACTIVE_TEXTURE) as u32;
            let mut prev_tex = [None; 3];
            for (i, slot) in prev_tex.iter_mut().enumerate() {
                gl.active_texture(glow::TEXTURE0 + i as u32);
                *slot = std::num::NonZeroU32::new(gl.get_parameter_i32(glow::TEXTURE_BINDING_2D) as u32).map(glow::NativeTexture);
            }
            let prev_scissor = gl.is_enabled(glow::SCISSOR_TEST);
            let prev_cull = gl.is_enabled(glow::CULL_FACE);
            let prev_depth = gl.is_enabled(glow::DEPTH_TEST);
            let prev_stencil = gl.is_enabled(glow::STENCIL_TEST);
            let prev_blend = gl.is_enabled(glow::BLEND);

            if self.ensure_targets(width, height, need_smaa).is_some() {
                let t = self.targets.as_mut().unwrap();
                gl.viewport(0, 0, width, height);
                gl.disable(glow::SCISSOR_TEST);
                gl.disable(glow::CULL_FACE);
                gl.disable(glow::DEPTH_TEST);
                gl.disable(glow::STENCIL_TEST);
                gl.disable(glow::BLEND);
                gl.color_mask(true, true, true, true);
                gl.bind_vertex_array(Some(self.vao));

                let metrics = [1.0 / width as f32, 1.0 / height as f32, width as f32, height as f32];

                if quality == SmaaQuality::Fxaa {
                    if let Some(fxaa_prog) = self.fxaa_prog {
                        blit_fbo_to_fbo(&gl, target_fbo, Some(t.color_fbo), width, height);
                        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, target_fbo);
                        gl.use_program(Some(fxaa_prog));
                        gl.uniform_4_f32(self.fxaa_rt_metrics.as_ref(), metrics[0], metrics[1], metrics[2], metrics[3]);
                        gl.active_texture(glow::TEXTURE0);
                        gl.bind_texture(glow::TEXTURE_2D, Some(t.color_tex));
                        gl.uniform_1_i32(self.fxaa_color_tex.as_ref(), 0);
                        gl.draw_arrays(glow::TRIANGLES, 0, 3);
                    }
                } else if let (Some(p), Some(edges_fbo), Some(edges_tex), Some(blend_fbo), Some(blend_tex)) = (
                    self.programs.as_ref(),
                    t.edges_fbo,
                    t.edges_tex,
                    t.blend_fbo,
                    t.blend_tex,
                ) {
                    // Blit target FBO -> color_fbo
                    blit_fbo_to_fbo(&gl, target_fbo, Some(t.color_fbo), width, height);

                    // Pass 1: Edge Detection (marks edge pixels in stencil = 1)
                    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(edges_fbo));
                    gl.clear_color(0.0, 0.0, 0.0, 0.0);
                    if t.stencil_rb.is_some() {
                        gl.enable(glow::STENCIL_TEST);
                        gl.stencil_mask(0xFF);
                        gl.clear_stencil(0);
                        gl.clear(glow::COLOR_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
                        gl.stencil_func(glow::ALWAYS, 1, 0xFF);
                        gl.stencil_op(glow::KEEP, glow::KEEP, glow::REPLACE);
                    } else {
                        gl.clear(glow::COLOR_BUFFER_BIT);
                    }
                    gl.use_program(Some(p.edge_prog));
                    gl.uniform_4_f32(p.edge_rt_metrics.as_ref(), metrics[0], metrics[1], metrics[2], metrics[3]);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, Some(t.color_tex));
                    gl.uniform_1_i32(p.edge_color_tex.as_ref(), 0);
                    gl.draw_arrays(glow::TRIANGLES, 0, 3);

                    // Pass 2: Blending Weight Calculation (early-stencil skips non-edge pixels)
                    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(blend_fbo));
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    if t.stencil_rb.is_some() {
                        gl.stencil_mask(0x00);
                        gl.stencil_func(glow::EQUAL, 1, 0xFF);
                        gl.stencil_op(glow::KEEP, glow::KEEP, glow::KEEP);
                    }
                    gl.use_program(Some(p.weight_prog));
                    gl.uniform_4_f32(p.weight_rt_metrics.as_ref(), metrics[0], metrics[1], metrics[2], metrics[3]);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, Some(edges_tex));
                    gl.uniform_1_i32(p.weight_edges_tex.as_ref(), 0);
                    gl.active_texture(glow::TEXTURE1);
                    gl.bind_texture(glow::TEXTURE_2D, Some(self.area_tex));
                    gl.uniform_1_i32(p.weight_area_tex.as_ref(), 1);
                    gl.active_texture(glow::TEXTURE2);
                    gl.bind_texture(glow::TEXTURE_2D, Some(self.search_tex));
                    gl.uniform_1_i32(p.weight_search_tex.as_ref(), 2);
                    gl.draw_arrays(glow::TRIANGLES, 0, 3);

                    if t.stencil_rb.is_some() {
                        gl.disable(glow::STENCIL_TEST);
                        gl.stencil_mask(0xFF);
                        gl.stencil_func(glow::ALWAYS, 0, 0xFF);
                        gl.invalidate_framebuffer(glow::DRAW_FRAMEBUFFER, &[glow::STENCIL_ATTACHMENT]);
                        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(edges_fbo));
                        gl.invalidate_framebuffer(glow::DRAW_FRAMEBUFFER, &[glow::COLOR_ATTACHMENT0, glow::STENCIL_ATTACHMENT]);
                    }

                    // Pass 3: Neighborhood Blending -> target FBO
                    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, target_fbo);
                    gl.use_program(Some(p.blend_prog));
                    gl.uniform_4_f32(p.blend_rt_metrics.as_ref(), metrics[0], metrics[1], metrics[2], metrics[3]);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, Some(t.color_tex));
                    gl.uniform_1_i32(p.blend_color_tex.as_ref(), 0);
                    gl.active_texture(glow::TEXTURE1);
                    gl.bind_texture(glow::TEXTURE_2D, Some(blend_tex));
                    gl.uniform_1_i32(p.blend_blend_tex.as_ref(), 1);
                    gl.draw_arrays(glow::TRIANGLES, 0, 3);
                }

                if temporal {
                    if let Some(t_prog) = self.temporal_prog {
                        if !t.has_history {
                            blit_fbo_to_fbo(&gl, target_fbo, Some(t.history_fbo), width, height);
                            t.has_history = true;
                        } else {
                            blit_fbo_to_fbo(&gl, target_fbo, Some(t.color_fbo), width, height);

                            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, target_fbo);
                            gl.use_program(Some(t_prog));
                            gl.uniform_4_f32(self.temporal_rt_metrics.as_ref(), metrics[0], metrics[1], metrics[2], metrics[3]);
                            gl.active_texture(glow::TEXTURE0);
                            gl.bind_texture(glow::TEXTURE_2D, Some(t.color_tex));
                            gl.uniform_1_i32(self.temporal_color_tex.as_ref(), 0);
                            gl.active_texture(glow::TEXTURE1);
                            gl.bind_texture(glow::TEXTURE_2D, Some(t.history_tex));
                            gl.uniform_1_i32(self.temporal_history_tex.as_ref(), 1);
                            gl.draw_arrays(glow::TRIANGLES, 0, 3);

                            blit_fbo_to_fbo(&gl, target_fbo, Some(t.history_fbo), width, height);
                        }
                    }
                } else {
                    t.has_history = false;
                }
            }

            // Restore GL state
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, prev_draw_fbo);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, prev_read_fbo);
            gl.viewport(prev_vp[0], prev_vp[1], prev_vp[2], prev_vp[3]);
            gl.bind_vertex_array(prev_vao);
            if prev_prog.is_none() || gl.is_program(prev_prog.unwrap()) {
                gl.use_program(prev_prog);
            }
            for (i, slot) in prev_tex.into_iter().enumerate() {
                gl.active_texture(glow::TEXTURE0 + i as u32);
                gl.bind_texture(glow::TEXTURE_2D, slot);
            }
            gl.active_texture(prev_active);
            if prev_scissor { gl.enable(glow::SCISSOR_TEST) } else { gl.disable(glow::SCISSOR_TEST) }
            if prev_cull { gl.enable(glow::CULL_FACE) } else { gl.disable(glow::CULL_FACE) }
            if prev_depth { gl.enable(glow::DEPTH_TEST) } else { gl.disable(glow::DEPTH_TEST) }
            if prev_stencil { gl.enable(glow::STENCIL_TEST) } else { gl.disable(glow::STENCIL_TEST) }
            if prev_blend { gl.enable(glow::BLEND) } else { gl.disable(glow::BLEND) }
        }
        true
    }
}

fn blit_fbo_to_fbo(
    gl: &glow::Context,
    src: Option<glow::NativeFramebuffer>,
    dst: Option<glow::NativeFramebuffer>,
    width: i32,
    height: i32,
) {
    unsafe {
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, src);
        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, dst);
        gl.blit_framebuffer(
            0,
            0,
            width,
            height,
            0,
            0,
            width,
            height,
            glow::COLOR_BUFFER_BIT,
            glow::NEAREST,
        );
    }
}

impl Drop for SmaaPass {
    fn drop(&mut self) {
        if let Some(t) = self.targets.take() {
            t.destroy(&self.gl);
        }
        if let Some(p) = self.programs.take() {
            p.destroy(&self.gl);
        }
        unsafe {
            if let Some(prog) = self.fxaa_prog.take() {
                self.gl.delete_program(prog);
            }
            if let Some(prog) = self.temporal_prog.take() {
                self.gl.delete_program(prog);
            }
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_texture(self.area_tex);
            self.gl.delete_texture(self.search_tex);
        }
    }
}
