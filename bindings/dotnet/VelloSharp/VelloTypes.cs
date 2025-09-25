using System;
using System.Collections.Generic;
using System.Numerics;
using System.Runtime.InteropServices;

namespace VelloSharp;

public enum FillRule
{
    NonZero = VelloFillRule.NonZero,
    EvenOdd = VelloFillRule.EvenOdd,
}

public enum LineCap
{
    Butt = VelloLineCap.Butt,
    Round = VelloLineCap.Round,
    Square = VelloLineCap.Square,
}

public enum LineJoin
{
    Miter = VelloLineJoin.Miter,
    Round = VelloLineJoin.Round,
    Bevel = VelloLineJoin.Bevel,
}

public enum AntialiasingMode
{
    Area = VelloAaMode.Area,
    Msaa8 = VelloAaMode.Msaa8,
    Msaa16 = VelloAaMode.Msaa16,
}

public enum RenderFormat
{
    Rgba8 = VelloRenderFormat.Rgba8,
    Bgra8 = VelloRenderFormat.Bgra8,
}

public enum ImageAlphaMode
{
    Straight = VelloImageAlphaMode.Straight,
    Premultiplied = VelloImageAlphaMode.Premultiplied,
}

public enum ExtendMode
{
    Pad = VelloExtendMode.Pad,
    Repeat = VelloExtendMode.Repeat,
    Reflect = VelloExtendMode.Reflect,
}

public enum ImageQuality
{
    Low = VelloImageQualityMode.Low,
    Medium = VelloImageQualityMode.Medium,
    High = VelloImageQualityMode.High,
}

public enum LayerMix
{
    Normal = VelloBlendMix.Normal,
    Multiply = VelloBlendMix.Multiply,
    Screen = VelloBlendMix.Screen,
    Overlay = VelloBlendMix.Overlay,
    Darken = VelloBlendMix.Darken,
    Lighten = VelloBlendMix.Lighten,
    ColorDodge = VelloBlendMix.ColorDodge,
    ColorBurn = VelloBlendMix.ColorBurn,
    HardLight = VelloBlendMix.HardLight,
    SoftLight = VelloBlendMix.SoftLight,
    Difference = VelloBlendMix.Difference,
    Exclusion = VelloBlendMix.Exclusion,
    Hue = VelloBlendMix.Hue,
    Saturation = VelloBlendMix.Saturation,
    Color = VelloBlendMix.Color,
    Luminosity = VelloBlendMix.Luminosity,
    Clip = VelloBlendMix.Clip,
}

public enum LayerCompose
{
    Clear = VelloBlendCompose.Clear,
    Copy = VelloBlendCompose.Copy,
    Dest = VelloBlendCompose.Dest,
    SrcOver = VelloBlendCompose.SrcOver,
    DestOver = VelloBlendCompose.DestOver,
    SrcIn = VelloBlendCompose.SrcIn,
    DestIn = VelloBlendCompose.DestIn,
    SrcOut = VelloBlendCompose.SrcOut,
    DestOut = VelloBlendCompose.DestOut,
    SrcAtop = VelloBlendCompose.SrcAtop,
    DestAtop = VelloBlendCompose.DestAtop,
    Xor = VelloBlendCompose.Xor,
    Plus = VelloBlendCompose.Plus,
    PlusLighter = VelloBlendCompose.PlusLighter,
}

public enum GlyphRunStyle
{
    Fill = VelloGlyphRunStyle.Fill,
    Stroke = VelloGlyphRunStyle.Stroke,
}

public readonly record struct RgbaColor(float R, float G, float B, float A)
{
    public static RgbaColor FromBytes(byte r, byte g, byte b, byte a = 255)
    {
        const float Scale = 1f / 255f;
        return new RgbaColor(r * Scale, g * Scale, b * Scale, a * Scale);
    }
}

public readonly record struct GradientStop(float Offset, RgbaColor Color)
{
    public float Offset { get; init; } = Offset;
    public RgbaColor Color { get; init; } = Color;
}

public abstract class Brush
{
    internal abstract BrushMarshaler CreateMarshaler();
}

public sealed class SolidColorBrush : Brush
{
    public SolidColorBrush(RgbaColor color) => Color = color;

