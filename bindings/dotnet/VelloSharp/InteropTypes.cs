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
}
