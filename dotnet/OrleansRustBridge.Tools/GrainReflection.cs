using System.Reflection;

using Orleans;

using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tools;

/// <summary>A grain method discovered by reflection.</summary>
public sealed record DiscoveredMethod(
    string Name,
    Type? RequestType,
    Type? ReturnType,
    IReadOnlyList<(string Name, Type Type)> Parameters);

/// <summary>A grain interface discovered by reflection.</summary>
public sealed record DiscoveredGrain(
    Type Interface,
    string InterfaceName,
    string GrainType,
    string KeyKind,
    IReadOnlyList<DiscoveredMethod> Methods);

/// <summary>
/// Reflects over an assembly to discover Orleans grain interfaces and the
/// metadata needed to emit a manifest or generate bridge invokers.
/// </summary>
public static class GrainReflection
{
    /// <summary>Discover grain interfaces in <paramref name="assembly"/>.</summary>
    public static IReadOnlyList<DiscoveredGrain> Discover(Assembly assembly, ISet<string>? only)
    {
        var grains = new List<DiscoveredGrain>();
        foreach (var type in assembly.GetExportedTypes())
        {
            if (!type.IsInterface || !typeof(IGrain).IsAssignableFrom(type) || IsMarker(type))
            {
                continue;
            }

            if (only is { Count: > 0 } && !only.Contains(type.FullName ?? type.Name))
            {
                continue;
            }

            var keyKind = KeyKindFor(type);
            if (keyKind is null)
            {
                continue;
            }

            grains.Add(new DiscoveredGrain(
                type,
                type.FullName ?? type.Name,
                GrainTypeFor(type),
                keyKind,
                DiscoverMethods(type)));
        }

        grains.Sort((a, b) => string.CompareOrdinal(a.InterfaceName, b.InterfaceName));
        return grains;
    }

    /// <summary>Map a discovered method to a manifest descriptor.</summary>
    public static GrainMethodDescriptor ToDescriptor(DiscoveredMethod method) =>
        new(method.Name, TypeName(method.RequestType), TypeName(method.ReturnType), "json")
        {
            Parameters = method.Parameters
                .Select(p => new MethodParameterDescriptor(p.Name, TypeName(p.Type)))
                .ToList(),
        };

    /// <summary>The .NET type name used in manifests, or empty for void/none.</summary>
    public static string TypeName(Type? type) => type is null ? string.Empty : type.FullName ?? type.Name;

    private static bool IsMarker(Type type) =>
        type == typeof(IGrain)
        || type == typeof(IAddressable)
        || (type.Namespace == "Orleans" && type.Name.StartsWith("IGrainWith", StringComparison.Ordinal));

    private static string? KeyKindFor(Type type)
    {
        if (typeof(IGrainWithStringKey).IsAssignableFrom(type))
        {
            return "string";
        }

        if (typeof(IGrainWithIntegerKey).IsAssignableFrom(type))
        {
            return "int64";
        }

        if (typeof(IGrainWithGuidKey).IsAssignableFrom(type))
        {
            return "guid";
        }

        return null;
    }

    private static string GrainTypeFor(Type type)
    {
        if (type.GetCustomAttribute<BridgeGrainAttribute>() is { } attribute)
        {
            return attribute.GrainType;
        }

        var name = type.Name;
        if (name.Length > 1 && name[0] == 'I' && char.IsUpper(name[1]))
        {
            name = name[1..];
        }

        if (name.EndsWith("Grain", StringComparison.Ordinal))
        {
            name = name[..^"Grain".Length];
        }

        return name.ToLowerInvariant();
    }

    private static IReadOnlyList<DiscoveredMethod> DiscoverMethods(Type type)
    {
        var methods = new List<DiscoveredMethod>();
        foreach (var iface in InterfaceClosure(type))
        {
            foreach (var method in iface.GetMethods())
            {
                var parameters = method.GetParameters();
                var request = parameters.Length >= 1 ? parameters[0].ParameterType : null;
                var paramList = parameters
                    .Select(p => (p.Name ?? "arg", p.ParameterType))
                    .ToList();
                methods.Add(new DiscoveredMethod(
                    method.Name, request, UnwrapTask(method.ReturnType), paramList));
            }
        }

        return methods;
    }

    private static IEnumerable<Type> InterfaceClosure(Type type)
    {
        yield return type;
        foreach (var iface in type.GetInterfaces())
        {
            if (typeof(IGrain).IsAssignableFrom(iface) && !IsMarker(iface))
            {
                yield return iface;
            }
        }
    }

    private static Type? UnwrapTask(Type returnType)
    {
        if (returnType == typeof(Task) || returnType == typeof(ValueTask))
        {
            return null;
        }

        if (returnType.IsGenericType)
        {
            var definition = returnType.GetGenericTypeDefinition();
            if (definition == typeof(Task<>) || definition == typeof(ValueTask<>))
            {
                return returnType.GetGenericArguments()[0];
            }
        }

        return returnType;
    }
}
