using Orleans;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

internal sealed class FakeInvoker(string interfaceName, string grainType) : IBridgeGrainInvoker
{
    public string InterfaceName => interfaceName;
    public string GrainType => grainType;
    public IReadOnlyList<BridgeMethodDescriptor> Methods => [];
    public IReadOnlyList<string> SupportedKeyKinds => ["string"];

    public Task<byte[]> InvokeAsync(IClusterClient client, BridgeInvocation invocation, CancellationToken cancellationToken) =>
        Task.FromResult(Array.Empty<byte>());
}

public class GrainInvokerRegistryTests
{
    [Fact]
    public void ResolvesRegisteredInvoker()
    {
        var registry = new GrainInvokerRegistry([new FakeInvoker("IFoo", "foo")]);
        Assert.Equal("foo", registry.Resolve("IFoo", "foo").GrainType);
    }

    [Fact]
    public void UnknownInterfaceThrowsUnknownGrain()
    {
        var registry = new GrainInvokerRegistry([new FakeInvoker("IFoo", "foo")]);
        var ex = Assert.Throws<BridgeException>(() => registry.Resolve("IBar", "bar"));
        Assert.Equal(BridgeErrorCodes.UnknownGrain, ex.Code);
    }

    [Fact]
    public void DuplicateInterfaceIsRejected()
    {
        Assert.Throws<InvalidOperationException>(() =>
            new GrainInvokerRegistry([new FakeInvoker("IFoo", "foo"), new FakeInvoker("IFoo", "other")]));
    }
}

public class ManifestSerializationTests
{
    [Fact]
    public void SerializesSnakeCaseJson()
    {
        var manifest = new BridgeManifest(
            "svc",
            "dev",
            "0.1.0",
            BridgeManifest.CurrentSchemaVersion,
            [
                new GrainContractDescriptor(
                    "Counter.Abstractions.ICounterGrain",
                    "counter",
                    [new GrainMethodDescriptor("Get", string.Empty, "System.Int64", "json")],
                    ["string"]),
            ]);

        var json = manifest.ToJson();
        Assert.Contains("\"service_id\": \"svc\"", json);
        Assert.Contains("\"interface_name\": \"Counter.Abstractions.ICounterGrain\"", json);
        Assert.Contains("\"supported_key_kinds\"", json);
        Assert.Contains("\"response_type\": \"System.Int64\"", json);
    }
}
