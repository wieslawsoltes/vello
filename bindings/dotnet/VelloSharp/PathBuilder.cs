using System;
using System.Collections.Generic;
using System.Numerics;
using System.Runtime.InteropServices;

namespace VelloSharp;

public sealed class PathBuilder
{
    private readonly List<VelloPathElement> _elements = new();

    public int Count => _elements.Count;

    internal VelloPathVerb FirstVerb => _elements.Count > 0 ? _elements[0].Verb : default;

    public PathBuilder MoveTo(double x, double y)
    {
        _elements.Add(new VelloPathElement
        {
            Verb = VelloPathVerb.MoveTo,
            X0 = x,
            Y0 = y,
        });
        return this;
    }

    public PathBuilder LineTo(double x, double y)
    {
        _elements.Add(new VelloPathElement
        {
            Verb = VelloPathVerb.LineTo,
            X0 = x,
            Y0 = y,
        });
        return this;
    }

    public PathBuilder QuadraticTo(double cx, double cy, double x, double y)
    {
        _elements.Add(new VelloPathElement
        {
            Verb = VelloPathVerb.QuadTo,
            X0 = cx,
            Y0 = cy,
            X1 = x,
            Y1 = y,
        });
        return this;
    }

    public PathBuilder CubicTo(double c1x, double c1y, double c2x, double c2y, double x, double y)
    {
        _elements.Add(new VelloPathElement
        {
            Verb = VelloPathVerb.CubicTo,
            X0 = c1x,
            Y0 = c1y,
            X1 = c2x,
            Y1 = c2y,
            X2 = x,
            Y2 = y,
        });
        return this;
    }

    public PathBuilder Close()
    {
        _elements.Add(new VelloPathElement { Verb = VelloPathVerb.Close });
        return this;
    }

    public void Clear() => _elements.Clear();

    internal ReadOnlySpan<VelloPathElement> AsSpan()
    {
        return CollectionsMarshal.AsSpan(_elements);
    }
}
