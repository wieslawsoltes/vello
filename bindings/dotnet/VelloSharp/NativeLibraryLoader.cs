using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace VelloSharp;

internal static class NativeLibraryLoader
{
    private static readonly object Sync = new();
    private static bool _initialized;

#pragma warning disable CA2255 // Module initializers limited use warning suppressed intentionally for native resolver registration.
    [ModuleInitializer]
    internal static void Initialize() => EnsureInitialized();
#pragma warning restore CA2255

    private static void EnsureInitialized()
    {
        if (_initialized)
        {
            return;
        }

        lock (Sync)
        {
            if (_initialized)
            {
                return;
            }

            NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, Resolve);
            _initialized = true;
        }
    }

    private static IntPtr Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, NativeMethods.LibraryName, StringComparison.Ordinal))
        {
            return IntPtr.Zero;
        }

        foreach (var candidate in EnumerateProbePaths(assembly))
        {
            if (!string.IsNullOrWhiteSpace(candidate) && NativeLibrary.TryLoad(candidate, out var handle))
            {
                return handle;
            }
        }

        return NativeLibrary.Load(libraryName, assembly, searchPath);
    }

    private static IEnumerable<string> EnumerateProbePaths(Assembly assembly)
    {
        var fileName = GetLibraryFileName();
        var rid = RuntimeInformation.RuntimeIdentifier;

        if (!string.IsNullOrEmpty(AppContext.BaseDirectory))
        {
            yield return Path.Combine(AppContext.BaseDirectory, fileName);
            if (!string.IsNullOrEmpty(rid))
            {
                yield return Path.Combine(AppContext.BaseDirectory, "runtimes", rid, "native", fileName);

                var ridBase = GetRidBase(rid);
                if (!string.Equals(ridBase, rid, StringComparison.Ordinal))
                {
                    yield return Path.Combine(AppContext.BaseDirectory, "runtimes", ridBase, "native", fileName);
                }
            }
        }

        var assemblyLocation = assembly.Location;
        if (!string.IsNullOrWhiteSpace(assemblyLocation))
        {
            var assemblyDir = Path.GetDirectoryName(assemblyLocation);
            if (!string.IsNullOrEmpty(assemblyDir))
            {
                yield return Path.Combine(assemblyDir, fileName);
                if (!string.IsNullOrEmpty(rid))
                {
                    yield return Path.Combine(assemblyDir, "runtimes", rid, "native", fileName);

                    var ridBase = GetRidBase(rid);
                    if (!string.Equals(ridBase, rid, StringComparison.Ordinal))
                    {
                        yield return Path.Combine(assemblyDir, "runtimes", ridBase, "native", fileName);
                    }
                }
            }
        }

        yield return fileName;
    }

    private static string GetRidBase(string rid)
    {
        if (string.IsNullOrEmpty(rid))
        {
            return string.Empty;
        }

        var dashIndex = rid.IndexOf('-');
        return dashIndex > 0 ? rid[..dashIndex] : rid;
    }

    private static string GetLibraryFileName()
    {
        if (OperatingSystem.IsWindows())
        {
            return "vello_ffi.dll";
        }

        if (OperatingSystem.IsMacOS())
        {
            return "libvello_ffi.dylib";
        }

        return "libvello_ffi.so";
    }
}
