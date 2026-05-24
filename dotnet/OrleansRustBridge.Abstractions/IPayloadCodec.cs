namespace OrleansRustBridge.Abstractions;

/// <summary>
/// Encodes and decodes grain request/response values. Payloads are opaque to
/// the bridge transport; only the invoker and codec understand their shape.
/// </summary>
public interface IPayloadCodec
{
    /// <summary>The codec name as it appears on the wire (e.g. <c>json</c>).</summary>
    string Name { get; }

    /// <summary>Decode <paramref name="payload"/> into a value of type T.</summary>
    T Decode<T>(ReadOnlyMemory<byte> payload);

    /// <summary>Encode <paramref name="value"/> to bytes.</summary>
    byte[] Encode<T>(T value);
}

/// <summary>Resolves a codec by name.</summary>
public interface IPayloadCodecRegistry
{
    /// <summary>The names of all registered codecs.</summary>
    IReadOnlyCollection<string> CodecNames { get; }

    /// <summary>
    /// Resolve a codec by name, falling back to <c>json</c> when
    /// <paramref name="name"/> is empty.
    /// </summary>
    /// <exception cref="BridgeException">If no codec matches.</exception>
    IPayloadCodec Resolve(string? name);
}
