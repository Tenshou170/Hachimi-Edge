use std::ffi::{c_void, CStr};

use log::{error, info};
use windows::{
    core::{w, HRESULT, Interface, PCSTR},
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{ID3DBlob, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST},
            Direct3D11::{
                ID3D11BlendState, ID3D11Buffer, ID3D11DepthStencilState, ID3D11Device,
                ID3D11DeviceContext, ID3D11PixelShader, ID3D11RasterizerState,
                ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView,
                ID3D11Texture2D, ID3D11VertexShader, D3D11_BIND_CONSTANT_BUFFER,
                D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BLEND_DESC,
                D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_ALWAYS, D3D11_CPU_ACCESS_WRITE,
                D3D11_CULL_NONE, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ZERO,
                D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_LINEAR_MIP_POINT,
                D3D11_FILTER_MIN_MAG_MIP_POINT, D3D11_MAPPED_SUBRESOURCE,
                D3D11_MAP_WRITE_DISCARD, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
                D3D11_SAMPLER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC,
                D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC,
                D3D11_USAGE_IMMUTABLE, D3D11_VIEWPORT,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8G8_UNORM,
                    DXGI_FORMAT_R8_UNORM, DXGI_SAMPLE_DESC,
                },
                IDXGISwapChain,
            },
        },
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
};

use crate::windows::hachimi_impl::SmaaQuality;

const AREA_TEX_BYTES: &[u8] = include_bytes!("../../../assets/smaa_area.bin");
const SEARCH_TEX_BYTES: &[u8] = include_bytes!("../../../assets/smaa_search.bin");
const OFFICIAL_SMAA_HLSL: &str = include_str!("../../../assets/SMAA.hlsl");

type D3DCompileFn = unsafe extern "system" fn(
    p_src_data: *const c_void,
    src_data_size: usize,
    p_source_name: *const i8,
    p_defines: *const c_void,
    p_include: *const c_void,
    p_entrypoint: *const i8,
    p_target: *const i8,
    flags1: u32,
    flags2: u32,
    pp_code: *mut *mut c_void,
    pp_error_msgs: *mut *mut c_void,
) -> HRESULT;

#[repr(C)]
#[derive(Copy, Clone)]
struct SmaaConstants {
    rt_metrics: [f32; 4], // (1/w, 1/h, w, h)
}

const WRAPPER_PREAMBLE: &str = r#"
cbuffer SmaaCB : register(b0) {
    float4 rtMetrics;
};
#define SMAA_RT_METRICS rtMetrics
#define SMAA_HLSL_4_1
"#;

const WRAPPER_ENTRYPOINTS: &str = r#"
Texture2D colorTex   : register(t0);
Texture2D edgesTex   : register(t0);
Texture2D areaTex    : register(t1);
Texture2D blendTex   : register(t1);
Texture2D historyTex : register(t1);
Texture2D searchTex  : register(t2);

struct EdgeVSOut {
    float4 pos : SV_POSITION;
    float2 uv  : TEXCOORD0;
    float4 offset[3] : TEXCOORD1;
};

struct WeightVSOut {
    float4 pos : SV_POSITION;
    float2 uv  : TEXCOORD0;
    float2 pixcoord : TEXCOORD1;
    float4 offset[3] : TEXCOORD2;
};

struct BlendVSOut {
    float4 pos : SV_POSITION;
    float2 uv  : TEXCOORD0;
    float4 offset : TEXCOORD1;
};

void FullscreenQuad(uint id, out float4 pos, out float2 uv) {
    uv = float2((id << 1) & 2, id & 2);
    pos = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
}

EdgeVSOut EdgeVS(uint id : SV_VertexID) {
    EdgeVSOut o;
    FullscreenQuad(id, o.pos, o.uv);
    SMAAEdgeDetectionVS(o.uv, o.offset);
    return o;
}

float2 EdgePS(EdgeVSOut i) : SV_TARGET {
    return SMAAColorEdgeDetectionPS(i.uv, i.offset, colorTex);
}

WeightVSOut WeightVS(uint id : SV_VertexID) {
    WeightVSOut o;
    FullscreenQuad(id, o.pos, o.uv);
    SMAABlendingWeightCalculationVS(o.uv, o.pixcoord, o.offset);
    return o;
}

