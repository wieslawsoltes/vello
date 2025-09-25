using System;
using System.Numerics;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform;
using Avalonia.Threading;
using VelloSharp;
using FillRule = VelloSharp.FillRule;
using Vector = Avalonia.Vector;

namespace AvaloniaVelloDemo;

public sealed class VelloView : Control, IDisposable
{
    private readonly Renderer _renderer;
    private readonly Scene _scene;
    private readonly PathBuilder _path = new();
    private readonly StrokeStyle _stroke = new() { Width = 1.5, LineJoin = LineJoin.Miter, StartCap = LineCap.Butt, EndCap = LineCap.Butt };
    private WriteableBitmap? _bitmap;
    private readonly DispatcherTimer _timer;
    private float _time;
    private bool _disposed;

    public VelloView()
    {
        ClipToBounds = true;
        _renderer = new Renderer(1, 1);
        _scene = new Scene();

        _timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(16) };
        _timer.Tick += (_, _) =>
        {
            _time += 0.016f;
            InvalidateVisual();
        };
        _timer.Start();
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);

        if (_disposed)
        {
            return;
        }

        var scaling = VisualRoot?.RenderScaling ?? 1.0;
        var width = Math.Max(1, (int)Math.Ceiling(Bounds.Width * scaling));
        var height = Math.Max(1, (int)Math.Ceiling(Bounds.Height * scaling));

        EnsureBitmap(width, height, scaling);
        BuildScene(width, height, _time);

        if (_bitmap is null)
        {
            return;
        }

        using var frame = _bitmap.Lock();
        unsafe
        {
            var span = new Span<byte>((void*)frame.Address, frame.RowBytes * frame.Size.Height);
            var parameters = new RenderParams(
                (uint)width,
                (uint)height,
                RgbaColor.FromBytes(18, 18, 20),
                AntialiasingMode.Msaa8)
            {
                Format = RenderFormat.Bgra8,
            };
            _renderer.Render(_scene, parameters, span, frame.RowBytes);
        }

        var sourceRect = new Rect(0, 0, _bitmap.PixelSize.Width, _bitmap.PixelSize.Height);
        context.DrawImage(_bitmap, sourceRect, Bounds);
    }

    private void BuildScene(int width, int height, float time)
    {
        _scene.Reset();
        var transform = Matrix3x2.Identity;

        DrawGrid(width, height, transform);
        DrawAxis(width, height, transform);
        DrawRotor(width, height, time);
    }

    private void DrawGrid(int width, int height, Matrix3x2 transform)
    {
        const int Step = 64;
        var color = RgbaColor.FromBytes(60, 60, 72, 255);
        for (var x = Step; x < width; x += Step)
        {
            _path.Clear();
            _path.MoveTo(x, 0).LineTo(x, height);
            _scene.StrokePath(_path, _stroke, transform, color);
        }

        for (var y = Step; y < height; y += Step)
        {
            _path.Clear();
            _path.MoveTo(0, y).LineTo(width, y);
            _scene.StrokePath(_path, _stroke, transform, color);
        }
    }

    private void DrawAxis(int width, int height, Matrix3x2 transform)
    {
        var center = new Vector2(width / 2f, height / 2f);
        var axisColor = RgbaColor.FromBytes(220, 85, 85);
        var axisStroke = new StrokeStyle { Width = 2.5, LineJoin = LineJoin.Miter, StartCap = LineCap.Round, EndCap = LineCap.Round };

        _path.Clear();
        _path.MoveTo(0, center.Y).LineTo(width, center.Y);
        _scene.StrokePath(_path, axisStroke, transform, axisColor);

        axisColor = RgbaColor.FromBytes(95, 175, 240);
        _path.Clear();
        _path.MoveTo(center.X, 0).LineTo(center.X, height);
        _scene.StrokePath(_path, axisStroke, transform, axisColor);
    }

    private void DrawRotor(int width, int height, float time)
    {
        var center = new Vector2(width / 2f, height / 2f);
        var radius = Math.Min(width, height) * 0.35f;
        var rotation = Matrix3x2.CreateRotation(time * 0.8f, center);

        _path.Clear();
        const int BladeCount = 5;
        for (var i = 0; i < BladeCount; i++)
        {
            var angle = (float)(i * Math.Tau / BladeCount);
            var tip = center + new Vector2((float)Math.Cos(angle), (float)Math.Sin(angle)) * radius;
            var ctrl = center + new Vector2((float)Math.Cos(angle + 0.3f), (float)Math.Sin(angle + 0.3f)) * (radius * 0.4f);

            _path.MoveTo(center.X, center.Y);
            _path.QuadraticTo(ctrl.X, ctrl.Y, tip.X, tip.Y);
            _path.Close();
        }

        var bladeColor = RgbaColor.FromBytes(156, 220, 255, 204);
        _scene.FillPath(_path, FillRule.NonZero, rotation, bladeColor);

        var rimStroke = new StrokeStyle
        {
            Width = 4.5,
            LineJoin = LineJoin.Round,
            StartCap = LineCap.Round,
            EndCap = LineCap.Round,
        };

        _path.Clear();
        const int Segments = 64;
        for (var i = 0; i <= Segments; i++)
        {
            var t = i / (float)Segments;
            var angle = t * MathF.Tau;
            var point = center + new Vector2(MathF.Cos(angle), MathF.Sin(angle)) * radius;
            if (i == 0)
            {
                _path.MoveTo(point.X, point.Y);
            }
            else
            {
                _path.LineTo(point.X, point.Y);
            }
        }
        _path.Close();

        _scene.StrokePath(_path, rimStroke, Matrix3x2.Identity, RgbaColor.FromBytes(240, 240, 245, 230));
    }

    private void EnsureBitmap(int width, int height, double scaling)
    {
        if (_bitmap is { } bitmap && bitmap.PixelSize.Width == width && bitmap.PixelSize.Height == height)
        {
            return;
        }

        _bitmap?.Dispose();
        var dpi = 96.0 * scaling;
        _bitmap = new WriteableBitmap(
            new PixelSize(width, height),
            new Vector(dpi, dpi),
            PixelFormat.Bgra8888,
            AlphaFormat.Premul);
        _renderer.Resize((uint)width, (uint)height);
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _timer.Stop();
        _bitmap?.Dispose();
        _scene.Dispose();
        _renderer.Dispose();
        _disposed = true;
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnDetachedFromVisualTree(e);
        Dispose();
    }
}
