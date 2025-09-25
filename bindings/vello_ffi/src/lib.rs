#![cfg_attr(not(test), deny(clippy::all))]
#![allow(clippy::missing_safety_doc)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]

use std::{
    cell::RefCell,
    ffi::{CString, c_char},
    num::NonZeroUsize,
    slice,
};

use futures_intrusive::channel::shared::oneshot_channel;
use kurbo::{Affine, BezPath, Cap, Join, Rect, Stroke};
use peniko::{
    BlendMode, Blob, Brush, BrushRef, Color, ColorStop, ColorStops, Extend, Fill, FontData,
    Gradient, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality,
};
use vello::{AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::{Buffer, Device, Queue};

#[cfg(feature = "trace-paths")]
macro_rules! trace_path {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

#[cfg(not(feature = "trace-paths"))]
macro_rules! trace_path {
    ($($arg:tt)*) => {};
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloStatus {
    Success = 0,
    NullPointer = 1,
    InvalidArgument = 2,
    DeviceCreationFailed = 3,
    RenderError = 4,
    MapFailed = 5,
    Unsupported = 6,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn clear_last_error() {
    LAST_ERROR.with(|slot| slot.borrow_mut().take());
}

fn set_last_error(msg: impl Into<String>) {
    let msg = msg.into();
    let cstr = CString::new(msg).unwrap_or_else(|_| CString::new("invalid error message").unwrap());
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(cstr));
}

unsafe fn slice_from_raw<'a, T>(ptr: *const T, len: usize) -> Result<&'a [T], VelloStatus> {
    if len == 0 {
        Ok(&[])
    } else if ptr.is_null() {
        Err(VelloStatus::NullPointer)
    } else {
        Ok(unsafe { slice::from_raw_parts(ptr, len) })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vello_last_error_message() -> *const c_char {
    LAST_ERROR.with(|slot| match slot.borrow().as_ref() {
        Some(cstr) => cstr.as_ptr(),
        None => std::ptr::null(),
    })
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloAffine {
    pub m11: f64,
    pub m12: f64,
    pub m21: f64,
    pub m22: f64,
    pub dx: f64,
    pub dy: f64,
}

impl From<VelloAffine> for Affine {
    fn from(value: VelloAffine) -> Self {
        Affine::new([
            value.m11, value.m12, value.m21, value.m22, value.dx, value.dy,
        ])
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<VelloColor> for Color {
    fn from(value: VelloColor) -> Self {
        Color::new([value.r, value.g, value.b, value.a])
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone)]
pub enum VelloFillRule {
    NonZero = 0,
    EvenOdd = 1,
}

impl From<VelloFillRule> for Fill {
    fn from(value: VelloFillRule) -> Self {
        match value {
            VelloFillRule::NonZero => Fill::NonZero,
            VelloFillRule::EvenOdd => Fill::EvenOdd,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone)]
pub enum VelloPathVerb {
    MoveTo = 0,
    LineTo = 1,
    QuadTo = 2,
    CubicTo = 3,
    Close = 4,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloPathElement {
    pub verb: VelloPathVerb,
    pub _padding: i32,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn build_bez_path(elements: &[VelloPathElement]) -> Result<BezPath, &'static str> {
    if elements.is_empty() {
        return Err("path is empty");
    }
    let mut path = BezPath::new();
    let mut has_move = false;
    for (idx, elem) in elements.iter().enumerate() {
        if idx == 0 {
            trace_path!(
                "native first verb raw={:?} ({}), bytes={:02X?}",
                elem.verb,
                elem.verb as i32,
                unsafe { &*(elem as *const VelloPathElement as *const [u8; 16]) }
            );
        }
        match elem.verb {
            VelloPathVerb::MoveTo => {
                has_move = true;
                path.move_to((elem.x0, elem.y0));
            }
            VelloPathVerb::LineTo => {
                if !has_move {
                    return Err("path must start with MoveTo");
                }
                path.line_to((elem.x0, elem.y0));
            }
            VelloPathVerb::QuadTo => {
                if !has_move {
                    return Err("path must start with MoveTo");
                }
                path.quad_to((elem.x0, elem.y0), (elem.x1, elem.y1));
            }
            VelloPathVerb::CubicTo => {
                if !has_move {
                    return Err("path must start with MoveTo");
                }
                path.curve_to((elem.x0, elem.y0), (elem.x1, elem.y1), (elem.x2, elem.y2));
            }
            VelloPathVerb::Close => {
                if idx == 0 {
                    return Err("path cannot begin with Close");
                }
                path.close_path();
            }
        }
    }
    if !has_move {
        return Err("path must contain a MoveTo");
    }
    Ok(path)
}

fn convert_gradient_stops(stops: &[VelloGradientStop]) -> Result<ColorStops, VelloStatus> {
    if stops.is_empty() {
        return Err(VelloStatus::InvalidArgument);
    }
    let mut converted = Vec::with_capacity(stops.len());
    for stop in stops {
        if !(0.0..=1.0).contains(&stop.offset) || !stop.offset.is_finite() {
            return Err(VelloStatus::InvalidArgument);
        }
        let color: Color = (*stop).color.into();
        converted.push(ColorStop::from((stop.offset, color)));
    }
    Ok(converted.as_slice().into())
}

fn gradient_from_linear(linear: &VelloLinearGradient) -> Result<Gradient, VelloStatus> {
    let stops = unsafe { slice_from_raw(linear.stops, linear.stop_count)? };
    let mut gradient = Gradient::new_linear(
        (linear.start.x, linear.start.y),
        (linear.end.x, linear.end.y),
    );
    gradient.extend = linear.extend.into();
    gradient.stops = convert_gradient_stops(stops)?;
    Ok(gradient)
}

fn gradient_from_radial(radial: &VelloRadialGradient) -> Result<Gradient, VelloStatus> {
    if radial.start_radius < 0.0 || radial.end_radius < 0.0 {
        return Err(VelloStatus::InvalidArgument);
    }
    let stops = unsafe { slice_from_raw(radial.stops, radial.stop_count)? };
    let mut gradient = Gradient::new_two_point_radial(
        (radial.start_center.x, radial.start_center.y),
        radial.start_radius,
        (radial.end_center.x, radial.end_center.y),
        radial.end_radius,
    );
    gradient.extend = radial.extend.into();
    gradient.stops = convert_gradient_stops(stops)?;
    Ok(gradient)
}

fn image_brush_from_params(params: &VelloImageBrushParams) -> Result<ImageBrush, VelloStatus> {
    let Some(handle) = (unsafe { params.image.as_ref() }) else {
        return Err(VelloStatus::NullPointer);
    };
    let mut brush = ImageBrush::new(handle.image.clone());
    brush.sampler.x_extend = params.x_extend.into();
    brush.sampler.y_extend = params.y_extend.into();
    brush.sampler.quality = params.quality.into();
    brush.sampler.alpha = params.alpha;
    Ok(brush)
}

fn brush_from_ffi(brush: &VelloBrush) -> Result<Brush, VelloStatus> {
    match brush.kind {
        VelloBrushKind::Solid => Ok(Brush::Solid(brush.solid.into())),
        VelloBrushKind::LinearGradient => Ok(Brush::Gradient(gradient_from_linear(&brush.linear)?)),
        VelloBrushKind::RadialGradient => Ok(Brush::Gradient(gradient_from_radial(&brush.radial)?)),
        VelloBrushKind::Image => Ok(Brush::Image(image_brush_from_params(&brush.image)?)),
    }
}

fn blend_mode_from_params(params: &VelloLayerParams) -> BlendMode {
    BlendMode::new(params.mix.into(), params.compose.into())
}

fn renderer_options_from_ffi(options: &VelloRendererOptions) -> RendererOptions {
    let mut support = AaSupport {
        area: options.support_area,
        msaa8: options.support_msaa8,
        msaa16: options.support_msaa16,
    };
    if !support.area && !support.msaa8 && !support.msaa16 {
        support = AaSupport::area_only();
    }
    let init_threads = if options.init_threads <= 0 {
        None
    } else {
        NonZeroUsize::new(options.init_threads as usize)
    };
    RendererOptions {
        use_cpu: options.use_cpu,
        antialiasing_support: support,
        num_init_threads: init_threads,
        pipeline_cache: None,
    }
}

fn optional_affine(ptr: *const VelloAffine) -> Result<Option<Affine>, VelloStatus> {
    if ptr.is_null() {
        Ok(None)
    } else {
        let affine = unsafe { (*ptr).into() };
        Ok(Some(affine))
    }
}

fn image_format_from_render(format: VelloRenderFormat) -> ImageFormat {
    match format {
        VelloRenderFormat::Rgba8 => ImageFormat::Rgba8,
        VelloRenderFormat::Bgra8 => ImageFormat::Bgra8,
    }
}

fn fill_path_with_brush(
    scene: &mut Scene,
    fill_rule: VelloFillRule,
    transform: VelloAffine,
    brush: &VelloBrush,
    brush_transform: *const VelloAffine,
    elements: &[VelloPathElement],
) -> Result<(), VelloStatus> {
    let path = build_bez_path(elements).map_err(|err| {
        set_last_error(err);
        VelloStatus::InvalidArgument
    })?;
    let brush = brush_from_ffi(brush)?;
    let brush_ref = BrushRef::from(&brush);
    let brush_transform = optional_affine(brush_transform)?;
    scene.fill(
        fill_rule.into(),
        transform.into(),
        brush_ref,
        brush_transform,
        &path,
    );
    Ok(())
}

fn stroke_path_with_brush(
    scene: &mut Scene,
    style: &VelloStrokeStyle,
    transform: VelloAffine,
    brush: &VelloBrush,
    brush_transform: *const VelloAffine,
    elements: &[VelloPathElement],
) -> Result<(), VelloStatus> {
    let path = build_bez_path(elements).map_err(|err| {
        set_last_error(err);
        VelloStatus::InvalidArgument
    })?;
    let stroke = style.to_stroke()?;
    let brush = brush_from_ffi(brush)?;
    let brush_ref = BrushRef::from(&brush);
    let brush_transform = optional_affine(brush_transform)?;
    scene.stroke(&stroke, transform.into(), brush_ref, brush_transform, &path);
    Ok(())
}

#[repr(i32)]
#[derive(Debug, Copy, Clone)]
pub enum VelloLineCap {
    Butt = 0,
    Round = 1,
    Square = 2,
}

impl From<VelloLineCap> for Cap {
    fn from(value: VelloLineCap) -> Self {
        match value {
            VelloLineCap::Butt => Cap::Butt,
            VelloLineCap::Round => Cap::Round,
            VelloLineCap::Square => Cap::Square,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone)]
pub enum VelloLineJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

impl From<VelloLineJoin> for Join {
    fn from(value: VelloLineJoin) -> Self {
        match value {
            VelloLineJoin::Miter => Join::Miter,
            VelloLineJoin::Round => Join::Round,
            VelloLineJoin::Bevel => Join::Bevel,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloStrokeStyle {
    pub width: f64,
    pub miter_limit: f64,
    pub start_cap: VelloLineCap,
    pub end_cap: VelloLineCap,
    pub line_join: VelloLineJoin,
    pub dash_phase: f64,
    pub dash_pattern: *const f64,
    pub dash_length: usize,
}

impl VelloStrokeStyle {
    fn to_stroke(&self) -> Result<Stroke, VelloStatus> {
        let mut stroke = Stroke::new(self.width)
            .with_start_cap(self.start_cap.into())
            .with_end_cap(self.end_cap.into())
            .with_join(self.line_join.into())
            .with_miter_limit(self.miter_limit);
        if self.dash_length > 0 {
            if self.dash_pattern.is_null() {
                return Err(VelloStatus::InvalidArgument);
            }
            let dashes = unsafe { slice::from_raw_parts(self.dash_pattern, self.dash_length) };
            stroke = stroke.with_dashes(self.dash_phase, dashes.to_vec());
        }
        Ok(stroke)
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone)]
pub enum VelloAaMode {
    Area = 0,
    Msaa8 = 1,
    Msaa16 = 2,
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloRenderFormat {
    Rgba8 = 0,
    Bgra8 = 1,
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloImageAlphaMode {
    Straight = 0,
    Premultiplied = 1,
}

impl From<VelloImageAlphaMode> for ImageAlphaType {
    fn from(value: VelloImageAlphaMode) -> Self {
        match value {
            VelloImageAlphaMode::Straight => ImageAlphaType::Alpha,
            VelloImageAlphaMode::Premultiplied => ImageAlphaType::AlphaPremultiplied,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloExtendMode {
    Pad = 0,
    Repeat = 1,
    Reflect = 2,
}

impl From<VelloExtendMode> for Extend {
    fn from(value: VelloExtendMode) -> Self {
        match value {
            VelloExtendMode::Pad => Extend::Pad,
            VelloExtendMode::Repeat => Extend::Repeat,
            VelloExtendMode::Reflect => Extend::Reflect,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloImageQualityMode {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl From<VelloImageQualityMode> for ImageQuality {
    fn from(value: VelloImageQualityMode) -> Self {
        match value {
            VelloImageQualityMode::Low => ImageQuality::Low,
            VelloImageQualityMode::Medium => ImageQuality::Medium,
            VelloImageQualityMode::High => ImageQuality::High,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloBrushKind {
    Solid = 0,
    LinearGradient = 1,
    RadialGradient = 2,
    Image = 3,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloGradientStop {
    pub offset: f32,
    pub color: VelloColor,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloLinearGradient {
    pub start: VelloPoint,
    pub end: VelloPoint,
    pub extend: VelloExtendMode,
    pub stops: *const VelloGradientStop,
    pub stop_count: usize,
}

impl Default for VelloLinearGradient {
    fn default() -> Self {
        Self {
            start: VelloPoint { x: 0.0, y: 0.0 },
            end: VelloPoint { x: 0.0, y: 0.0 },
            extend: VelloExtendMode::Pad,
            stops: std::ptr::null(),
            stop_count: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloRadialGradient {
    pub start_center: VelloPoint,
    pub start_radius: f32,
    pub end_center: VelloPoint,
    pub end_radius: f32,
    pub extend: VelloExtendMode,
    pub stops: *const VelloGradientStop,
    pub stop_count: usize,
}

impl Default for VelloRadialGradient {
    fn default() -> Self {
        Self {
            start_center: VelloPoint { x: 0.0, y: 0.0 },
            start_radius: 0.0,
            end_center: VelloPoint { x: 0.0, y: 0.0 },
            end_radius: 0.0,
            extend: VelloExtendMode::Pad,
            stops: std::ptr::null(),
            stop_count: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloImageBrushParams {
    pub image: *const VelloImageHandle,
    pub x_extend: VelloExtendMode,
    pub y_extend: VelloExtendMode,
    pub quality: VelloImageQualityMode,
    pub alpha: f32,
}

impl Default for VelloImageBrushParams {
    fn default() -> Self {
        Self {
            image: std::ptr::null(),
            x_extend: VelloExtendMode::Pad,
            y_extend: VelloExtendMode::Pad,
            quality: VelloImageQualityMode::Medium,
            alpha: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloBrush {
    pub kind: VelloBrushKind,
    pub solid: VelloColor,
    pub linear: VelloLinearGradient,
    pub radial: VelloRadialGradient,
    pub image: VelloImageBrushParams,
}

impl Default for VelloBrush {
    fn default() -> Self {
        Self {
            kind: VelloBrushKind::Solid,
            solid: VelloColor {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            linear: VelloLinearGradient::default(),
            radial: VelloRadialGradient::default(),
            image: VelloImageBrushParams::default(),
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloBlendMix {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
    Clip = 128,
}

impl From<VelloBlendMix> for peniko::Mix {
    fn from(value: VelloBlendMix) -> Self {
        match value {
            VelloBlendMix::Normal => peniko::Mix::Normal,
            VelloBlendMix::Multiply => peniko::Mix::Multiply,
            VelloBlendMix::Screen => peniko::Mix::Screen,
            VelloBlendMix::Overlay => peniko::Mix::Overlay,
            VelloBlendMix::Darken => peniko::Mix::Darken,
            VelloBlendMix::Lighten => peniko::Mix::Lighten,
            VelloBlendMix::ColorDodge => peniko::Mix::ColorDodge,
            VelloBlendMix::ColorBurn => peniko::Mix::ColorBurn,
            VelloBlendMix::HardLight => peniko::Mix::HardLight,
            VelloBlendMix::SoftLight => peniko::Mix::SoftLight,
            VelloBlendMix::Difference => peniko::Mix::Difference,
            VelloBlendMix::Exclusion => peniko::Mix::Exclusion,
            VelloBlendMix::Hue => peniko::Mix::Hue,
            VelloBlendMix::Saturation => peniko::Mix::Saturation,
            VelloBlendMix::Color => peniko::Mix::Color,
            VelloBlendMix::Luminosity => peniko::Mix::Luminosity,
            VelloBlendMix::Clip => peniko::Mix::Clip,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloBlendCompose {
    Clear = 0,
    Copy = 1,
    Dest = 2,
    SrcOver = 3,
    DestOver = 4,
    SrcIn = 5,
    DestIn = 6,
    SrcOut = 7,
    DestOut = 8,
    SrcAtop = 9,
    DestAtop = 10,
    Xor = 11,
    Plus = 12,
    PlusLighter = 13,
}

impl From<VelloBlendCompose> for peniko::Compose {
    fn from(value: VelloBlendCompose) -> Self {
        match value {
            VelloBlendCompose::Clear => peniko::Compose::Clear,
            VelloBlendCompose::Copy => peniko::Compose::Copy,
            VelloBlendCompose::Dest => peniko::Compose::Dest,
            VelloBlendCompose::SrcOver => peniko::Compose::SrcOver,
            VelloBlendCompose::DestOver => peniko::Compose::DestOver,
            VelloBlendCompose::SrcIn => peniko::Compose::SrcIn,
            VelloBlendCompose::DestIn => peniko::Compose::DestIn,
            VelloBlendCompose::SrcOut => peniko::Compose::SrcOut,
            VelloBlendCompose::DestOut => peniko::Compose::DestOut,
            VelloBlendCompose::SrcAtop => peniko::Compose::SrcAtop,
            VelloBlendCompose::DestAtop => peniko::Compose::DestAtop,
            VelloBlendCompose::Xor => peniko::Compose::Xor,
            VelloBlendCompose::Plus => peniko::Compose::Plus,
            VelloBlendCompose::PlusLighter => peniko::Compose::PlusLighter,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloLayerParams {
    pub mix: VelloBlendMix,
    pub compose: VelloBlendCompose,
    pub alpha: f32,
    pub transform: VelloAffine,
    pub clip_elements: *const VelloPathElement,
    pub clip_element_count: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloRendererOptions {
    pub use_cpu: bool,
    pub support_area: bool,
    pub support_msaa8: bool,
    pub support_msaa16: bool,
    pub init_threads: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloGlyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VelloGlyphRunStyle {
    Fill = 0,
    Stroke = 1,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloGlyphRunOptions {
    pub transform: VelloAffine,
    pub glyph_transform: *const VelloAffine,
    pub font_size: f32,
    pub hint: bool,
    pub style: VelloGlyphRunStyle,
    pub brush: VelloBrush,
    pub brush_alpha: f32,
    pub stroke_style: VelloStrokeStyle,
}

impl From<VelloAaMode> for AaConfig {
    fn from(value: VelloAaMode) -> Self {
        match value {
            VelloAaMode::Area => AaConfig::Area,
            VelloAaMode::Msaa8 => AaConfig::Msaa8,
            VelloAaMode::Msaa16 => AaConfig::Msaa16,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VelloRenderParams {
    pub width: u32,
    pub height: u32,
    pub base_color: VelloColor,
    pub antialiasing: VelloAaMode,
    pub format: VelloRenderFormat,
}

struct RenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: usize,
    unpadded_bytes_per_row: usize,
}

#[repr(C)]
pub struct VelloImageHandle {
    image: ImageData,
}

#[repr(C)]
pub struct VelloFontHandle {
    font: FontData,
}

impl RenderTarget {
    fn extent(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }
}

struct RendererContext {
    device: Device,
    queue: Queue,
    renderer: Renderer,
    target: RenderTarget,
}

fn align_to(value: usize, alignment: usize) -> Option<usize> {
    if alignment == 0 {
        return Some(value);
    }
    let remainder = value % alignment;
    if remainder == 0 {
        Some(value)
    } else {
        value.checked_add(alignment - remainder)
    }
}

impl RendererContext {
    fn new(width: u32, height: u32) -> Result<Self, VelloStatus> {
        Self::new_with_options(width, height, RendererOptions::default())
    }

    fn new_with_options(
        width: u32,
        height: u32,
        renderer_options: RendererOptions,
    ) -> Result<Self, VelloStatus> {
        if width == 0 || height == 0 {
            return Err(VelloStatus::InvalidArgument);
        }
        let backends = wgpu::Backends::from_env().unwrap_or_default();
        let flags = wgpu::InstanceFlags::from_build_config().with_env();
        let backend_options = wgpu::BackendOptions::from_env_or_default();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            flags,
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options,
        });
        let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &instance, None,
        ))
        .map_err(|_| VelloStatus::DeviceCreationFailed)?;
        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();
        if adapter_features.contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES) {
            required_features |= wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        }
        let required_limits = adapter.limits();
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("vello_ffi_device"),
            required_features,
            required_limits,
            memory_hints: wgpu::MemoryHints::default(),
            trace: Default::default(),
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))
            .map_err(|_| VelloStatus::DeviceCreationFailed)?;
        let renderer = Renderer::new(&device, renderer_options)
            .map_err(|_| VelloStatus::DeviceCreationFailed)?;
        let target = create_render_target(&device, width, height)?;
        Ok(Self {
            device,
            queue,
            renderer,
            target,
        })
    }

    fn ensure_target(&mut self, width: u32, height: u32) -> Result<(), VelloStatus> {
        if width == self.target.width && height == self.target.height {
            return Ok(());
        }
        self.target = create_render_target(&self.device, width, height)?;
        Ok(())
    }

    fn render_into(
        &mut self,
        scene: &Scene,
        params: &VelloRenderParams,
        out_ptr: *mut u8,
        out_stride: usize,
        out_size: usize,
    ) -> Result<(), VelloStatus> {
        if out_ptr.is_null() {
            return Err(VelloStatus::NullPointer);
        }
        if params.width == 0 || params.height == 0 {
            return Err(VelloStatus::InvalidArgument);
        }
        self.ensure_target(params.width, params.height)?;
        if out_stride < self.target.unpadded_bytes_per_row {
            return Err(VelloStatus::InvalidArgument);
        }
        let required_size = out_stride
            .checked_mul(params.height as usize)
            .ok_or(VelloStatus::InvalidArgument)?;
        if out_size < required_size {
            return Err(VelloStatus::InvalidArgument);
        }

        if self.target.padded_bytes_per_row > u32::MAX as usize {
            return Err(VelloStatus::Unsupported);
        }

        let render_params = RenderParams {
            base_color: params.base_color.into(),
            width: params.width,
            height: params.height,
            antialiasing_method: params.antialiasing.into(),
        };

        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                scene,
                &self.target.view,
                &render_params,
            )
            .map_err(|_| VelloStatus::RenderError)?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vello_ffi_copy_encoder"),
            });
        let bytes_per_row = self.target.padded_bytes_per_row as u32;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.target.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(params.height),
                },
            },
            self.target.extent(),
        );
        self.queue.submit(Some(encoder.finish()));
        let buffer_slice = self.target.readback.slice(..);
        let (sender, receiver) = oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });
        self.device
            .poll(wgpu::PollType::Wait)
            .map_err(|_| VelloStatus::MapFailed)?;
        match pollster::block_on(receiver.receive()) {
            Some(Ok(())) => {}
            _ => return Err(VelloStatus::MapFailed),
        }
        let mapped = buffer_slice.get_mapped_range();
        let output = unsafe { slice::from_raw_parts_mut(out_ptr, out_size) };
        let row_size = self.target.unpadded_bytes_per_row;
        if row_size % 4 != 0 {
            self.target.readback.unmap();
            return Err(VelloStatus::Unsupported);
        }
        let padded = self.target.padded_bytes_per_row;
        for y in 0..params.height as usize {
            let src_offset = y * padded;
            let dst_offset = y * out_stride;
            let src = &mapped[src_offset..src_offset + row_size];
            let dst = &mut output[dst_offset..dst_offset + row_size];
            match params.format {
                VelloRenderFormat::Rgba8 => {
                    dst.copy_from_slice(src);
                }
                VelloRenderFormat::Bgra8 => {
                    for (rgba, bgra) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
                        bgra[0] = rgba[2];
                        bgra[1] = rgba[1];
                        bgra[2] = rgba[0];
                        bgra[3] = rgba[3];
                    }
                }
            }
        }
        drop(mapped);
        self.target.readback.unmap();
        Ok(())
    }
}

fn create_render_target(
    device: &Device,
    width: u32,
    height: u32,
) -> Result<RenderTarget, VelloStatus> {
    if width == 0 || height == 0 {
        return Err(VelloStatus::InvalidArgument);
    }
    let unpadded = (width as usize)
        .checked_mul(4)
        .ok_or(VelloStatus::InvalidArgument)?;
    let padded = align_to(unpadded, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize)
        .ok_or(VelloStatus::InvalidArgument)?;
    if padded > u32::MAX as usize {
        return Err(VelloStatus::Unsupported);
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vello_ffi_target_texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("vello_ffi_target_view"),
        ..Default::default()
    });
    let buffer_size = padded
        .checked_mul(height as usize)
        .ok_or(VelloStatus::InvalidArgument)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vello_ffi_readback"),
        size: buffer_size as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    Ok(RenderTarget {
        texture,
        view,
        readback,
        width,
        height,
        padded_bytes_per_row: padded,
        unpadded_bytes_per_row: unpadded,
    })
}

#[repr(C)]
pub struct VelloRendererHandle {
    inner: RendererContext,
}

#[repr(C)]
pub struct VelloSceneHandle {
    inner: Scene,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_renderer_create(
    width: u32,
    height: u32,
) -> *mut VelloRendererHandle {
    clear_last_error();
    match RendererContext::new(width, height) {
        Ok(inner) => Box::into_raw(Box::new(VelloRendererHandle { inner })),
        Err(status) => {
            set_last_error(format!("Failed to create renderer: {:?}", status));
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_renderer_create_with_options(
    width: u32,
    height: u32,
    options: VelloRendererOptions,
) -> *mut VelloRendererHandle {
    clear_last_error();
    let renderer_options = renderer_options_from_ffi(&options);
    match RendererContext::new_with_options(width, height, renderer_options) {
        Ok(inner) => Box::into_raw(Box::new(VelloRendererHandle { inner })),
        Err(status) => {
            set_last_error(format!("Failed to create renderer: {:?}", status));
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_renderer_destroy(renderer: *mut VelloRendererHandle) {
    if !renderer.is_null() {
        unsafe {
            drop(Box::from_raw(renderer));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_renderer_render(
    renderer: *mut VelloRendererHandle,
    scene: *const VelloSceneHandle,
    params: VelloRenderParams,
    buffer: *mut u8,
    stride: usize,
    buffer_size: usize,
) -> VelloStatus {
    clear_last_error();
    let Some(renderer) = (unsafe { renderer.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    let Some(scene) = (unsafe { scene.as_ref() }) else {
        return VelloStatus::NullPointer;
    };
    match renderer
        .inner
        .render_into(&scene.inner, &params, buffer, stride, buffer_size)
    {
        Ok(()) => VelloStatus::Success,
        Err(status) => {
            set_last_error(format!("Render failed: {:?}", status));
            status
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_renderer_resize(
    renderer: *mut VelloRendererHandle,
    width: u32,
    height: u32,
) -> VelloStatus {
    clear_last_error();
    let Some(renderer) = (unsafe { renderer.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    match renderer.inner.ensure_target(width, height) {
        Ok(()) => VelloStatus::Success,
        Err(status) => {
            set_last_error(format!("Resize failed: {:?}", status));
            status
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vello_scene_create() -> *mut VelloSceneHandle {
    clear_last_error();
    Box::into_raw(Box::new(VelloSceneHandle {
        inner: Scene::new(),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_destroy(scene: *mut VelloSceneHandle) {
    if !scene.is_null() {
        unsafe {
            drop(Box::from_raw(scene));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_reset(scene: *mut VelloSceneHandle) {
    if let Some(scene) = unsafe { scene.as_mut() } {
        scene.inner.reset();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_fill_path(
    scene: *mut VelloSceneHandle,
    fill_rule: VelloFillRule,
    transform: VelloAffine,
    color: VelloColor,
    elements: *const VelloPathElement,
    element_count: usize,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    if elements.is_null() {
        return VelloStatus::NullPointer;
    }
    let slice = match unsafe { slice_from_raw(elements, element_count) } {
        Ok(slice) => slice,
        Err(status) => return status,
    };
    let mut brush = VelloBrush::default();
    brush.kind = VelloBrushKind::Solid;
    brush.solid = color;
    match fill_path_with_brush(
        &mut scene.inner,
        fill_rule,
        transform,
        &brush,
        std::ptr::null(),
        slice,
    ) {
        Ok(()) => VelloStatus::Success,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_stroke_path(
    scene: *mut VelloSceneHandle,
    style: VelloStrokeStyle,
    transform: VelloAffine,
    color: VelloColor,
    elements: *const VelloPathElement,
    element_count: usize,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    if elements.is_null() {
        return VelloStatus::NullPointer;
    }
    let slice = match unsafe { slice_from_raw(elements, element_count) } {
        Ok(slice) => slice,
        Err(status) => return status,
    };
    let mut brush = VelloBrush::default();
    brush.kind = VelloBrushKind::Solid;
    brush.solid = color;
    match stroke_path_with_brush(
        &mut scene.inner,
        &style,
        transform,
        &brush,
        std::ptr::null(),
        slice,
    ) {
        Ok(()) => VelloStatus::Success,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_fill_path_brush(
    scene: *mut VelloSceneHandle,
    fill_rule: VelloFillRule,
    transform: VelloAffine,
    brush: VelloBrush,
    brush_transform: *const VelloAffine,
    elements: *const VelloPathElement,
    element_count: usize,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    let slice = match unsafe { slice_from_raw(elements, element_count) } {
        Ok(slice) => slice,
        Err(status) => return status,
    };
    match fill_path_with_brush(
        &mut scene.inner,
        fill_rule,
        transform,
        &brush,
        brush_transform,
        slice,
    ) {
        Ok(()) => VelloStatus::Success,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_stroke_path_brush(
    scene: *mut VelloSceneHandle,
    style: VelloStrokeStyle,
    transform: VelloAffine,
    brush: VelloBrush,
    brush_transform: *const VelloAffine,
    elements: *const VelloPathElement,
    element_count: usize,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    let slice = match unsafe { slice_from_raw(elements, element_count) } {
        Ok(slice) => slice,
        Err(status) => return status,
    };
    match stroke_path_with_brush(
        &mut scene.inner,
        &style,
        transform,
        &brush,
        brush_transform,
        slice,
    ) {
        Ok(()) => VelloStatus::Success,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_push_layer(
    scene: *mut VelloSceneHandle,
    params: VelloLayerParams,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    let clip = match unsafe { slice_from_raw(params.clip_elements, params.clip_element_count) } {
        Ok(slice) => slice,
        Err(status) => return status,
    };
    let path = match build_bez_path(clip) {
        Ok(path) => path,
        Err(err) => {
            set_last_error(err);
            return VelloStatus::InvalidArgument;
        }
    };
    let blend_mode = blend_mode_from_params(&params);
    scene
        .inner
        .push_layer(blend_mode, params.alpha, params.transform.into(), &path);
    VelloStatus::Success
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_push_luminance_mask_layer(
    scene: *mut VelloSceneHandle,
    alpha: f32,
    transform: VelloAffine,
    elements: *const VelloPathElement,
    element_count: usize,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    let clip = match unsafe { slice_from_raw(elements, element_count) } {
        Ok(slice) => slice,
        Err(status) => return status,
    };
    let path = match build_bez_path(clip) {
        Ok(path) => path,
        Err(err) => {
            set_last_error(err);
            return VelloStatus::InvalidArgument;
        }
    };
    scene
        .inner
        .push_luminance_mask_layer(alpha, transform.into(), &path);
    VelloStatus::Success
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_pop_layer(scene: *mut VelloSceneHandle) {
    if let Some(scene) = unsafe { scene.as_mut() } {
        scene.inner.pop_layer();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_draw_blurred_rounded_rect(
    scene: *mut VelloSceneHandle,
    transform: VelloAffine,
    rect: VelloRect,
    color: VelloColor,
    radius: f64,
    std_dev: f64,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    if !(rect.width.is_finite() && rect.height.is_finite()) || rect.width < 0.0 || rect.height < 0.0
    {
        return VelloStatus::InvalidArgument;
    }
    let brush: Color = color.into();
    scene.inner.draw_blurred_rounded_rect(
        transform.into(),
        Rect::new(rect.x, rect.y, rect.x + rect.width, rect.y + rect.height),
        brush,
        radius,
        std_dev,
    );
    VelloStatus::Success
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_image_create(
    format: VelloRenderFormat,
    alpha: VelloImageAlphaMode,
    width: u32,
    height: u32,
    pixels: *const u8,
    stride: usize,
) -> *mut VelloImageHandle {
    clear_last_error();
    if width == 0 || height == 0 {
        set_last_error("Image dimensions must be non-zero");
        return std::ptr::null_mut();
    }
    if pixels.is_null() {
        set_last_error("Pixel data pointer is null");
        return std::ptr::null_mut();
    }
    let bytes_per_row = match (width as usize).checked_mul(4) {
        Some(value) => value,
        None => {
            set_last_error("Image dimensions overflow");
            return std::ptr::null_mut();
        }
    };
    let stride = if stride == 0 { bytes_per_row } else { stride };
    if stride < bytes_per_row {
        set_last_error("Stride is smaller than row size");
        return std::ptr::null_mut();
    }
    let total_size = match stride.checked_mul(height as usize) {
        Some(value) => value,
        None => {
            set_last_error("Image size overflow");
            return std::ptr::null_mut();
        }
    };
    let src = unsafe { slice::from_raw_parts(pixels, total_size) };
    let mut buffer = vec![0u8; bytes_per_row * height as usize];
    for y in 0..height as usize {
        let src_offset = y * stride;
        let dst_offset = y * bytes_per_row;
        buffer[dst_offset..dst_offset + bytes_per_row]
            .copy_from_slice(&src[src_offset..src_offset + bytes_per_row]);
    }
    let blob = Blob::from(buffer);
    let image = ImageData {
        data: blob,
        format: image_format_from_render(format),
        alpha_type: alpha.into(),
        width,
        height,
    };
    Box::into_raw(Box::new(VelloImageHandle { image }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_image_destroy(image: *mut VelloImageHandle) {
    if !image.is_null() {
        unsafe { drop(Box::from_raw(image)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_draw_image(
    scene: *mut VelloSceneHandle,
    brush: VelloImageBrushParams,
    transform: VelloAffine,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    let brush = match image_brush_from_params(&brush) {
        Ok(brush) => brush,
        Err(status) => return status,
    };
    scene.inner.draw_image(brush.as_ref(), transform.into());
    VelloStatus::Success
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_font_create(
    data: *const u8,
    length: usize,
    index: u32,
) -> *mut VelloFontHandle {
    clear_last_error();
    if length == 0 {
        set_last_error("Font data length must be non-zero");
        return std::ptr::null_mut();
    }
    if data.is_null() {
        set_last_error("Font data pointer is null");
        return std::ptr::null_mut();
    }
    let slice = unsafe { slice::from_raw_parts(data, length) };
    let blob = Blob::from(slice.to_vec());
    let font = FontData::new(blob, index);
    Box::into_raw(Box::new(VelloFontHandle { font }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_font_destroy(font: *mut VelloFontHandle) {
    if !font.is_null() {
        unsafe { drop(Box::from_raw(font)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vello_scene_draw_glyph_run(
    scene: *mut VelloSceneHandle,
    font: *const VelloFontHandle,
    glyphs: *const VelloGlyph,
    glyph_count: usize,
    options: VelloGlyphRunOptions,
) -> VelloStatus {
    clear_last_error();
    let Some(scene) = (unsafe { scene.as_mut() }) else {
        return VelloStatus::NullPointer;
    };
    let Some(font) = (unsafe { font.as_ref() }) else {
        return VelloStatus::NullPointer;
    };
    let glyphs = match unsafe { slice_from_raw(glyphs, glyph_count) } {
        Ok(slice) => slice,
        Err(status) => return status,
    };
    let brush = match brush_from_ffi(&options.brush) {
        Ok(brush) => brush,
        Err(status) => return status,
    };
    let brush_ref = BrushRef::from(&brush);
    let glyph_transform = match optional_affine(options.glyph_transform) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let mut builder = scene
        .inner
        .draw_glyphs(&font.font)
        .transform(options.transform.into())
        .font_size(options.font_size)
        .hint(options.hint)
        .brush(brush_ref)
        .brush_alpha(options.brush_alpha);
    builder = builder.glyph_transform(glyph_transform);

    let glyph_iter = glyphs.iter().copied().map(|glyph| Glyph {
        id: glyph.id,
        x: glyph.x,
        y: glyph.y,
    });

    match options.style {
        VelloGlyphRunStyle::Fill => builder.draw(Fill::NonZero, glyph_iter),
        VelloGlyphRunStyle::Stroke => match options.stroke_style.to_stroke() {
            Ok(stroke) => builder.draw(&stroke, glyph_iter),
            Err(status) => return status,
        },
    }

    VelloStatus::Success
}