float4 WeightPS(WeightVSOut i) : SV_TARGET {
    return SMAABlendingWeightCalculationPS(i.uv, i.pixcoord, i.offset, edgesTex, areaTex, searchTex, float4(0, 0, 0, 0));
}

BlendVSOut BlendVS(uint id : SV_VertexID) {
    BlendVSOut o;
    FullscreenQuad(id, o.pos, o.uv);
    SMAANeighborhoodBlendingVS(o.uv, o.offset);
    return o;
}

float4 BlendPS(BlendVSOut i) : SV_TARGET {
    return SMAANeighborhoodBlendingPS(i.uv, i.offset, colorTex, blendTex);
}

struct TemporalVSOut {
    float4 pos : SV_POSITION;
    float2 uv  : TEXCOORD0;
};

TemporalVSOut TemporalVS(uint id : SV_VertexID) {
    TemporalVSOut o;
    FullscreenQuad(id, o.pos, o.uv);
    return o;
}

float4 TemporalPS(TemporalVSOut i) : SV_TARGET {
    float2 px = rtMetrics.xy;
    float4 curr = colorTex.SampleLevel(PointSampler, i.uv, 0);
    float3 hist = historyTex.SampleLevel(PointSampler, i.uv, 0).rgb;

    float3 m1 = float3(0, 0, 0);
    float3 m2 = float3(0, 0, 0);
    float3 minC = curr.rgb;
    float3 maxC = curr.rgb;

    [unroll]
    for (int y = -1; y <= 1; y++) {
        [unroll]
        for (int x = -1; x <= 1; x++) {
            float3 c = colorTex.SampleLevel(PointSampler, i.uv + float2(x, y) * px, 0).rgb;
            m1 += c;
            m2 += c * c;
            minC = min(minC, c);
            maxC = max(maxC, c);
        }
    }

    float3 mean = m1 / 9.0;
    float3 stddev = sqrt(max(0.0, m2 / 9.0 - mean * mean));
    float3 clipMin = max(minC, mean - stddev * 0.75);
    float3 clipMax = min(maxC, mean + stddev * 0.75);
    float3 clampedHist = clamp(hist, clipMin, clipMax);

    // Gate temporal strength by local edge contrast so smooth moving surfaces keep crisp 60fps motion
    float3 rangeVec = maxC - minC;
    float localRange = max(max(rangeVec.r, rangeVec.g), rangeVec.b);
    float edgeFactor = smoothstep(0.03, 0.12, localRange);

    // Reject based on distance outside the 3x3 neighborhood so high-contrast 1px toon steps
    // oscillating inside the neighborhood still stabilize, while real motion rejects cleanly.
    float3 outVec = max(float3(0.0, 0.0, 0.0), max(minC - hist, hist - maxC));
    float distOutside = max(max(outVec.r, outVec.g), outVec.b);
    float motionReject = 1.0 - smoothstep(0.01, 0.10, distOutside);
    float alpha = 0.50 * edgeFactor * motionReject;
    return float4(lerp(curr.rgb, clampedHist, alpha), curr.a);
}
"#;

const FXAA_HLSL: &str = r#"
cbuffer SmaaCB : register(b0) {
    float4 rtMetrics;
};
Texture2D colorTex : register(t0);
SamplerState LinearSampler : register(s0);

struct FxaaVSOut {
    float4 pos : SV_POSITION;
    float2 uv  : TEXCOORD0;
};

