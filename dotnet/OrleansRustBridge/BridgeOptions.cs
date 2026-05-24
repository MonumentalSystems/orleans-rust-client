namespace OrleansRustBridge;

/// <summary>
/// Strongly typed bridge configuration. Bound from the
/// <c>OrleansRustBridge</c> configuration section and/or set in code.
/// </summary>
public sealed class BridgeOptions
{
    /// <summary>Configuration section name.</summary>
    public const string SectionName = "OrleansRustBridge";

    /// <summary>Orleans service id reported by Health/manifest.</summary>
    public string ServiceId { get; set; } = "orleans-rust-bridge";

    /// <summary>Orleans cluster id reported by Health/manifest.</summary>
    public string ClusterId { get; set; } = "dev";

    /// <summary>Bridge version reported by Health/manifest.</summary>
    public string BridgeVersion { get; set; } = "0.1.0";

    /// <summary>Enabled payload codecs. JSON is enabled by default.</summary>
    public IList<string> PayloadCodecs { get; set; } = new List<string> { "json" };

    /// <summary>Default per-call deadline when the caller does not specify one.</summary>
    public int DefaultTimeoutMs { get; set; } = 30_000;

    /// <summary>
    /// When <c>true</c>, include exception detail (type and message) in error
    /// responses. Leave <c>false</c> outside development to avoid leaking
    /// internals.
    /// </summary>
    public bool IncludeExceptionDetail { get; set; }

    /// <summary>
    /// Request-context keys to copy from Orleans' <c>RequestContext</c> back to
    /// the caller as response-context entries after each call.
    /// </summary>
    public IList<string> PropagatedResponseContextKeys { get; set; } = new List<string>();
}
