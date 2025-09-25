#![cfg_attr(not(test), deny(clippy::all))]
#![allow(clippy::missing_safety_doc)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]

use std::{
    cell::RefCell,
    ffi::{CString, c_char},
    slice,
};

use futures_intrusive::channel::shared::oneshot_channel;
use kurbo::{Affine, BezPath, Cap, Join, Stroke};
use peniko::{Color, Fill};
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};
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
        let renderer = Renderer::new(&device, RendererOptions::default())
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
    let slice = unsafe { slice::from_raw_parts(elements, element_count) };
    match build_bez_path(slice) {
        Ok(path) => {
            let brush: Color = color.into();
            scene
                .inner
                .fill(fill_rule.into(), transform.into(), brush, None, &path);
            VelloStatus::Success
        }
        Err(err) => {
            set_last_error(err);
            VelloStatus::InvalidArgument
        }
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
    let slice = unsafe { slice::from_raw_parts(elements, element_count) };
    let stroke = match style.to_stroke() {
        Ok(stroke) => stroke,
        Err(status) => {
            set_last_error("Invalid stroke style");
            return status;
        }
    };
    match build_bez_path(slice) {
        Ok(path) => {
            let brush: Color = color.into();
            scene
                .inner
                .stroke(&stroke, transform.into(), brush, None, &path);
            VelloStatus::Success
        }
        Err(err) => {
            set_last_error(err);
            VelloStatus::InvalidArgument
        }
    }
}