    public RgbaColor Color { get; }

    internal override BrushMarshaler CreateMarshaler()
    {
        var native = new VelloBrush
        {
            Kind = VelloBrushKind.Solid,
            Solid = Color.ToNative(),
            Linear = default,
            Radial = default,
            Image = default,
        };
        return new BrushMarshaler(native, null);
    }
}

public sealed class LinearGradientBrush : Brush
{
    private readonly VelloGradientStop[] _stops;

    public LinearGradientBrush(Vector2 start, Vector2 end, IReadOnlyList<GradientStop> stops, ExtendMode extend = ExtendMode.Pad)
    {
        ArgumentNullException.ThrowIfNull(stops);
        if (stops.Count == 0)
        {
            throw new ArgumentException("At least one gradient stop is required.", nameof(stops));
        }

        Start = start;
        End = end;
        Extend = extend;
        _stops = new VelloGradientStop[stops.Count];
        for (var i = 0; i < stops.Count; i++)
        {
            var stop = stops[i];
            _stops[i] = new VelloGradientStop
            {
                Offset = stop.Offset,
                Color = stop.Color.ToNative(),
            };
        }
    }

    public Vector2 Start { get; }
    public Vector2 End { get; }
    public ExtendMode Extend { get; }

    internal override BrushMarshaler CreateMarshaler()
    {
        var native = new VelloBrush
        {
            Kind = VelloBrushKind.LinearGradient,
            Linear = new VelloLinearGradient
            {
                Start = Start.ToNativePoint(),
                End = End.ToNativePoint(),
                Extend = (VelloExtendMode)Extend,
                StopCount = (nuint)_stops.Length,
            },
        };
        return new BrushMarshaler(native, _stops);
    }
}

public sealed class RadialGradientBrush : Brush
{
    private readonly VelloGradientStop[] _stops;

    public RadialGradientBrush(
        Vector2 startCenter,
        float startRadius,
        Vector2 endCenter,
        float endRadius,
        IReadOnlyList<GradientStop> stops,
        ExtendMode extend = ExtendMode.Pad)
    {
        ArgumentNullException.ThrowIfNull(stops);
        if (stops.Count == 0)
        {
            throw new ArgumentException("At least one gradient stop is required.", nameof(stops));
        }
        if (startRadius < 0f || endRadius < 0f)
        {
            throw new ArgumentOutOfRangeException(nameof(startRadius), "Radii must be non-negative.");
        }

        StartCenter = startCenter;
        StartRadius = startRadius;
        EndCenter = endCenter;
        EndRadius = endRadius;
        Extend = extend;
        _stops = new VelloGradientStop[stops.Count];
        for (var i = 0; i < stops.Count; i++)
        {
            var stop = stops[i];
            _stops[i] = new VelloGradientStop
            {
                Offset = stop.Offset,
                Color = stop.Color.ToNative(),
            };
        }
    }

    public Vector2 StartCenter { get; }
    public float StartRadius { get; }
    public Vector2 EndCenter { get; }
    public float EndRadius { get; }
    public ExtendMode Extend { get; }

    internal override BrushMarshaler CreateMarshaler()
    {
        var native = new VelloBrush
        {
            Kind = VelloBrushKind.RadialGradient,
            Radial = new VelloRadialGradient
            {
                StartCenter = StartCenter.ToNativePoint(),
                StartRadius = StartRadius,
                EndCenter = EndCenter.ToNativePoint(),
                EndRadius = EndRadius,
                Extend = (VelloExtendMode)Extend,
                StopCount = (nuint)_stops.Length,
            },
        };
        return new BrushMarshaler(native, _stops);
    }
}

public sealed class ImageBrush : Brush
{
    public ImageBrush(Image image)
    {
        Image = image ?? throw new ArgumentNullException(nameof(image));
    }

    public Image Image { get; }
    public ExtendMode XExtend { get; set; } = ExtendMode.Pad;
    public ExtendMode YExtend { get; set; } = ExtendMode.Pad;
    public ImageQuality Quality { get; set; } = ImageQuality.Medium;
    public float Alpha { get; set; } = 1f;

