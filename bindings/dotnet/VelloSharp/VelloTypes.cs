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

public readonly record struct RgbaColor(float R, float G, float B, float A)
{
    public static RgbaColor FromBytes(byte r, byte g, byte b, byte a = 255)
    {
        const float Scale = 1f / 255f;
        return new RgbaColor(r * Scale, g * Scale, b * Scale, a * Scale);
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