FxaaVSOut FxaaVS(uint id : SV_VertexID) {
    FxaaVSOut o;
    o.uv = float2((id << 1) & 2, id & 2);
    o.pos = float4(o.uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    return o;
}

float FxaaLuma(float3 rgb) {
    return dot(rgb, float3(0.299, 0.587, 0.114));
}

float4 FxaaPS(FxaaVSOut i) : SV_TARGET {
    float2 posM = i.uv;
    float2 rcpFrame = rtMetrics.xy;
    float4 rgbyM = colorTex.SampleLevel(LinearSampler, posM, 0);
    float lumaM = FxaaLuma(rgbyM.rgb);
    float lumaS = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(0, 1)).rgb);
    float lumaE = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(1, 0)).rgb);
    float lumaN = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(0, -1)).rgb);
    float lumaW = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(-1, 0)).rgb);

    float maxSM = max(lumaS, lumaM);
    float minSM = min(lumaS, lumaM);
    float maxESM = max(lumaE, maxSM);
    float minESM = min(lumaE, minSM);
    float maxWN = max(lumaN, lumaW);
    float minWN = min(lumaN, lumaW);
    float rangeMax = max(maxWN, maxESM);
    float rangeMin = min(minWN, minESM);
    float rangeMaxScaled = rangeMax * 0.125;
    float range = rangeMax - rangeMin;
    if (range < max(0.0625, rangeMaxScaled)) {
        return rgbyM;
    }

    float lumaNW = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(-1, -1)).rgb);
    float lumaSE = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(1, 1)).rgb);
    float lumaNE = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(1, -1)).rgb);
    float lumaSW = FxaaLuma(colorTex.SampleLevel(LinearSampler, posM, 0, int2(-1, 1)).rgb);

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
    float lengthSign = rcpFrame.x;
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
    float subpixC = saturate(abs(subpixB) * subpixRcpRange);

    float2 posB = posM;
    float2 offNP;
    offNP.x = (!horzSpan) ? 0.0 : rcpFrame.x;
    offNP.y = (horzSpan) ? 0.0 : rcpFrame.y;
    if (!horzSpan) posB.x += lengthSign * 0.5;
    if (horzSpan) posB.y += lengthSign * 0.5;

    float2 posN = posB - offNP;
    float2 posP = posB + offNP;
    float subpixD = ((-2.0) * subpixC) + 3.0;
    float lumaEndN = FxaaLuma(colorTex.SampleLevel(LinearSampler, posN, 0).rgb);
    float subpixE = subpixC * subpixC;
    float lumaEndP = FxaaLuma(colorTex.SampleLevel(LinearSampler, posP, 0).rgb);

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

    float steps[4] = { 2.0, 2.0, 2.0, 4.0 };
    [unroll]
    for (int s = 0; s < 4; s++) {
        if (doneN && doneP) break;
        if (!doneN) lumaEndN = FxaaLuma(colorTex.SampleLevel(LinearSampler, posN, 0).rgb) - lumaNN * 0.5;
        if (!doneP) lumaEndP = FxaaLuma(colorTex.SampleLevel(LinearSampler, posP, 0).rgb) - lumaNN * 0.5;
        doneN = abs(lumaEndN) >= gradientScaled;
        doneP = abs(lumaEndP) >= gradientScaled;
        if (!doneN) posN -= offNP * steps[s];
        if (!doneP) posP += offNP * steps[s];
    }

    float dstN = horzSpan ? (posM.x - posN.x) : (posM.y - posN.y);
    float dstP = horzSpan ? (posP.x - posM.x) : (posP.y - posM.y);
    bool directionN = dstN < dstP;
    float dst = min(dstN, dstP);
    bool goodSpan = ((directionN ? lumaEndN : lumaEndP) < 0.0) != lumaMLTZero;
    float spanLength = dstP + dstN;
    float pixelOffset = (dst * (-1.0 / spanLength)) + 0.5;
    float subpixOffset = subpixF * subpixF * 0.75;
    float finalOffset = max(goodSpan ? pixelOffset : 0.0, subpixOffset);

    if (!horzSpan) posM.x += finalOffset * lengthSign;
    if (horzSpan) posM.y += finalOffset * lengthSign;
    return float4(colorTex.SampleLevel(LinearSampler, posM, 0).rgb, rgbyM.a);
}
"#;

struct SmaaShaders {
    quality: SmaaQuality,
    edge_vs: ID3D11VertexShader,
    edge_ps: ID3D11PixelShader,
    weight_vs: ID3D11VertexShader,
    weight_ps: ID3D11PixelShader,
    blend_vs: ID3D11VertexShader,
    blend_ps: ID3D11PixelShader,
}

