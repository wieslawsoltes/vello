using System;
using System.Runtime.InteropServices;

namespace VelloSharp;

public sealed class Renderer : IDisposable
{
    private IntPtr _handle;

    public Renderer(uint width, uint height, RendererOptions? options = null)
    {
        _handle = options.HasValue
            ? NativeMethods.vello_renderer_create_with_options(width, height, options.Value.ToNative())
            : NativeMethods.vello_renderer_create(width, height);
        if (_handle == IntPtr.Zero)
        {
            throw new InvalidOperationException(NativeHelpers.GetLastErrorMessage() ?? "Failed to create Vello renderer.");
        }
    }

    public void Resize(uint width, uint height)
    {
        ThrowIfDisposed();
        var status = NativeMethods.vello_renderer_resize(_handle, width, height);
        NativeHelpers.ThrowOnError(status, "Renderer resize failed");
    }

    public void Render(Scene scene, RenderParams renderParams, Span<byte> destination, int strideBytes)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(scene);
        if (strideBytes <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(strideBytes), "Stride must be positive.");
        }
        if (destination.IsEmpty)
        {
            throw new ArgumentException("Destination buffer must not be empty.", nameof(destination));
        }
        var minimumStride = checked((int)(renderParams.Width * 4u));
        if (strideBytes < minimumStride)
        {
            throw new ArgumentOutOfRangeException(nameof(strideBytes), $"Stride must be at least {minimumStride} bytes for the requested width.");
        }
        var requiredSize = checked((long)strideBytes * renderParams.Height);
        if (destination.Length < requiredSize)
        {
            throw new ArgumentException($"Destination buffer must contain at least {requiredSize} bytes.", nameof(destination));
        }

        var nativeParams = new VelloRenderParams
        {
            Width = renderParams.Width,
            Height = renderParams.Height,
            BaseColor = renderParams.BaseColor.ToNative(),
            Antialiasing = (VelloAaMode)renderParams.Antialiasing,
            Format = (VelloRenderFormat)renderParams.Format,
        };

        var bufferSize = (nuint)destination.Length;
        var stride = (nuint)strideBytes;

        VelloStatus status;
        unsafe
        {
            fixed (byte* ptr = destination)
            {
                status = NativeMethods.vello_renderer_render(
                    _handle,
                    scene.Handle,
                    nativeParams,
                    (IntPtr)ptr,
                    stride,
                    bufferSize);
            }
        }

        NativeHelpers.ThrowOnError(status, "Renderer render failed");
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_renderer_destroy(_handle);
            _handle = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }
    }

    ~Renderer()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_renderer_destroy(_handle);
        }
    }

    private void ThrowIfDisposed()
    {
        if (_handle == IntPtr.Zero)
        {
            throw new ObjectDisposedException(nameof(Renderer));
        }
    }
}
