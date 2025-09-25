using System;
using System.Runtime.InteropServices;

namespace VelloSharp;

internal static class NativeMethods
{
    private const string LibraryName = "vello_ffi";

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr vello_renderer_create(uint width, uint height);

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
    internal static extern IntPtr vello_last_error_message();
}
