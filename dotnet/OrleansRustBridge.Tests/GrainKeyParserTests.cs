using Orleans.Bridge.V1;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class GrainKeyParserTests
{
    private static readonly string[] StringOnly = ["string"];
    private static readonly string[] AllKinds = ["string", "int64", "guid"];

    [Fact]
    public void ParsesStringKey()
    {
        var key = GrainKeyParser.Parse(new GrainKey { StringKey = "abc" }, StringOnly);
        Assert.Equal(BridgeGrainKeyKind.String, key.Kind);
        Assert.Equal("abc", key.AsString());
    }

    [Fact]
    public void ParsesInt64Key()
    {
        var key = GrainKeyParser.Parse(new GrainKey { Int64Key = 7 }, AllKinds);
        Assert.Equal(7, key.AsInt64());
    }

    [Fact]
    public void ParsesGuidKey()
    {
        var guid = Guid.NewGuid();
        var key = GrainKeyParser.Parse(new GrainKey { GuidKey = guid.ToString() }, AllKinds);
        Assert.Equal(guid, key.AsGuid());
    }

    [Fact]
    public void RejectsUnsupportedKind()
    {
        var ex = Assert.Throws<BridgeException>(() =>
            GrainKeyParser.Parse(new GrainKey { Int64Key = 1 }, StringOnly));
        Assert.Equal(BridgeErrorCodes.InvalidKey, ex.Code);
    }

    [Fact]
    public void RejectsMissingKey()
    {
        Assert.Throws<BridgeException>(() => GrainKeyParser.Parse(null, AllKinds));
        Assert.Throws<BridgeException>(() => GrainKeyParser.Parse(new GrainKey(), AllKinds));
    }

    [Fact]
    public void RejectsMalformedGuid()
    {
        var ex = Assert.Throws<BridgeException>(() =>
            GrainKeyParser.Parse(new GrainKey { GuidKey = "not-a-guid" }, AllKinds));
        Assert.Equal(BridgeErrorCodes.InvalidKey, ex.Code);
    }
}
