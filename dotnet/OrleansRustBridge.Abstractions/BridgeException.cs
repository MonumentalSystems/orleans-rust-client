namespace OrleansRustBridge.Abstractions;

/// <summary>
/// An exception carrying a stable bridge error code. Grain invokers throw these
/// for predictable, client-visible failures; the bridge transport maps them to
/// a gRPC status plus a structured error trailer.
/// </summary>
public sealed class BridgeException : Exception
{
    /// <summary>Stable error code; see <see cref="BridgeErrorCodes"/>.</summary>
    public string Code { get; }

    /// <summary>Whether the caller may safely retry.</summary>
    public bool Retryable { get; }

    /// <summary>Optional extra detail (suppressed unless dev mode is enabled).</summary>
    public string? Detail { get; }

    /// <summary>Create a bridge exception.</summary>
    public BridgeException(string code, string message, bool retryable = false, string? detail = null, Exception? inner = null)
        : base(message, inner)
    {
        Code = code;
        Retryable = retryable;
        Detail = detail;
    }

    /// <summary>The target interface/grain type is not registered.</summary>
    public static BridgeException UnknownGrain(string interfaceName, string grainType) =>
        new(BridgeErrorCodes.UnknownGrain, $"no invoker registered for interface '{interfaceName}' (grain type '{grainType}')");

    /// <summary>The grain exists but does not expose the requested method.</summary>
    public static BridgeException UnknownMethod(string interfaceName, string method) =>
        new(BridgeErrorCodes.UnknownMethod, $"interface '{interfaceName}' has no method '{method}'");

    /// <summary>The supplied key was missing or of the wrong kind.</summary>
    public static BridgeException InvalidKey(string message) =>
        new(BridgeErrorCodes.InvalidKey, message);

    /// <summary>The request payload could not be decoded.</summary>
    public static BridgeException InvalidPayload(string message, Exception? inner = null) =>
        new(BridgeErrorCodes.InvalidPayload, message, retryable: false, detail: inner?.Message, inner: inner);
}
