using Microsoft.Extensions.DependencyInjection;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class ServiceCollectionExtensionsTests
{
    private static ServiceProvider Build(Action<BridgeOptions>? configure)
    {
        var services = new ServiceCollection();
        services.AddSingleton<IBridgeGrainInvoker>(new FakeInvoker("IFoo", "foo"));
        services.AddOrleansRustBridge(configure);
        return services.BuildServiceProvider();
    }

    [Fact]
    public void RegistersOptionsCodecsAndRegistry()
    {
        using var provider = Build(options =>
        {
            options.ServiceId = "svc";
            options.PayloadCodecs = ["json", "protobuf"];
        });

        var options = provider.GetRequiredService<BridgeOptions>();
        Assert.Equal("svc", options.ServiceId);

        var codecs = provider.GetRequiredService<IPayloadCodecRegistry>();
        Assert.Equal("json", codecs.Resolve("json").Name);
        Assert.Equal("protobuf", codecs.Resolve("protobuf").Name);

        var registry = provider.GetRequiredService<GrainInvokerRegistry>();
        Assert.Equal("foo", registry.Resolve("IFoo", "foo").GrainType);
    }

    [Fact]
    public void DefaultsToJsonOnlyCodec()
    {
        using var provider = Build(configure: null);
        var codecs = provider.GetRequiredService<IPayloadCodecRegistry>();
        Assert.Contains("json", codecs.CodecNames);
        Assert.DoesNotContain("protobuf", codecs.CodecNames);
    }

    [Fact]
    public void UnknownConfiguredCodecIsRejected()
    {
        using var provider = Build(options => options.PayloadCodecs = ["xml"]);
        Assert.Throws<InvalidOperationException>(() => provider.GetRequiredService<IPayloadCodecRegistry>());
    }
}
