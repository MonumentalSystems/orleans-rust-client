using Orleans.Bridge.V1;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class JsonPayloadCodecTests
{
    private readonly JsonPayloadCodec _codec = new();

    [Fact]
    public void RoundTripsPrimitives()
    {
        var encoded = _codec.Encode(42L);
        Assert.Equal(42L, _codec.Decode<long>(encoded));
        Assert.Equal("42", System.Text.Encoding.UTF8.GetString(encoded));
    }

    [Fact]
    public void EncodesUnitAsNull()
    {
        var encoded = _codec.Encode<object?>(null);
        Assert.Equal("null", System.Text.Encoding.UTF8.GetString(encoded));
    }

    [Fact]
    public void DecodesEmptyPayloadToDefault()
    {
        Assert.Equal(0L, _codec.Decode<long>(ReadOnlyMemory<byte>.Empty));
    }

    [Fact]
    public void MalformedPayloadThrows()
    {
        var bytes = "not-json"u8.ToArray();
        Assert.ThrowsAny<Exception>(() => _codec.Decode<long>(bytes));
    }
}

public class ProtobufPayloadCodecTests
{
    private readonly ProtobufPayloadCodec _codec = new();

    [Fact]
    public void RoundTripsProtobufMessages()
    {
        var message = new HealthResponse { Status = "healthy", ServiceId = "svc" };
        var encoded = _codec.Encode(message);
        var decoded = _codec.Decode<HealthResponse>(encoded);
        Assert.Equal("healthy", decoded.Status);
        Assert.Equal("svc", decoded.ServiceId);
    }

    [Fact]
    public void RejectsNonMessageTypes()
    {
        Assert.Throws<BridgeException>(() => _codec.Decode<long>(new byte[] { 1, 2, 3 }));
        Assert.Throws<BridgeException>(() => _codec.Encode(5L));
    }
}

public class PayloadCodecRegistryTests
{
    private static PayloadCodecRegistry Registry() =>
        new(new IPayloadCodec[] { new JsonPayloadCodec(), new ProtobufPayloadCodec() });

    [Fact]
    public void ResolvesByName()
    {
        Assert.Equal("json", Registry().Resolve("json").Name);
        Assert.Equal("protobuf", Registry().Resolve("protobuf").Name);
    }

    [Fact]
    public void EmptyNameDefaultsToJson()
    {
        Assert.Equal("json", Registry().Resolve(null).Name);
        Assert.Equal("json", Registry().Resolve(string.Empty).Name);
    }

    [Fact]
    public void UnknownCodecThrowsBridgeError()
    {
        var ex = Assert.Throws<BridgeException>(() => Registry().Resolve("xml"));
        Assert.Equal(BridgeErrorCodes.InvalidPayload, ex.Code);
    }

    [Fact]
    public void EmptyRegistryIsRejected()
    {
        Assert.Throws<InvalidOperationException>(() => new PayloadCodecRegistry(Array.Empty<IPayloadCodec>()));
    }
}