    internal override BrushMarshaler CreateMarshaler()
    {
        var native = new VelloBrush
        {
            Kind = VelloBrushKind.Image,
            Image = new VelloImageBrushParams
            {
                Image = Image.Handle,
                XExtend = (VelloExtendMode)XExtend,
                YExtend = (VelloExtendMode)YExtend,
                Quality = (VelloImageQualityMode)Quality,
                Alpha = Alpha,
            },
        };
        return new BrushMarshaler(native, null);
    }

    internal VelloImageBrushParams ToNative() => new()
    {
        Image = Image.Handle,
        XExtend = (VelloExtendMode)XExtend,
        YExtend = (VelloExtendMode)YExtend,
        Quality = (VelloImageQualityMode)Quality,
        Alpha = Alpha,
    };
}

internal struct BrushMarshaler : IDisposable
{
    internal VelloBrush Brush;
    private GCHandle? _stopsHandle;

    internal BrushMarshaler(VelloBrush brush, VelloGradientStop[]? stops)
    {
        Brush = brush;
        _stopsHandle = null;

        Brush.Linear.Stops = IntPtr.Zero;
        Brush.Radial.Stops = IntPtr.Zero;

        if (stops is { Length: > 0 })
        {
            _stopsHandle = GCHandle.Alloc(stops, GCHandleType.Pinned);
            var ptr = _stopsHandle.Value.AddrOfPinnedObject();
            switch (Brush.Kind)
            {
                case VelloBrushKind.LinearGradient:
                    Brush.Linear.Stops = ptr;
                    break;
                case VelloBrushKind.RadialGradient:
                    Brush.Radial.Stops = ptr;
                    break;
            }
        }
    }

    public void Dispose()
    {
        if (_stopsHandle.HasValue)
        {
            _stopsHandle.Value.Free();
            _stopsHandle = null;
        }
    }
}

internal static class NativeConversionExtensions
{
    public static VelloColor ToNative(this RgbaColor color) => new()
    {
        R = color.R,
        G = color.G,
        B = color.B,
        A = color.A,
    };

    public static VelloPoint ToNativePoint(this Vector2 point) => new()
    {
        X = point.X,
        Y = point.Y,
    };

    public static VelloAffine ToNativeAffine(this Matrix3x2 matrix) => new()
    {
        M11 = matrix.M11,
        M12 = matrix.M12,
        M21 = matrix.M21,
        M22 = matrix.M22,
        Dx = matrix.M31,
        Dy = matrix.M32,
    };
}

public readonly struct RendererOptions
{
    public RendererOptions(
        bool useCpu = false,
        bool supportArea = true,
        bool supportMsaa8 = true,
        bool supportMsaa16 = true,
        int? initThreads = null)
    {
        UseCpu = useCpu;
        SupportArea = supportArea;
        SupportMsaa8 = supportMsaa8;
        SupportMsaa16 = supportMsaa16;
        InitThreads = initThreads;
    }

    public bool UseCpu { get; }
    public bool SupportArea { get; }
    public bool SupportMsaa8 { get; }
    public bool SupportMsaa16 { get; }
    public int? InitThreads { get; }

    internal VelloRendererOptions ToNative() => new()
    {
        UseCpu = UseCpu,
        SupportArea = SupportArea,
        SupportMsaa8 = SupportMsaa8,
        SupportMsaa16 = SupportMsaa16,
        InitThreads = InitThreads ?? 0,
    };
}

public readonly struct LayerBlend
{
    public LayerBlend(LayerMix mix, LayerCompose compose)
    {
        Mix = mix;
        Compose = compose;
    }

    public LayerMix Mix { get; }
    public LayerCompose Compose { get; }
}

public readonly struct Glyph
{
    public Glyph(uint id, float x, float y)
    {
        Id = id;
        X = x;
        Y = y;
    }

    public uint Id { get; }
    public float X { get; }
    public float Y { get; }

    internal VelloGlyph ToNative() => new()
    {
        Id = Id,
        X = X,
        Y = Y,
    };
}

public sealed class GlyphRunOptions
{
    public Brush Brush { get; set; } = new SolidColorBrush(RgbaColor.FromBytes(0, 0, 0));
    public float FontSize { get; set; } = 16f;
    public bool Hint { get; set; }
    public GlyphRunStyle Style { get; set; } = GlyphRunStyle.Fill;
    public StrokeStyle? Stroke { get; set; }
    public float BrushAlpha { get; set; } = 1f;
    public Matrix3x2 Transform { get; set; } = Matrix3x2.Identity;
    public Matrix3x2? GlyphTransform { get; set; }
}

