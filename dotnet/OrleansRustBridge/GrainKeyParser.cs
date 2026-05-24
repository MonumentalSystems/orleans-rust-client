using Orleans.Bridge.V1;

using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>Translates protocol grain keys into <see cref="BridgeGrainKey"/>.</summary>
public static class GrainKeyParser
{
    /// <summary>
    /// Parse <paramref name="key"/>, validating it against the key kinds the
    /// target grain supports.
    /// </summary>
    /// <exception cref="BridgeException">If the key is missing or unsupported.</exception>
    public static BridgeGrainKey Parse(GrainKey? key, IReadOnlyList<string> supportedKinds)
    {
        if (key is null)
        {
            throw BridgeException.InvalidKey("request is missing a grain key");
        }

        return key.KindCase switch
        {
            GrainKey.KindOneofCase.StringKey => Validate("string", supportedKinds, BridgeGrainKey.FromString(key.StringKey)),
            GrainKey.KindOneofCase.Int64Key => Validate("int64", supportedKinds, BridgeGrainKey.FromInt64(key.Int64Key)),
            GrainKey.KindOneofCase.GuidKey => Validate("guid", supportedKinds, ParseGuid(key.GuidKey)),
            _ => throw BridgeException.InvalidKey("grain key has no value set"),
        };
    }

    private static BridgeGrainKey Validate(string kind, IReadOnlyList<string> supportedKinds, BridgeGrainKey parsed)
    {
        if (supportedKinds.Count > 0 && !supportedKinds.Contains(kind, StringComparer.OrdinalIgnoreCase))
        {
            throw BridgeException.InvalidKey(
                $"grain does not support {kind} keys (supported: {string.Join(", ", supportedKinds)})");
        }

        return parsed;
    }

    private static BridgeGrainKey ParseGuid(string value)
    {
        if (!Guid.TryParse(value, out var guid))
        {
            throw BridgeException.InvalidKey($"'{value}' is not a valid GUID");
        }

        return BridgeGrainKey.FromGuid(guid);
    }
}
