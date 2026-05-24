using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>Resolves payload codecs by name (case-insensitive).</summary>
public sealed class PayloadCodecRegistry : IPayloadCodecRegistry
{
    private readonly Dictionary<string, IPayloadCodec> _codecs;

    /// <summary>Create a registry over the given codecs.</summary>
    public PayloadCodecRegistry(IEnumerable<IPayloadCodec> codecs)
    {
        _codecs = codecs.ToDictionary(codec => codec.Name, StringComparer.OrdinalIgnoreCase);
        if (_codecs.Count == 0)
        {
            throw new InvalidOperationException("at least one payload codec must be enabled");
        }
    }

    /// <inheritdoc />
    public IReadOnlyCollection<string> CodecNames => _codecs.Keys;

    /// <inheritdoc />
    public IPayloadCodec Resolve(string? name)
    {
        var key = string.IsNullOrEmpty(name) ? "json" : name;
        if (_codecs.TryGetValue(key, out var codec))
        {
            return codec;
        }

        throw new BridgeException(
            BridgeErrorCodes.InvalidPayload,
            $"unsupported payload codec '{name}'; enabled: {string.Join(", ", _codecs.Keys)}");
    }
}
