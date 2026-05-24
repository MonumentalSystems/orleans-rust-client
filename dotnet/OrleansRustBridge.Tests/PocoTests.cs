using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class BridgeOptionsTests
{
    [Fact]
    public void HasSensibleDefaults()
    {
        var options = new BridgeOptions();
        Assert.Equal("OrleansRustBridge", BridgeOptions.SectionName);
        Assert.Equal("orleans-rust-bridge", options.ServiceId);
        Assert.Equal("dev", options.ClusterId);
        Assert.Equal("0.1.0", options.BridgeVersion);
        Assert.Equal(30_000, options.DefaultTimeoutMs);
        Assert.False(options.IncludeExceptionDetail);
        Assert.Contains("json", options.PayloadCodecs);
        Assert.Empty(options.PropagatedResponseContextKeys);
    }

    [Fact]
    public void IsMutable()
    {
        var options = new BridgeOptions
        {
            ServiceId = "svc",
            ClusterId = "prod",
            BridgeVersion = "1.2.3",
            DefaultTimeoutMs = 5_000,
            IncludeExceptionDetail = true,
            PayloadCodecs = ["json", "protobuf"],
            PropagatedResponseContextKeys = ["trace-id"],
        };
        Assert.Equal("svc", options.ServiceId);
        Assert.Equal("prod", options.ClusterId);
        Assert.Equal("1.2.3", options.BridgeVersion);
        Assert.Equal(5_000, options.DefaultTimeoutMs);
        Assert.True(options.IncludeExceptionDetail);
        Assert.Equal(2, options.PayloadCodecs.Count);
        Assert.Contains("trace-id", options.PropagatedResponseContextKeys);
    }
}

public class BridgeAttributesTests
{
    [BridgeGrain("widget", PayloadCodec = "protobuf")]
    private interface IAnnotatedGrain
    {
        [BridgeMethod("Renamed")]
        void Original();
    }

    [Fact]
    public void GrainAttributeCarriesMetadata()
    {
        var attribute = typeof(IAnnotatedGrain).GetCustomAttributes(typeof(BridgeGrainAttribute), false);
        var grain = Assert.IsType<BridgeGrainAttribute>(attribute.Single());
        Assert.Equal("widget", grain.GrainType);
        Assert.Equal("protobuf", grain.PayloadCodec);
    }

    [Fact]
    public void GrainAttributeDefaultsToJson()
    {
        Assert.Equal("json", new BridgeGrainAttribute("g").PayloadCodec);
    }

    [Fact]
    public void MethodAttributeCarriesName()
    {
        Assert.Equal("Renamed", new BridgeMethodAttribute("Renamed").Name);
    }
}

public class DescriptorTests
{
    [Fact]
    public void MethodDescriptorDefaultsAndParameters()
    {
        var descriptor = new BridgeMethodDescriptor("Add", "System.Int64", "System.Int64");
        Assert.Equal("json", descriptor.PayloadCodec);
        Assert.Empty(descriptor.Parameters);

        var withParams = descriptor with
        {
            Parameters = [new MethodParameterDescriptor("value", "System.Int64")],
        };
        Assert.Single(withParams.Parameters);
        Assert.Equal("value", withParams.Parameters[0].Name);
        Assert.Equal("System.Int64", withParams.Parameters[0].Type);
    }

    [Fact]
    public void ContractDescriptorHoldsMethods()
    {
        var contract = new GrainContractDescriptor(
            "IFoo",
            "foo",
            [new GrainMethodDescriptor("Get", string.Empty, "System.Int64", "json")],
            ["string"]);
        Assert.Equal("IFoo", contract.InterfaceName);
        Assert.Equal("foo", contract.GrainType);
        Assert.Single(contract.Methods);
        Assert.Equal("string", contract.SupportedKeyKinds.Single());
    }
}
