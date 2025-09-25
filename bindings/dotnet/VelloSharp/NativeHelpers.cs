using System;
using System.Runtime.InteropServices;

namespace VelloSharp;

internal static class NativeHelpers
{
    internal static void ThrowOnError(VelloStatus status, string message)
    {
        if (status == VelloStatus.Success)
        {
            return;
        }

        var native = GetLastErrorMessage();
        if (!string.IsNullOrWhiteSpace(native))
        {
            throw new InvalidOperationException($"{message}: {native} (status: {status})");
        }

        throw new InvalidOperationException($"{message} (status: {status})");
    }

    internal static string? GetLastErrorMessage()
    {
        var ptr = NativeMethods.vello_last_error_message();
        return ptr == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(ptr);
    }
}
