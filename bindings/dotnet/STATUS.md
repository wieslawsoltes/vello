# VelloSharp Binding Status

This document captures the current state of the .NET bindings for Vello and the work required to make them production ready.

## Current State

- `bindings/dotnet/VelloSharp` currently exposes only `Renderer.Render(...)` and `Scene.FillPath`/`Scene.StrokePath`. Everything else in Vello’s scene API (layers, brushes, images, text, blur primitives, gradients, glyph runs) remains inaccessible. See `bindings/dotnet/VelloSharp/VelloScene.cs`, `bindings/dotnet/VelloSharp/VelloRenderer.cs`.
- Interop still relies on manual `.dylib` copies; debug instrumentation (`println!`, `Console.WriteLine`) is compiled in; the CPU readback pipeline in `bindings/vello_ffi/src/lib.rs` must be stabilised (mapping/unmapping, format selection).
- Samples render via copying into Avalonia’s `WriteableBitmap`; there is no Skia/Vello interop helper, no GPU render path, and no automated verification.

## Completion Plan

- **Harden the FFI layer:** remove lingering diagnostics, clean up the readback pipeline (single map/unmap per frame, correct texture usages, configurable BGRA/RGBA), and gate features so the Rust side can ship as a reusable crate.
- **Expose missing scene/renderer surface area:** add externs and C# wrappers for layer stack management, gradients/brushes, images, glyph runs, blurred rectangles, stroke options, and renderer options (AA support, CPU fallback) so the .NET API covers Vello’s primitives.
- **Create safe C# abstractions:** manage resource lifetimes via `IDisposable`, introduce immutable structs for colors/gradients, provide span-friendly glyph/path builders, and ensure argument validation/error propagation through `NativeHelpers`.
- **Automate native loading:** implement a `DllImport` resolver and RID-specific binary distribution so `dotnet build` produces and copies the correct `libvello_ffi` without manual intervention.
- **Add integration helpers:** supply an Avalonia control owning the renderer, a SkiaSharp `SKBitmap`/`SKSurface` bridge, and CPU/GPU render paths with stride/format negotiation.
- **Improve coverage:** add unit tests for marshaling and stroke/fill behaviours, render smoke tests that hash outputs, CI jobs building Rust and .NET, and documentation in `bindings/dotnet/README.md` covering setup, threading, and deployment.

