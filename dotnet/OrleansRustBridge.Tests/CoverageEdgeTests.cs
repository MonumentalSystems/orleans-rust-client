using System.Reflection;
using System.Text;

using Google.Protobuf;
using Google.Protobuf.Reflection;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

// A type that implements IMessage but exposes no static Parser property, to
// exercise the protobuf codec's defensive branch.
internal sealed class ParserlessMessage : IMessage
{
    public MessageDescriptor Descriptor => throw new NotSupportedException();
    public int CalculateSize() => 0;
    public void MergeFrom(CodedInputStream input) { }
    public void WriteTo(CodedOutputStream output) { }
}

public class CoverageEdgeTests
{
    [Fact]
    public void DecodeRequestRethrowsCodecBridgeException()
    {
        // The protobuf codec rejects non-IMessage types with a BridgeException;
        // BridgeInvocation must surface it as-is, not wrap it.
        var invocation = new BridgeInvocation(
            BridgeGrainKey.FromString("k"), "m", Encoding.UTF8.GetBytes("0"), new ProtobufPayloadCodec());
        var ex = Assert.Throws<BridgeException>(() => invocation.DecodeRequest<long>());
        Assert.Equal(BridgeErrorCodes.InvalidPayload, ex.Code);
    }

    [Fact]
    public void ProtobufCodecRejectsTypeWithoutParser()
    {
        var codec = new ProtobufPayloadCodec();
        var ex = Assert.Throws<BridgeException>(() => codec.Decode<ParserlessMessage>(new byte[] { 1 }));
        Assert.Equal(BridgeErrorCodes.InvalidPayload, ex.Code);
        Assert.Contains("Parser", ex.Message);
    }

    [Fact]
    public void GrainKeyToStringHandlesUnknownKind()
    {
        // Construct via the private ctor with an out-of-range kind to exercise
        // the defensive default in ToString.
        var ctor = typeof(BridgeGrainKey)
            .GetConstructors(BindingFlags.NonPublic | BindingFlags.Instance)
            .Single();
        var key = (BridgeGrainKey)ctor.Invoke([(BridgeGrainKeyKind)99, null, 0L, Guid.Empty]);
        Assert.Equal(string.Empty, key.ToString());
    }
}
