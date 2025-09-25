using System;
using System.Runtime.InteropServices;

namespace VelloSharp;

internal enum VelloStatus
{
    Success = 0,
    NullPointer = 1,
    InvalidArgument = 2,
    DeviceCreationFailed = 3,
    RenderError = 4,
    MapFailed = 5,
    Unsupported = 6,
}

internal enum VelloFillRule : int
{
    NonZero = 0,
    EvenOdd = 1,
}

internal enum VelloPathVerb : int
{
    MoveTo = 0,
    LineTo = 1,
    QuadTo = 2,
    CubicTo = 3,
    Close = 4,
}

internal enum VelloLineCap : int
{
    Butt = 0,
    Round = 1,
    Square = 2,
}

internal enum VelloLineJoin : int
{
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

internal enum VelloAaMode : int
{
    Area = 0,
    Msaa8 = 1,
    Msaa16 = 2,
}

internal enum VelloRenderFormat : int
{
    Rgba8 = 0,
    Bgra8 = 1,
}

internal enum VelloImageAlphaMode : int
{
    Straight = 0,
    Premultiplied = 1,
}

internal enum VelloExtendMode : int
{
    Pad = 0,
    Repeat = 1,
    Reflect = 2,
}

internal enum VelloImageQualityMode : int
{
    Low = 0,
    Medium = 1,
    High = 2,
}

internal enum VelloBrushKind : int
{
    Solid = 0,
    LinearGradient = 1,
    RadialGradient = 2,
    Image = 3,
}

internal enum VelloBlendMix : int
{
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

internal enum VelloBlendCompose : int
{
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

internal enum VelloGlyphRunStyle : int
{
    Fill = 0,
    Stroke = 1,
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloPoint
{
    public double X;
    public double Y;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloPathElement
{
    public VelloPathVerb Verb;
    private int _padding;
    public double X0;
    public double Y0;
    public double X1;
    public double Y1;
    public double X2;
    public double Y2;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloAffine
{
    public double M11;
    public double M12;
    public double M21;
    public double M22;
    public double Dx;
    public double Dy;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloColor
{
    public float R;
    public float G;
    public float B;
    public float A;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloGradientStop
{
    public float Offset;
    public VelloColor Color;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloLinearGradient
{
    public VelloPoint Start;
    public VelloPoint End;
    public VelloExtendMode Extend;
    public IntPtr Stops;
    public nuint StopCount;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloRadialGradient
{
    public VelloPoint StartCenter;
    public float StartRadius;
    public VelloPoint EndCenter;
    public float EndRadius;
    public VelloExtendMode Extend;
    public IntPtr Stops;
    public nuint StopCount;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloImageBrushParams
{
    public IntPtr Image;
    public VelloExtendMode XExtend;
    public VelloExtendMode YExtend;
    public VelloImageQualityMode Quality;
    public float Alpha;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloBrush
{
    public VelloBrushKind Kind;
    public VelloColor Solid;
    public VelloLinearGradient Linear;
    public VelloRadialGradient Radial;
    public VelloImageBrushParams Image;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloStrokeStyle
{
    public double Width;
    public double MiterLimit;
    public VelloLineCap StartCap;
    public VelloLineCap EndCap;
    public VelloLineJoin LineJoin;
    public double DashPhase;
    public IntPtr DashPattern;
    public nuint DashLength;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloRenderParams
{
    public uint Width;
    public uint Height;
    public VelloColor BaseColor;
    public VelloAaMode Antialiasing;
    public VelloRenderFormat Format;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloRect
{
    public double X;
    public double Y;
    public double Width;
    public double Height;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloLayerParams
{
    public VelloBlendMix Mix;
    public VelloBlendCompose Compose;
    public float Alpha;
    public VelloAffine Transform;
    public IntPtr ClipElements;
    public nuint ClipElementCount;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloRendererOptions
{
    [MarshalAs(UnmanagedType.I1)]
    public bool UseCpu;
    [MarshalAs(UnmanagedType.I1)]
    public bool SupportArea;
    [MarshalAs(UnmanagedType.I1)]
    public bool SupportMsaa8;
    [MarshalAs(UnmanagedType.I1)]
    public bool SupportMsaa16;
    public int InitThreads;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloGlyph
{
    public uint Id;
    public float X;
    public float Y;
}

[StructLayout(LayoutKind.Sequential)]
internal struct VelloGlyphRunOptions
{
    public VelloAffine Transform;
    public IntPtr GlyphTransform;
    public float FontSize;
    [MarshalAs(UnmanagedType.I1)]
    public bool Hint;
    public VelloGlyphRunStyle Style;
    public VelloBrush Brush;
    public float BrushAlpha;
    public VelloStrokeStyle StrokeStyle;
}
