using System.Text;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class BridgeInvocationTests
{
    private static readonly JsonPayloadCodec Json = new();

    private static BridgeInvocation Invocation(string method, string payloadJson) =>
        new(BridgeGrainKey.FromString("k"), method, Encoding.UTF8.GetBytes(payloadJson), Json);

    [Fact]
    public void ExposesCallMetadata()
    {
        var invocation = Invocation("Add", "5");
        Assert.Equal("Add", invocation.Method);
        Assert.Equal(BridgeGrainKeyKind.String, invocation.Key.Kind);
        Assert.Equal("json", invocation.Codec.Name);
    }

    [Fact]
    public void DecodesAndEncodes()
    {
        Assert.Equal(5L, Invocation("Add", "5").DecodeRequest<long>());
        Assert.Equal("5", Encoding.UTF8.GetString(Invocation("Add", "5").Encode(5L)));
        Assert.Equal("null", Encoding.UTF8.GetString(Invocation("Reset", "null").EncodeUnit()));
    }

    [Fact]
    public void MalformedRequestThrowsInvalidPayload()
    {
        var ex = Assert.Throws<BridgeException>(() => Invocation("Add", "not-json").DecodeRequest<long>());
        Assert.Equal(BridgeErrorCodes.InvalidPayload, ex.Code);
    }

    [Fact]
    public void DecodesPositionalArguments()
    {
        var invocation = Invocation("Adjust", "[7, \"hi\", true]");
        Assert.Equal(7L, invocation.DecodeArgument<long>(0));
        Assert.Equal("hi", invocation.DecodeArgument<string>(1));
        Assert.True(invocation.DecodeArgument<bool>(2));
    }

    [Fact]
    public void NonArrayArgumentThrowsInvalidPayload()
    {
        var ex = Assert.Throws<BridgeException>(() => Invocation("Adjust", "5").DecodeArgument<long>(0));
        Assert.Equal(BridgeErrorCodes.InvalidPayload, ex.Code);
    }

    [Fact]
    public void OutOfRangeArgumentThrowsInvalidPayload()
    {
        var ex = Assert.Throws<BridgeException>(() => Invocation("Adjust", "[1]").DecodeArgument<long>(3));
        Assert.Equal(BridgeErrorCodes.InvalidPayload, ex.Code);
    }
}
