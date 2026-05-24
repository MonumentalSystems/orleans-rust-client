using System.Text.Json;

namespace OrleansRustBridge.Abstractions;

/// <summary>
/// A single grain invocation handed to an <see cref="IBridgeGrainInvoker"/>.
/// Carries the decoded key, the method name, the opaque request payload, and
/// the codec to use for (de)serialization. One instance is created per call, so
/// it is safe to use from a singleton invoker.
/// </summary>
public sealed class BridgeInvocation
{
    private static readonly JsonSerializerOptions ArgumentOptions = new(JsonSerializerDefaults.Web);

    /// <summary>Create an invocation context.</summary>
    public BridgeInvocation(BridgeGrainKey key, string method, ReadOnlyMemory<byte> payload, IPayloadCodec codec)
    {
        Key = key;
        Method = method;
        Payload = payload;
        Codec = codec;
    }

    /// <summary>The target grain key.</summary>
    public BridgeGrainKey Key { get; }

    /// <summary>The method being invoked.</summary>
    public string Method { get; }

    /// <summary>The opaque, codec-encoded request payload.</summary>
    public ReadOnlyMemory<byte> Payload { get; }

    /// <summary>The codec for this call.</summary>
    public IPayloadCodec Codec { get; }

    /// <summary>
    /// Decode the request payload as <typeparamref name="T"/>. Decoding
    /// failures surface to the client as <see cref="BridgeErrorCodes.InvalidPayload"/>.
    /// </summary>
    public T DecodeRequest<T>()
    {
        try
        {
            return Codec.Decode<T>(Payload);
        }
        catch (Exception ex) when (ex is not BridgeException)
        {
            throw BridgeException.InvalidPayload(
                $"could not decode request for '{Method}' as {typeof(T).Name}", ex);
        }
    }

    /// <summary>
    /// Decode the argument at <paramref name="index"/> from a multi-argument
    /// request. Multi-argument payloads are a JSON array of positional
    /// arguments (as produced by the generated multi-arg client methods).
    /// </summary>
    public T DecodeArgument<T>(int index)
    {
        try
        {
            using var document = System.Text.Json.JsonDocument.Parse(Payload);
            var root = document.RootElement;
            if (root.ValueKind != System.Text.Json.JsonValueKind.Array || index >= root.GetArrayLength())
            {
                throw BridgeException.InvalidPayload(
                    $"expected a JSON array argument at index {index} for '{Method}'");
            }

            return root[index].Deserialize<T>(ArgumentOptions)!;
        }
        catch (Exception ex) when (ex is not BridgeException)
        {
            throw BridgeException.InvalidPayload(
                $"could not decode argument {index} for '{Method}' as {typeof(T).Name}", ex);
        }
    }

    /// <summary>Encode a response value.</summary>
    public byte[] Encode<T>(T value) => Codec.Encode(value);

    /// <summary>Encode a unit/void response (e.g. JSON <c>null</c>).</summary>
    public byte[] EncodeUnit() => Codec.Encode<object?>(null);
}
