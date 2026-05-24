using Orleans;

namespace OrleansRustBridge.Abstractions;

/// <summary>Describes a single dispatchable grain method.</summary>
/// <param name="Name">Method name as exposed on the grain interface.</param>
/// <param name="RequestType">First-argument .NET type name, or empty for none.</param>
/// <param name="ResponseType">Response .NET type name, or empty for void.</param>
/// <param name="PayloadCodec">Codec used for this method's payloads.</param>
public sealed record BridgeMethodDescriptor(
    string Name,
    string RequestType,
    string ResponseType,
    string PayloadCodec = "json")
{
    /// <summary>Full parameter list (enables multi-argument methods).</summary>
    public IReadOnlyList<MethodParameterDescriptor> Parameters { get; init; } = [];
}

/// <summary>
/// Dispatches a generic bridge invocation to a strongly typed Orleans grain
/// call. This is the production-recommended path (the brief's "Mode A"): one
/// invoker per grain interface keeps Orleans method dispatch type-safe.
/// </summary>
public interface IBridgeGrainInvoker
{
    /// <summary>The fully-qualified grain interface name this invoker serves.</summary>
    string InterfaceName { get; }

    /// <summary>The grain type alias used for dispatch.</summary>
    string GrainType { get; }

    /// <summary>The methods this invoker can dispatch.</summary>
    IReadOnlyList<BridgeMethodDescriptor> Methods { get; }

    /// <summary>The key kinds this grain supports (<c>string</c>, <c>int64</c>, <c>guid</c>).</summary>
    IReadOnlyList<string> SupportedKeyKinds { get; }

    /// <summary>
    /// Resolve the grain for <paramref name="invocation"/>, call the requested
    /// method, and return the encoded response.
    /// </summary>
    /// <exception cref="BridgeException">For unknown methods or invalid payloads.</exception>
    Task<byte[]> InvokeAsync(
        IClusterClient client,
        BridgeInvocation invocation,
        CancellationToken cancellationToken);
}
