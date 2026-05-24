using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class BridgeGrainKeyTests
{
    [Fact]
    public void StringKeyRoundTrips()
    {
        var key = BridgeGrainKey.FromString("abc");
        Assert.Equal(BridgeGrainKeyKind.String, key.Kind);
        Assert.Equal("abc", key.AsString());
        Assert.Equal("abc", key.ToString());
    }

    [Fact]
    public void Int64KeyRoundTrips()
    {
        var key = BridgeGrainKey.FromInt64(42);
        Assert.Equal(BridgeGrainKeyKind.Int64, key.Kind);
        Assert.Equal(42, key.AsInt64());
        Assert.Equal("42", key.ToString());
    }

    [Fact]
    public void GuidKeyRoundTrips()
    {
        var guid = Guid.NewGuid();
        var key = BridgeGrainKey.FromGuid(guid);
        Assert.Equal(BridgeGrainKeyKind.Guid, key.Kind);
        Assert.Equal(guid, key.AsGuid());
        Assert.Equal(guid.ToString(), key.ToString());
    }

    [Fact]
    public void WrongKindAccessorsThrowInvalidKey()
    {
        var stringKey = BridgeGrainKey.FromString("k");
        Assert.Equal(BridgeErrorCodes.InvalidKey, Assert.Throws<BridgeException>(() => stringKey.AsInt64()).Code);
        Assert.Equal(BridgeErrorCodes.InvalidKey, Assert.Throws<BridgeException>(() => stringKey.AsGuid()).Code);

        var intKey = BridgeGrainKey.FromInt64(1);
        Assert.Throws<BridgeException>(() => intKey.AsString());
        Assert.Throws<BridgeException>(() => intKey.AsGuid());

        var guidKey = BridgeGrainKey.FromGuid(Guid.Empty);
        Assert.Throws<BridgeException>(() => guidKey.AsString());
        Assert.Throws<BridgeException>(() => guidKey.AsInt64());
    }

    [Fact]
    public void BridgeExceptionFactoriesCarryCodes()
    {
        Assert.Equal(BridgeErrorCodes.UnknownGrain, BridgeException.UnknownGrain("IFoo", "foo").Code);
        Assert.Equal(BridgeErrorCodes.UnknownMethod, BridgeException.UnknownMethod("IFoo", "Bar").Code);
        Assert.Equal(BridgeErrorCodes.InvalidKey, BridgeException.InvalidKey("nope").Code);

        var invalid = BridgeException.InvalidPayload("bad", new FormatException("inner"));
        Assert.Equal(BridgeErrorCodes.InvalidPayload, invalid.Code);
        Assert.Equal("inner", invalid.Detail);
        Assert.IsType<FormatException>(invalid.InnerException);
    }
}
