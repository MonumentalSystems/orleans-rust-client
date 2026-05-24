namespace OrleansRustBridge.Abstractions;

/// <summary>
/// Stable, machine-readable error codes returned to clients. These strings are
/// part of the bridge contract and match the codes the Rust client matches on.
/// </summary>
public static class BridgeErrorCodes
{
    /// <summary>The target interface/grain type is not registered.</summary>
    public const string UnknownGrain = "unknown_grain";

    /// <summary>The grain exists but does not expose the requested method.</summary>
    public const string UnknownMethod = "unknown_method";

    /// <summary>The supplied key kind is not valid for the target grain.</summary>
    public const string InvalidKey = "invalid_key";

    /// <summary>The request payload could not be interpreted under its codec.</summary>
    public const string InvalidPayload = "invalid_payload";

    /// <summary>A response value could not be serialized back to the caller.</summary>
    public const string SerializationError = "serialization_error";

    /// <summary>Orleans rejected the message (overload, placement failure...).</summary>
    public const string OrleansRejection = "orleans_rejection";

    /// <summary>The grain call exceeded its deadline.</summary>
    public const string OrleansTimeout = "orleans_timeout";

    /// <summary>The cluster could not be reached.</summary>
    public const string OrleansUnavailable = "orleans_unavailable";

    /// <summary>The grain method threw an application exception.</summary>
    public const string ApplicationError = "application_error";

    /// <summary>The call was cancelled before completion.</summary>
    public const string Cancelled = "cancelled";

    /// <summary>An unexpected bridge-internal failure.</summary>
    public const string Internal = "internal";
}