public sealed class Image : IDisposable
{
    private IntPtr _handle;

    private Image(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle
    {
        get
        {
            if (_handle == IntPtr.Zero)
            {
                throw new ObjectDisposedException(nameof(Image));
            }
            return _handle;
        }
    }

    public static Image FromPixels(
        ReadOnlySpan<byte> pixels,
        int width,
        int height,
        RenderFormat format = RenderFormat.Rgba8,
        ImageAlphaMode alphaMode = ImageAlphaMode.Straight,
        int stride = 0)
    {
        if (width <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(width));
        }
        if (height <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(height));
        }
        var bytesPerRow = checked(width * 4);
        if (stride == 0)
        {
            stride = bytesPerRow;
        }
        if (stride < bytesPerRow)
        {
            throw new ArgumentOutOfRangeException(nameof(stride));
        }
        var required = checked(stride * height);
        if (pixels.Length < required)
        {
            throw new ArgumentException("Pixel data is smaller than expected for the provided stride and dimensions.", nameof(pixels));
        }

        var buffer = pixels[..required].ToArray();
        var handle = GCHandle.Alloc(buffer, GCHandleType.Pinned);
        try
        {
            var ptr = handle.AddrOfPinnedObject();
            var native = NativeMethods.vello_image_create(
                (VelloRenderFormat)format,
                (VelloImageAlphaMode)alphaMode,
                (uint)width,
                (uint)height,
                ptr,
                (nuint)stride);
            if (native == IntPtr.Zero)
            {
                throw new InvalidOperationException(NativeHelpers.GetLastErrorMessage() ?? "Failed to create image.");
            }
            return new Image(native);
        }
        finally
        {
            if (handle.IsAllocated)
            {
                handle.Free();
            }
        }
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_image_destroy(_handle);
            _handle = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }
    }

    ~Image()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_image_destroy(_handle);
        }
    }
}

public sealed class Font : IDisposable
{
    private IntPtr _handle;

    private Font(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle
    {
        get
        {
            if (_handle == IntPtr.Zero)
            {
                throw new ObjectDisposedException(nameof(Font));
            }
            return _handle;
        }
    }

    public static Font Load(ReadOnlySpan<byte> fontData, uint index = 0)
    {
        var buffer = fontData.ToArray();
        var handle = GCHandle.Alloc(buffer, GCHandleType.Pinned);
        try
        {
            var ptr = handle.AddrOfPinnedObject();
            var native = NativeMethods.vello_font_create(ptr, (nuint)buffer.Length, index);
            if (native == IntPtr.Zero)
            {
                throw new InvalidOperationException(NativeHelpers.GetLastErrorMessage() ?? "Failed to load font.");
            }
            return new Font(native);
        }
        finally
        {
            if (handle.IsAllocated)
            {
                handle.Free();
            }
        }
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_font_destroy(_handle);
            _handle = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }
    }

    ~Font()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.vello_font_destroy(_handle);
        }
    }
}

public sealed class StrokeStyle
{
    public double Width { get; set; } = 1.0;
    public double MiterLimit { get; set; } = 4.0;
    public LineCap StartCap { get; set; } = LineCap.Butt;
    public LineCap EndCap { get; set; } = LineCap.Butt;
    public LineJoin LineJoin { get; set; } = LineJoin.Miter;
    public double DashPhase { get; set; }
    public double[]? DashPattern { get; set; }
}

public readonly record struct RenderParams(
    uint Width,
    uint Height,
    RgbaColor BaseColor,
    AntialiasingMode Antialiasing = AntialiasingMode.Msaa8,
    RenderFormat Format = RenderFormat.Rgba8)
{
    public uint Width { get; init; } = Width;
    public uint Height { get; init; } = Height;
    public RgbaColor BaseColor { get; init; } = BaseColor;
    public AntialiasingMode Antialiasing { get; init; } = Antialiasing;
    public RenderFormat Format { get; init; } = Format;
}
