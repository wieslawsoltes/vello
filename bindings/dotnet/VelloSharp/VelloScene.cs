using System;
using System.Numerics;

namespace VelloSharp;

public sealed class Scene : IDisposable
{
    private IntPtr _handle;

    public Scene()
    {
        _handle = NativeMethods.vello_scene_create();
        if (_handle == IntPtr.Zero)
        {
            throw new InvalidOperationException("Failed to create Vello scene.");
        }
    }

    public void Reset()
    {
        ThrowIfDisposed();
        NativeMethods.vello_scene_reset(_handle);
    }

    public void FillPath(PathBuilder path, FillRule fillRule, Matrix3x2 transform, RgbaColor color)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(path);

        var span = path.AsSpan();
        if (span.IsEmpty)
        {
            throw new ArgumentException("Path must contain at least one element.", nameof(path));
        }

        var nativeTransform = ToAffine(transform);
        var nativeColor = ToColor(color);

        unsafe
        {
            fixed (VelloPathElement* elementPtr = span)
            {
                var status = NativeMethods.vello_scene_fill_path(
                    _handle,
                    (VelloFillRule)fillRule,
                    nativeTransform,
                    nativeColor,
                    elementPtr,
                    (nuint)span.Length);

                NativeHelpers.ThrowOnError(status, "FillPath failed");
            }
        }
    }

    public void StrokePath(PathBuilder path, StrokeStyle style, Matrix3x2 transform, RgbaColor color)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(path);
        ArgumentNullException.ThrowIfNull(style);

        var span = path.AsSpan();
        if (span.IsEmpty)
        {
            throw new ArgumentException("Path must contain at least one element.", nameof(path));
        }

        var nativeTransform = ToAffine(transform);
        var nativeColor = ToColor(color);
        VelloStatus status;

        unsafe
        {
            fixed (VelloPathElement* elementPtr = span)
            {
                if (style.DashPattern is { Length: > 0 } pattern)
                {
                    fixed (double* dashPtr = pattern)
                    {
                        var nativeStyle = CreateStrokeStyle(style, (IntPtr)dashPtr, (nuint)pattern.Length);
                        status = NativeMethods.vello_scene_stroke_path(
                            _handle,
                            nativeStyle,
                            nativeTransform,
                            nativeColor,
                            elementPtr,
                            (nuint)span.Length);
                    }
                }
                else
                {
                    var nativeStyle = CreateStrokeStyle(style, IntPtr.Zero, 0);
                    status = NativeMethods.vello_scene_stroke_path(
                        _handle,
                        nativeStyle,
                        nativeTransform,
                        nativeColor,
                        elementPtr,
                        (nuint)span.Length);
                }
            }
        }

        NativeHelpers.ThrowOnError(status, "StrokePath failed");
    }

    internal IntPtr Handle
    {
        get
        {
            ThrowIfDisposed();
            return _handle;
        }
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_scene_destroy(_handle);
            _handle = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }
    }

    ~Scene()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_scene_destroy(_handle);
        }
    }

    private static VelloAffine ToAffine(Matrix3x2 m) => new()
    {
        M11 = m.M11,
        M12 = m.M12,
        M21 = m.M21,
        M22 = m.M22,
        Dx = m.M31,
        Dy = m.M32,
    };

    private static VelloColor ToColor(RgbaColor color) => new()
    {
        R = color.R,
        G = color.G,
        B = color.B,
        A = color.A,
    };

    private static VelloStrokeStyle CreateStrokeStyle(StrokeStyle style, IntPtr dashPtr, nuint dashLength) => new()
    {
        Width = style.Width,
        MiterLimit = style.MiterLimit,
        StartCap = (VelloLineCap)style.StartCap,
        EndCap = (VelloLineCap)style.EndCap,
        LineJoin = (VelloLineJoin)style.LineJoin,
        DashPhase = style.DashPhase,
        DashPattern = dashPtr,
        DashLength = dashLength,
    };

    private void ThrowIfDisposed()
    {
        if (_handle == IntPtr.Zero)
        {
            throw new ObjectDisposedException(nameof(Scene));
        }
    }
}
