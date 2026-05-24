using Google.Protobuf;

using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>
/// Optional protobuf payload codec for <see cref="IMessage"/> types. Payload
/// bytes are opaque to the bridge; the invoker chooses the concrete message
/// type. Not enabled by default (see <see cref="BridgeOptions.PayloadCodecs"/>).
/// </summary>
public sealed class ProtobufPayloadCodec : IPayloadCodec
{
    /// <inheritdoc />
    public string Name => "protobuf";

    /// <inheritdoc />
    public T Decode<T>(ReadOnlyMemory<byte> payload)
    {
        if (!typeof(IMessage).IsAssignableFrom(typeof(T)))
        {
            throw new BridgeException(
                BridgeErrorCodes.InvalidPayload,
                $"protobuf codec can only decode Google.Protobuf.IMessage types, not {typeof(T).Name}");
        }

        var parser = (MessageParser?)typeof(T)
            .GetProperty("Parser", System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)
            ?.GetValue(null);
        if (parser is null)
        {
            throw new BridgeException(
                BridgeErrorCodes.InvalidPayload,
                $"protobuf type {typeof(T).Name} has no static Parser property");
        }

        return (T)parser.ParseFrom(payload.ToArray());
    }

    /// <inheritdoc />
    public byte[] Encode<T>(T value)
    {
        if (value is IMessage message)
        {
            return message.ToByteArray();
        }

        throw new BridgeException(
            BridgeErrorCodes.SerializationError,
            $"protobuf codec can only encode Google.Protobuf.IMessage values, not {typeof(T).Name}");
    }
}
