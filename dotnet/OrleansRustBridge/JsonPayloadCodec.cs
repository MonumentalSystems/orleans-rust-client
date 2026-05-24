using System.Text.Json;

using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>JSON payload codec built on <see cref="System.Text.Json"/>.</summary>
public sealed class JsonPayloadCodec : IPayloadCodec
{
    private static readonly JsonSerializerOptions Options = new(JsonSerializerDefaults.Web);

    /// <inheritdoc />
    public string Name => "json";

    /// <inheritdoc />
    public T Decode<T>(ReadOnlyMemory<byte> payload)
    {
        // An empty payload decodes to the type's default (e.g. unit calls).
        if (payload.IsEmpty)
        {
            return default!;
        }

        return JsonSerializer.Deserialize<T>(payload.Span, Options)!;
    }

    /// <inheritdoc />
    public byte[] Encode<T>(T value) => JsonSerializer.SerializeToUtf8Bytes(value, Options);
}
