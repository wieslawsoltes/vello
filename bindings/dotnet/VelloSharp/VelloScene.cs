using System;
using System.Numerics;
using System.Runtime.InteropServices;

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

    public void FillPath(PathBuilder path, FillRule fillRule, Matrix3x2 transform, RgbaColor color) =>
        FillPath(path, fillRule, transform, new SolidColorBrush(color));

    public void FillPath(
        PathBuilder path,
        FillRule fillRule,
        Matrix3x2 transform,
        Brush brush,
        Matrix3x2? brushTransform = null)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(path);
        ArgumentNullException.ThrowIfNull(brush);

        var span = path.AsSpan();
        if (span.IsEmpty)
        {
            throw new ArgumentException("Path must contain at least one element.", nameof(path));
        }

        using var marshaler = brush.CreateMarshaler();

        unsafe
        {
            fixed (VelloPathElement* elementPtr = span)
            {
                VelloAffine* brushTransformPtr = null;
                VelloAffine brushAffine = default;
                if (brushTransform.HasValue)
                {
                    brushAffine = brushTransform.Value.ToNativeAffine();
                    brushTransformPtr = &brushAffine;
                }

                var status = NativeMethods.vello_scene_fill_path_brush(
                    _handle,
                    (VelloFillRule)fillRule,
                    transform.ToNativeAffine(),
                    marshaler.Brush,
                    brushTransformPtr,
                    elementPtr,
                    (nuint)span.Length);

                NativeHelpers.ThrowOnError(status, "FillPath failed");
            }
        }
    }

    public void StrokePath(PathBuilder path, StrokeStyle style, Matrix3x2 transform, RgbaColor color) =>
        StrokePath(path, style, transform, new SolidColorBrush(color));

    public void StrokePath(
        PathBuilder path,
        StrokeStyle style,
        Matrix3x2 transform,
        Brush brush,
        Matrix3x2? brushTransform = null)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(path);
        ArgumentNullException.ThrowIfNull(style);
        ArgumentNullException.ThrowIfNull(brush);

        var span = path.AsSpan();
        if (span.IsEmpty)
        {
            throw new ArgumentException("Path must contain at least one element.", nameof(path));
        }

        using var marshaler = brush.CreateMarshaler();

        unsafe
        {
            fixed (VelloPathElement* elementPtr = span)
            {
                VelloAffine* brushTransformPtr = null;
                VelloAffine brushAffine = default;
                if (brushTransform.HasValue)
                {
                    brushAffine = brushTransform.Value.ToNativeAffine();
                    brushTransformPtr = &brushAffine;
                }

                if (style.DashPattern is { Length: > 0 } pattern)
                {
                    fixed (double* dashPtr = pattern)
                    {
                        var nativeStyle = CreateStrokeStyle(style, (IntPtr)dashPtr, (nuint)pattern.Length);
                        var status = NativeMethods.vello_scene_stroke_path_brush(
                            _handle,
                            nativeStyle,
                            transform.ToNativeAffine(),
                            marshaler.Brush,
                            brushTransformPtr,
                            elementPtr,
                            (nuint)span.Length);

                        NativeHelpers.ThrowOnError(status, "StrokePath failed");
                    }
                }
                else
                {
                    var nativeStyle = CreateStrokeStyle(style, IntPtr.Zero, 0);
                    var status = NativeMethods.vello_scene_stroke_path_brush(
                        _handle,
                        nativeStyle,
                        transform.ToNativeAffine(),
                        marshaler.Brush,
                        brushTransformPtr,
                        elementPtr,
                        (nuint)span.Length);

                    NativeHelpers.ThrowOnError(status, "StrokePath failed");
                }
            }
        }
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

    public void PushLayer(PathBuilder clip, LayerBlend blend, Matrix3x2 transform, float alpha = 1f)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(clip);

        var span = clip.AsSpan();
        if (span.IsEmpty)
        {
            throw new ArgumentException("Clip path must contain at least one element.", nameof(clip));
        }

        unsafe
        {
            fixed (VelloPathElement* elementPtr = span)
            {
                var native = new VelloLayerParams
                {
                    Mix = (VelloBlendMix)blend.Mix,
                    Compose = (VelloBlendCompose)blend.Compose,
                    Alpha = alpha,
                    Transform = transform.ToNativeAffine(),
                    ClipElements = (IntPtr)elementPtr,
                    ClipElementCount = (nuint)span.Length,
                };

                var status = NativeMethods.vello_scene_push_layer(_handle, in native);
                NativeHelpers.ThrowOnError(status, "PushLayer failed");
            }
        }
    }

    public void PushLuminanceMaskLayer(PathBuilder clip, Matrix3x2 transform, float alpha = 1f)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(clip);

        var span = clip.AsSpan();
        if (span.IsEmpty)
        {
            throw new ArgumentException("Clip path must contain at least one element.", nameof(clip));
        }

        unsafe
        {
            fixed (VelloPathElement* elementPtr = span)
            {
                var status = NativeMethods.vello_scene_push_luminance_mask_layer(
                    _handle,
                    alpha,
                    transform.ToNativeAffine(),
                    elementPtr,
                    (nuint)span.Length);
                NativeHelpers.ThrowOnError(status, "PushLuminanceMaskLayer failed");
            }
        }
    }

    public void PopLayer()
    {
        ThrowIfDisposed();
        NativeMethods.vello_scene_pop_layer(_handle);
    }

    public void DrawBlurredRoundedRect(Vector2 origin, Vector2 size, Matrix3x2 transform, RgbaColor color, double radius, double stdDev)
    {
        ThrowIfDisposed();
        if (size.X < 0 || size.Y < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(size), "Size components must be non-negative.");
        }

        var rect = new VelloRect
        {
            X = origin.X,
            Y = origin.Y,
            Width = size.X,
            Height = size.Y,
        };

        var status = NativeMethods.vello_scene_draw_blurred_rounded_rect(
            _handle,
            transform.ToNativeAffine(),
            rect,
            color.ToNative(),
            radius,
            stdDev);

        NativeHelpers.ThrowOnError(status, "DrawBlurredRoundedRect failed");
    }

    public void DrawImage(Image image, Matrix3x2 transform) => DrawImage(new ImageBrush(image), transform);

    public void DrawImage(ImageBrush brush, Matrix3x2 transform)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(brush);

        var status = NativeMethods.vello_scene_draw_image(
            _handle,
            brush.ToNative(),
            transform.ToNativeAffine());
        NativeHelpers.ThrowOnError(status, "DrawImage failed");
    }

    public void DrawGlyphRun(Font font, ReadOnlySpan<Glyph> glyphs, GlyphRunOptions options)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(font);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(options.Brush);

        if (glyphs.IsEmpty)
        {
            return;
        }

        using var brushMarshaler = options.Brush.CreateMarshaler();

        var glyphBuffer = new VelloGlyph[glyphs.Length];
        for (var i = 0; i < glyphs.Length; i++)
        {
            glyphBuffer[i] = glyphs[i].ToNative();
        }

        GCHandle dashHandle = default;
        var nativeOptions = new VelloGlyphRunOptions
        {
            Transform = options.Transform.ToNativeAffine(),
            FontSize = options.FontSize,
            Hint = options.Hint,
            Style = (VelloGlyphRunStyle)options.Style,
            Brush = brushMarshaler.Brush,
            BrushAlpha = options.BrushAlpha,
            StrokeStyle = default,
            GlyphTransform = IntPtr.Zero,
        };

        try
        {
            if (options.Style == GlyphRunStyle.Stroke)
            {
                var stroke = options.Stroke ?? throw new ArgumentException("Stroke options must be provided when GlyphRunStyle.Stroke is used.", nameof(options));
                if (stroke.DashPattern is { Length: > 0 } dashPattern)
                {
                    dashHandle = GCHandle.Alloc(dashPattern, GCHandleType.Pinned);
                    nativeOptions.StrokeStyle = CreateStrokeStyle(stroke, dashHandle.AddrOfPinnedObject(), (nuint)dashPattern.Length);
                }
                else
                {
                    nativeOptions.StrokeStyle = CreateStrokeStyle(stroke, IntPtr.Zero, 0);
                }
            }

            unsafe
            {
                fixed (VelloGlyph* glyphPtr = glyphBuffer)
                {
                    VelloAffine glyphTransformValue = default;
                    if (options.GlyphTransform.HasValue)
                    {
                        glyphTransformValue = options.GlyphTransform.Value.ToNativeAffine();
                        nativeOptions.GlyphTransform = (IntPtr)(&glyphTransformValue);
                    }
                    else
                    {
                        nativeOptions.GlyphTransform = IntPtr.Zero;
                    }

                    var status = NativeMethods.vello_scene_draw_glyph_run(
                        _handle,
                        font.Handle,
                        glyphPtr,
                        (nuint)glyphBuffer.Length,
                        in nativeOptions);

                    NativeHelpers.ThrowOnError(status, "DrawGlyphRun failed");
                }
            }
        }
        finally
        {
            if (dashHandle.IsAllocated)
            {
                dashHandle.Free();
            }
        }
    }

    ~Scene()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_scene_destroy(_handle);
        }
    }

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
