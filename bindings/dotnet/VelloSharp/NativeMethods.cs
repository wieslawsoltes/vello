using System;
using System.Runtime.InteropServices;

namespace VelloSharp;

internal static class NativeMethods
{
    internal const string LibraryName = "vello_ffi";

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr vello_renderer_create(uint width, uint height);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr vello_renderer_create_with_options(
        uint width,
        uint height,
        in VelloRendererOptions options);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void vello_renderer_destroy(IntPtr renderer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern VelloStatus vello_renderer_resize(IntPtr renderer, uint width, uint height);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern VelloStatus vello_renderer_render(
        IntPtr renderer,
        IntPtr scene,
        VelloRenderParams parameters,
        IntPtr buffer,
        nuint stride,
        nuint bufferSize);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr vello_scene_create();

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void vello_scene_destroy(IntPtr scene);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void vello_scene_reset(IntPtr scene);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static unsafe extern VelloStatus vello_scene_fill_path(
        IntPtr scene,
        VelloFillRule fill,
        VelloAffine transform,
        VelloColor color,
        VelloPathElement* elements,
        nuint elementCount);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static unsafe extern VelloStatus vello_scene_stroke_path(
        IntPtr scene,
        VelloStrokeStyle style,
        VelloAffine transform,
        VelloColor color,
        VelloPathElement* elements,
        nuint elementCount);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static unsafe extern VelloStatus vello_scene_fill_path_brush(
        IntPtr scene,
        VelloFillRule fill,
        VelloAffine transform,
        in VelloBrush brush,
        VelloAffine* brushTransform,
        VelloPathElement* elements,
        nuint elementCount);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static unsafe extern VelloStatus vello_scene_stroke_path_brush(
        IntPtr scene,
        in VelloStrokeStyle style,
        VelloAffine transform,
        in VelloBrush brush,
        VelloAffine* brushTransform,
        VelloPathElement* elements,
        nuint elementCount);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static unsafe extern VelloStatus vello_scene_push_layer(
        IntPtr scene,
        in VelloLayerParams layer);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static unsafe extern VelloStatus vello_scene_push_luminance_mask_layer(
        IntPtr scene,
        float alpha,
        VelloAffine transform,
        VelloPathElement* elements,
        nuint elementCount);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void vello_scene_pop_layer(IntPtr scene);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern VelloStatus vello_scene_draw_blurred_rounded_rect(
        IntPtr scene,
        VelloAffine transform,
        VelloRect rect,
        VelloColor color,
        double radius,
        double stdDev);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr vello_image_create(
        VelloRenderFormat format,
        VelloImageAlphaMode alpha,
        uint width,
        uint height,
        IntPtr pixels,
        nuint stride);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void vello_image_destroy(IntPtr image);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern VelloStatus vello_scene_draw_image(
        IntPtr scene,
        in VelloImageBrushParams brush,
        VelloAffine transform);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr vello_font_create(
        IntPtr data,
        nuint length,
        uint index);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void vello_font_destroy(IntPtr font);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static unsafe extern VelloStatus vello_scene_draw_glyph_run(
        IntPtr scene,
        IntPtr font,
        VelloGlyph* glyphs,
        nuint glyphCount,
        in VelloGlyphRunOptions options);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr vello_last_error_message();
}
