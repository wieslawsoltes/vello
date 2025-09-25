# VelloSharp .NET bindings

This directory hosts the managed bindings that expose the [`vello`](https://github.com/linebender/vello)
renderer to .NET applications. The bindings are layered:

- `bindings/vello_ffi` – a Rust `cdylib` that wraps the core Vello renderer behind a C ABI.
- `bindings/dotnet/VelloSharp` – a C# library that provides a safe, idiomatic API over the native exports.
- `bindings/dotnet/samples/AvaloniaVelloDemo` – an Avalonia desktop sample that renders an animated scene
  and blits it to a `WriteableBitmap` (making it easy to integrate with Skia-backed UI frameworks).

## Building the native library

Install the Rust toolchain (Rust 1.86 or newer) before building the managed projects. The `VelloSharp`
MSBuild project now drives `cargo build -p vello_ffi` automatically for the active .NET runtime identifier
and configuration. Running any of the following commands produces the native artifact and copies it to the
managed output directory under `runtimes/<rid>/native/` (and alongside the binaries for convenience):

```bash
dotnet build bindings/dotnet/VelloSharp/VelloSharp.csproj
dotnet build bindings/dotnet/samples/AvaloniaVelloDemo/AvaloniaVelloDemo.csproj
dotnet run --project bindings/dotnet/samples/AvaloniaVelloDemo/AvaloniaVelloDemo.csproj
```

By default the current host target triple is used. To build for an alternate RID, pass `-r <rid>` when
invoking `dotnet build` or set `RuntimeIdentifier` in your consuming project; make sure the corresponding
Rust target is installed (`rustup target add <triple>`). The produced files are named:

| RID        | Triple                       | Artifact                 |
| ---------- | ---------------------------- | ------------------------ |
| `win-x64`  | `x86_64-pc-windows-msvc`     | `vello_ffi.dll`          |
| `win-arm64`| `aarch64-pc-windows-msvc`    | `vello_ffi.dll`          |
| `osx-x64`  | `x86_64-apple-darwin`        | `libvello_ffi.dylib`     |
| `osx-arm64`| `aarch64-apple-darwin`       | `libvello_ffi.dylib`     |
| `linux-x64`| `x86_64-unknown-linux-gnu`   | `libvello_ffi.so`        |
| `linux-arm64`| `aarch64-unknown-linux-gnu`| `libvello_ffi.so`        |

## Using `VelloSharp`

Reference the `VelloSharp` project from your solution or publish it as a NuGet package.
A minimal render loop looks like:

```csharp
using System.Numerics;
using VelloSharp;

using var renderer = new Renderer(width: 1024, height: 768);
using var scene = new Scene();
var path = new PathBuilder();
path.MoveTo(100, 100).LineTo(700, 200).LineTo(420, 540).Close();
scene.FillPath(path, FillRule.NonZero, Matrix3x2.Identity, RgbaColor.FromBytes(0x47, 0x91, 0xF9));

var buffer = new byte[1024 * 768 * 4];
renderer.Render(
    scene,
    new RenderParams(1024, 768, RgbaColor.FromBytes(0x10, 0x10, 0x12))
    {
        Format = RenderFormat.Bgra8,
    },
    buffer,
    strideBytes: 1024 * 4);
```

`buffer` now contains BGRA pixels ready for presentation via SkiaSharp, Avalonia or any other API; omit the assignment to `Format` to receive RGBA output instead.

## Brushes and Layers

`Scene.FillPath` and `Scene.StrokePath` accept the `Brush` hierarchy, enabling linear/radial gradients and image brushes in addition to solid colors. Example:

```csharp
var brush = new LinearGradientBrush(
    start: new Vector2(0, 0),
    end: new Vector2(256, 0),
    stops: new[]
    {
        new GradientStop(0f, RgbaColor.FromBytes(255, 0, 128)),
        new GradientStop(1f, RgbaColor.FromBytes(0, 128, 255)),
    });
scene.FillPath(path, FillRule.NonZero, Matrix3x2.Identity, brush);
```

Layer management is accessible through `Scene.PushLayer`, `Scene.PushLuminanceMaskLayer`, and `Scene.PopLayer`, giving full control over blend modes and clip groups.

## Images and Glyphs

Use `Image.FromPixels` and `Scene.DrawImage` to render textures directly. Glyph runs can be issued via `Scene.DrawGlyphRun`, which takes a `Font`, a glyph span, and `GlyphRunOptions` for fill or stroke rendering.

## Renderer Options

`Renderer` exposes an optional `RendererOptions` argument to select CPU-only rendering or limit the available anti-aliasing pipelines at creation time.

## Avalonia integration

The `VelloView` control in the sample project demonstrates how to render directly into an
Avalonia `WriteableBitmap` that is drawn with the Skia backend:

```csharp
using var frame = _bitmap.Lock();
unsafe
{
    var span = new Span<byte>((void*)frame.Address, frame.RowBytes * frame.Size.Height);
    _renderer.Render(_scene, parameters, span, frame.RowBytes);
}
context.DrawImage(_bitmap, sourceRect, Bounds);
```

Key points:

- Use `VisualRoot.RenderScaling` to obtain physical pixels from logical Avalonia units.
- Reuse `Scene` and `Renderer` across frames; call `Scene.Reset()` before encoding a new frame.
- Pass the stride reported by `ILockedFramebuffer.RowBytes` back into `Renderer.Render`.

Run the sample with:

```bash
cd bindings/dotnet/samples/AvaloniaVelloDemo
dotnet run
```

The native `vello_ffi` library is copied next to the managed binaries automatically; no additional setup is
required as long as the Rust toolchain is installed.

## SkiaSharp interop

If you are already using SkiaSharp primitives (for example in Avalonia custom draw calls), you can wrap
Vello's BGRA output without additional conversions:

```csharp
using SkiaSharp;

void BlitToCanvas(SKCanvas canvas, ReadOnlySpan<byte> pixels, int width, int height, int stride)
{
    var info = new SKImageInfo(width, height, SKColorType.Bgra8888, SKAlphaType.Premul);
    unsafe
    {
        fixed (byte* ptr = pixels)
        {
            using var pixmap = new SKPixmap(info, (IntPtr)ptr, stride);
            using var paint = new SKPaint { FilterQuality = SKFilterQuality.High };
            canvas.DrawPixmap(pixmap, SKRect.Create(width, height), paint);
        }
    }
}
```

`InstallPixels` pins the span for the duration of the draw. If you need to retain the bitmap beyond the
current frame, copy the data into a dedicated `SKBitmap`.

## Repository layout recap

- `bindings/vello_ffi`: Rust source for the native shared library.
- `bindings/dotnet/VelloSharp`: C# wrapper library with `Scene`, `Renderer`, and path-building helpers.
- `bindings/dotnet/samples/AvaloniaVelloDemo`: Avalonia desktop sample that exercises the bindings.
