using System.Text.Json;
using System.Text.Json.Serialization;

namespace OrleansRustBridge.Abstractions;

/// <summary>
/// The contract manifest describing every grain the bridge can dispatch to.
/// Serialized to JSON (snake_case) for consumption by <c>orleans-rust-codegen</c>.
/// </summary>
/// <param name="ServiceId">Orleans service id the bridge connects to.</param>
/// <param name="ClusterId">Orleans cluster id the bridge connects to.</param>
/// <param name="BridgeVersion">Bridge version that produced the manifest.</param>
/// <param name="SchemaVersion">Manifest schema version.</param>
/// <param name="Grains">Grain contracts.</param>
public sealed record BridgeManifest(
    string ServiceId,
    string ClusterId,
    string BridgeVersion,
    string SchemaVersion,
    IReadOnlyList<GrainContractDescriptor> Grains)
{
    /// <summary>The current manifest schema version.</summary>
    public const string CurrentSchemaVersion = "1";

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.Never,
    };

    /// <summary>Serialize the manifest to snake_case JSON.</summary>
    public string ToJson() => JsonSerializer.Serialize(this, JsonOptions);
}