struct RenderTargets {
    width: u32,
    height: u32,
    format: DXGI_FORMAT,
    color_copy: ID3D11Texture2D,
    color_srv: ID3D11ShaderResourceView,
    edges_rtv: ID3D11RenderTargetView,
    edges_srv: ID3D11ShaderResourceView,
    blend_rtv: ID3D11RenderTargetView,
    blend_srv: ID3D11ShaderResourceView,
    history_tex: ID3D11Texture2D,
    history_srv: ID3D11ShaderResourceView,
    has_history: bool,
}

pub struct SmaaPass {
    compile: D3DCompileFn,
    shaders: Option<SmaaShaders>,
    fxaa_vs: Option<ID3D11VertexShader>,
    fxaa_ps: Option<ID3D11PixelShader>,
    temporal_vs: Option<ID3D11VertexShader>,
    temporal_ps: Option<ID3D11PixelShader>,
    linear_sampler: ID3D11SamplerState,
    point_sampler: ID3D11SamplerState,
    cbuffer: ID3D11Buffer,
    blend_state: ID3D11BlendState,
    raster_state: ID3D11RasterizerState,
    depth_state: ID3D11DepthStencilState,
    area_srv: ID3D11ShaderResourceView,
    search_srv: ID3D11ShaderResourceView,
    targets: Option<RenderTargets>,
}

fn load_d3d_compile() -> Option<D3DCompileFn> {
    unsafe {
        let module: HMODULE = LoadLibraryW(w!("d3dcompiler_47.dll"))
            .or_else(|_| LoadLibraryW(w!("d3dcompiler_43.dll")))
            .ok()?;
        let proc = GetProcAddress(module, PCSTR(b"D3DCompile\0".as_ptr()))?;
        Some(std::mem::transmute(proc))
    }
}

fn compile_shader(compile: D3DCompileFn, source: &str, entry: &CStr, target: &CStr) -> Result<Vec<u8>, String> {
    let mut code: *mut c_void = std::ptr::null_mut();
    let mut err: *mut c_void = std::ptr::null_mut();
    let hr = unsafe {
        compile(
            source.as_ptr() as *const c_void,
            source.len(),
            b"SMAA.hlsl\0".as_ptr() as *const i8,
            std::ptr::null(),
            std::ptr::null(),
            entry.as_ptr(),
            target.as_ptr(),
            1 << 15, // D3DCOMPILE_OPTIMIZATION_LEVEL3
            0,
            &mut code,
            &mut err,
        )
    };
    if hr.is_err() || code.is_null() {
        let msg = if !err.is_null() {
            let blob: ID3DBlob = unsafe { ID3DBlob::from_raw(err) };
            let ptr = unsafe { blob.GetBufferPointer() } as *const u8;
            let len = unsafe { blob.GetBufferSize() };
            String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(ptr, len) }).into_owned()
        } else {
            format!("HRESULT {:?}", hr)
        };
        return Err(msg);
    }
    let blob: ID3DBlob = unsafe { ID3DBlob::from_raw(code) };
    let ptr = unsafe { blob.GetBufferPointer() } as *const u8;
    let len = unsafe { blob.GetBufferSize() };
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec())
}

fn compile_preset(compile: D3DCompileFn, device: &ID3D11Device, quality: SmaaQuality) -> Option<SmaaShaders> {
    let preset_macro = match quality {
        SmaaQuality::Off | SmaaQuality::Fxaa => return None,
        SmaaQuality::Low => "#define SMAA_PRESET_LOW\n",
        SmaaQuality::Medium => "#define SMAA_PRESET_MEDIUM\n",
        SmaaQuality::High => "#define SMAA_PRESET_HIGH\n",
        SmaaQuality::Ultra => "#define SMAA_PRESET_ULTRA\n",
    };

    let source = format!("{}{}{}{}", WRAPPER_PREAMBLE, preset_macro, OFFICIAL_SMAA_HLSL, WRAPPER_ENTRYPOINTS);
    let vs_edge_bc = compile_shader(compile, &source, c"EdgeVS", c"vs_5_0").map_err(|e| error!("SMAA EdgeVS: {}", e)).ok()?;
    let ps_edge_bc = compile_shader(compile, &source, c"EdgePS", c"ps_5_0").map_err(|e| error!("SMAA EdgePS: {}", e)).ok()?;
    let vs_weight_bc = compile_shader(compile, &source, c"WeightVS", c"vs_5_0").map_err(|e| error!("SMAA WeightVS: {}", e)).ok()?;
    let ps_weight_bc = compile_shader(compile, &source, c"WeightPS", c"ps_5_0").map_err(|e| error!("SMAA WeightPS: {}", e)).ok()?;
    let vs_blend_bc = compile_shader(compile, &source, c"BlendVS", c"vs_5_0").map_err(|e| error!("SMAA BlendVS: {}", e)).ok()?;
    let ps_blend_bc = compile_shader(compile, &source, c"BlendPS", c"ps_5_0").map_err(|e| error!("SMAA BlendPS: {}", e)).ok()?;

    unsafe {
        let mut edge_vs = None;
        device.CreateVertexShader(&vs_edge_bc, None, Some(&mut edge_vs)).ok()?;
        let mut edge_ps = None;
        device.CreatePixelShader(&ps_edge_bc, None, Some(&mut edge_ps)).ok()?;
        let mut weight_vs = None;
        device.CreateVertexShader(&vs_weight_bc, None, Some(&mut weight_vs)).ok()?;
        let mut weight_ps = None;
        device.CreatePixelShader(&ps_weight_bc, None, Some(&mut weight_ps)).ok()?;
        let mut blend_vs = None;
        device.CreateVertexShader(&vs_blend_bc, None, Some(&mut blend_vs)).ok()?;
        let mut blend_ps = None;
        device.CreatePixelShader(&ps_blend_bc, None, Some(&mut blend_ps)).ok()?;

        info!("SMAA preset compiled");
        Some(SmaaShaders {
            quality,
            edge_vs: edge_vs?,
            edge_ps: edge_ps?,
            weight_vs: weight_vs?,
            weight_ps: weight_ps?,
            blend_vs: blend_vs?,
            blend_ps: blend_ps?,
        })
    }
}

fn create_static_srv(device: &ID3D11Device, width: u32, height: u32, format: DXGI_FORMAT, pitch: u32, bytes: &[u8]) -> Option<ID3D11ShaderResourceView> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: format,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_IMMUTABLE,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        ..Default::default()
    };
    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: bytes.as_ptr() as *const c_void,
        SysMemPitch: pitch,
        SysMemSlicePitch: 0,
    };
    let mut tex: Option<ID3D11Texture2D> = None;
    unsafe { device.CreateTexture2D(&desc, Some(&init), Some(&mut tex)) }.ok()?;
    let tex = tex?;
    let mut srv = None;
    unsafe { device.CreateShaderResourceView(&tex, None, Some(&mut srv)) }.ok()?;
    srv
}

impl SmaaPass {
    pub fn new(device: &ID3D11Device) -> Option<Self> {
        let compile = load_d3d_compile().or_else(|| {
            error!("SMAA: Failed to load d3dcompiler_47.dll");
            None
        })?;

        let area_srv = create_static_srv(device, 160, 560, DXGI_FORMAT_R8G8_UNORM, 160 * 2, AREA_TEX_BYTES)?;
        let search_srv = create_static_srv(device, 64, 16, DXGI_FORMAT_R8_UNORM, 64, SEARCH_TEX_BYTES)?;

        unsafe {
            let linear_desc = D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_LINEAR_MIP_POINT,
                AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                ComparisonFunc: D3D11_COMPARISON_ALWAYS,
                MaxLOD: f32::MAX,
                ..Default::default()
            };
            let mut linear_sampler = None;
            device.CreateSamplerState(&linear_desc, Some(&mut linear_sampler)).ok()?;

            let point_desc = D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_MIP_POINT,
                ..linear_desc
            };
            let mut point_sampler = None;
            device.CreateSamplerState(&point_desc, Some(&mut point_sampler)).ok()?;

            let cb_desc = windows::Win32::Graphics::Direct3D11::D3D11_BUFFER_DESC {
                ByteWidth: std::mem::size_of::<SmaaConstants>() as u32,
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                ..Default::default()
            };
            let mut cbuffer = None;
            device.CreateBuffer(&cb_desc, None, Some(&mut cbuffer)).ok()?;

            let mut blend_desc = D3D11_BLEND_DESC::default();
            blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: false.into(),
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
                ..Default::default()
            };
            let mut blend_state = None;
            device.CreateBlendState(&blend_desc, Some(&mut blend_state)).ok()?;

            let raster_desc = D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_NONE,
                DepthClipEnable: false.into(),
                ScissorEnable: false.into(),
                ..Default::default()
            };
            let mut raster_state = None;
            device.CreateRasterizerState(&raster_desc, Some(&mut raster_state)).ok()?;

            let depth_desc = D3D11_DEPTH_STENCIL_DESC {
                DepthEnable: false.into(),
                DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
                StencilEnable: false.into(),
                ..Default::default()
            };
            let mut depth_state = None;
            device.CreateDepthStencilState(&depth_desc, Some(&mut depth_state)).ok()?;

            Some(Self {
                compile,
                shaders: None,
                fxaa_vs: None,
                fxaa_ps: None,
                temporal_vs: None,
                temporal_ps: None,
                linear_sampler: linear_sampler?,
                point_sampler: point_sampler?,
                cbuffer: cbuffer?,
                blend_state: blend_state?,
                raster_state: raster_state?,
                depth_state: depth_state?,
                area_srv,
                search_srv,
                targets: None,
            })
        }
    }

    pub fn reset_targets(&mut self) {
        self.targets = None;
    }

    fn ensure_targets<'a>(
        targets: &'a mut Option<RenderTargets>,
        device: &ID3D11Device,
        width: u32,
        height: u32,
        format: DXGI_FORMAT,
    ) -> Option<&'a mut RenderTargets> {
        let needs_recreate = match targets {
            Some(t) => t.width != width || t.height != height || t.format != format,
            None => true,
        };
        if needs_recreate {
            *targets = None;
            let make_tex = |fmt: DXGI_FORMAT, bind: u32| -> Option<ID3D11Texture2D> {
                let desc = D3D11_TEXTURE2D_DESC {
                    Width: width,
                    Height: height,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: fmt,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: bind,
                    ..Default::default()
                };
                let mut tex = None;
                unsafe { device.CreateTexture2D(&desc, None, Some(&mut tex)) }.ok()?;
                tex
            };

            let color_copy = make_tex(format, D3D11_BIND_SHADER_RESOURCE.0 as u32)?;
            let mut color_srv = None;
            unsafe { device.CreateShaderResourceView(&color_copy, None, Some(&mut color_srv)) }.ok()?;

            let edges_tex = make_tex(DXGI_FORMAT_R8G8_UNORM, (D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE).0 as u32)?;
            let mut edges_rtv = None;
            unsafe { device.CreateRenderTargetView(&edges_tex, None, Some(&mut edges_rtv)) }.ok()?;
            let mut edges_srv = None;
            unsafe { device.CreateShaderResourceView(&edges_tex, None, Some(&mut edges_srv)) }.ok()?;

            let blend_tex = make_tex(DXGI_FORMAT_R8G8B8A8_UNORM, (D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE).0 as u32)?;
            let mut blend_rtv = None;
            unsafe { device.CreateRenderTargetView(&blend_tex, None, Some(&mut blend_rtv)) }.ok()?;
            let mut blend_srv = None;
            unsafe { device.CreateShaderResourceView(&blend_tex, None, Some(&mut blend_srv)) }.ok()?;

            let history_tex = make_tex(format, D3D11_BIND_SHADER_RESOURCE.0 as u32)?;
            let mut history_srv = None;
            unsafe { device.CreateShaderResourceView(&history_tex, None, Some(&mut history_srv)) }.ok()?;

            *targets = Some(RenderTargets {
                width,
                height,
                format,
                color_copy,
                color_srv: color_srv?,
                edges_rtv: edges_rtv?,
                edges_srv: edges_srv?,
                blend_rtv: blend_rtv?,
                blend_srv: blend_srv?,
                history_tex,
                history_srv: history_srv?,
                has_history: false,
            });
        }
        targets.as_mut()
    }

    pub fn apply(
        &mut self,
        device: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
        swap_chain: &IDXGISwapChain,
        backbuffer_rtv: &ID3D11RenderTargetView,
        quality: SmaaQuality,
        temporal: bool,
    ) {
        if quality == SmaaQuality::Off {
            if let Some(t) = &mut self.targets {
                t.has_history = false;
            }
            return;
        }
        if quality == SmaaQuality::Fxaa && (self.fxaa_vs.is_none() || self.fxaa_ps.is_none()) {
            if let (Ok(vs_bc), Ok(ps_bc)) = (
                compile_shader(self.compile, FXAA_HLSL, c"FxaaVS", c"vs_5_0").map_err(|e| error!("FXAA VS: {}", e)),
                compile_shader(self.compile, FXAA_HLSL, c"FxaaPS", c"ps_5_0").map_err(|e| error!("FXAA PS: {}", e)),
            ) {
                unsafe {
                    let mut vs = None;
                    let mut ps = None;
                    if device.CreateVertexShader(&vs_bc, None, Some(&mut vs)).is_ok()
                        && device.CreatePixelShader(&ps_bc, None, Some(&mut ps)).is_ok()
                    {
                        self.fxaa_vs = vs;
                        self.fxaa_ps = ps;
                        info!("FXAA 3.11 compiled");
                    }
                }
            }
        }
        if !matches!(quality, SmaaQuality::Off | SmaaQuality::Fxaa)
            && self.shaders.as_ref().map(|s| s.quality) != Some(quality)
        {
            self.shaders = compile_preset(self.compile, device, quality);
        }
        if temporal && (self.temporal_vs.is_none() || self.temporal_ps.is_none()) {
            let source = format!("{}#define SMAA_PRESET_MEDIUM\n{}{}", WRAPPER_PREAMBLE, OFFICIAL_SMAA_HLSL, WRAPPER_ENTRYPOINTS);
            if let (Ok(vs_bc), Ok(ps_bc)) = (
                compile_shader(self.compile, &source, c"TemporalVS", c"vs_5_0").map_err(|e| error!("SMAA TemporalVS: {}", e)),
                compile_shader(self.compile, &source, c"TemporalPS", c"ps_5_0").map_err(|e| error!("SMAA TemporalPS: {}", e)),
            ) {
                unsafe {
                    let mut vs = None;
                    let mut ps = None;
                    if device.CreateVertexShader(&vs_bc, None, Some(&mut vs)).is_ok()
                        && device.CreatePixelShader(&ps_bc, None, Some(&mut ps)).is_ok()
                    {
                        self.temporal_vs = vs;
                        self.temporal_ps = ps;
                    }
                }
            }
        }

        let Ok(backbuffer) = (unsafe { swap_chain.GetBuffer::<ID3D11Texture2D>(0) }) else {
            return;
        };
        let mut bb_desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { backbuffer.GetDesc(&mut bb_desc) };
        if bb_desc.Width == 0 || bb_desc.Height == 0 || bb_desc.SampleDesc.Count > 1 {
            return;
        }

        let Some(t) = Self::ensure_targets(&mut self.targets, device, bb_desc.Width, bb_desc.Height, bb_desc.Format) else {
            return;
        };

        unsafe {
            // Backup slots 1..=2 SRVs and slot 1 sampler (BackupState only saves slot 0)
            let mut prev_srvs: [Option<ID3D11ShaderResourceView>; 2] = [None, None];
            let mut prev_smp1: [Option<ID3D11SamplerState>; 1] = [None];
            ctx.PSGetShaderResources(1, Some(&mut prev_srvs));
            ctx.PSGetSamplers(1, Some(&mut prev_smp1));

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if ctx.Map(&self.cbuffer, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped)).is_ok() {
                let constants = SmaaConstants {
                    rt_metrics: [
                        1.0 / bb_desc.Width as f32,
                        1.0 / bb_desc.Height as f32,
                        bb_desc.Width as f32,
                        bb_desc.Height as f32,
                    ],
                };
                std::ptr::write(mapped.pData as *mut SmaaConstants, constants);
                ctx.Unmap(&self.cbuffer, 0);
            }

            let vp = [D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: bb_desc.Width as f32,
                Height: bb_desc.Height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }];
            ctx.RSSetViewports(Some(&vp));
            ctx.RSSetState(&self.raster_state);
            ctx.OMSetBlendState(&self.blend_state, None, 0xFFFF_FFFF);
            ctx.OMSetDepthStencilState(&self.depth_state, 0);
            ctx.IASetInputLayout(None);
            ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.PSSetSamplers(0, Some(&[Some(self.linear_sampler.clone()), Some(self.point_sampler.clone())]));
            ctx.VSSetConstantBuffers(0, Some(&[Some(self.cbuffer.clone())]));
            ctx.PSSetConstantBuffers(0, Some(&[Some(self.cbuffer.clone())]));

            if quality == SmaaQuality::Fxaa {
                if let (Some(f_vs), Some(f_ps)) = (&self.fxaa_vs, &self.fxaa_ps) {
                    ctx.CopyResource(&t.color_copy, &backbuffer);
                    ctx.OMSetRenderTargets(Some(&[Some(backbuffer_rtv.clone())]), None);
                    ctx.VSSetShader(f_vs, None);
                    ctx.PSSetShader(f_ps, None);
                    ctx.PSSetShaderResources(0, Some(&[Some(t.color_srv.clone()), None, None]));
                    ctx.Draw(3, 0);
                }
            } else if let Some(sh) = self.shaders.as_ref() {
                ctx.CopyResource(&t.color_copy, &backbuffer);

                // Pass 1: Color Edge Detection
                ctx.ClearRenderTargetView(&t.edges_rtv, &[0.0, 0.0, 0.0, 0.0]);
                ctx.OMSetRenderTargets(Some(&[Some(t.edges_rtv.clone())]), None);
                ctx.VSSetShader(&sh.edge_vs, None);
                ctx.PSSetShader(&sh.edge_ps, None);
                ctx.PSSetShaderResources(0, Some(&[Some(t.color_srv.clone())]));
                ctx.Draw(3, 0);

                // Pass 2: Blending Weight Calculation (with official AreaTex & SearchTex)
                ctx.ClearRenderTargetView(&t.blend_rtv, &[0.0, 0.0, 0.0, 0.0]);
                ctx.OMSetRenderTargets(Some(&[Some(t.blend_rtv.clone())]), None);
                ctx.VSSetShader(&sh.weight_vs, None);
                ctx.PSSetShader(&sh.weight_ps, None);
                ctx.PSSetShaderResources(0, Some(&[
                    Some(t.edges_srv.clone()),
                    Some(self.area_srv.clone()),
                    Some(self.search_srv.clone()),
                ]));
                ctx.Draw(3, 0);

                // Pass 3: Neighborhood Blending -> Backbuffer
                ctx.OMSetRenderTargets(Some(&[Some(backbuffer_rtv.clone())]), None);
                ctx.VSSetShader(&sh.blend_vs, None);
                ctx.PSSetShader(&sh.blend_ps, None);
                ctx.PSSetShaderResources(0, Some(&[
                    Some(t.color_srv.clone()),
                    Some(t.blend_srv.clone()),
                    None,
                ]));
                ctx.Draw(3, 0);
            }

            if temporal {
                if let (Some(t_vs), Some(t_ps)) = (&self.temporal_vs, &self.temporal_ps) {
                    if !t.has_history {
                        ctx.CopyResource(&t.history_tex, &backbuffer);
                        t.has_history = true;
                    } else {
                        ctx.CopyResource(&t.color_copy, &backbuffer);
                        ctx.OMSetRenderTargets(Some(&[Some(backbuffer_rtv.clone())]), None);
                        ctx.VSSetShader(t_vs, None);
                        ctx.PSSetShader(t_ps, None);
                        ctx.PSSetShaderResources(0, Some(&[
                            Some(t.color_srv.clone()),
                            Some(t.history_srv.clone()),
                            None,
                        ]));
                        ctx.Draw(3, 0);
                        ctx.CopyResource(&t.history_tex, &backbuffer);
                    }
                }
            } else {
                t.has_history = false;
            }

            // Restore slots 1..=2 SRVs and slot 1 sampler
            ctx.PSSetShaderResources(1, Some(&prev_srvs));
            ctx.PSSetSamplers(1, Some(&prev_smp1));
        }
    }
}
