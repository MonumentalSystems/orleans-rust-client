using Counter.Abstractions;

using Orleans;

using OrleansRustBridge.Abstractions;

namespace Counter.Bridge;

/// <summary>Invoker for the integer-keyed <see cref="IAccumulatorGrain"/>.</summary>
public sealed class AccumulatorGrainInvoker : IBridgeGrainInvoker
{
    private static readonly IReadOnlyList<string> Keys = ["int64"];

    private static readonly IReadOnlyList<BridgeMethodDescriptor> MethodList =
    [
        new BridgeMethodDescriptor("Add", "System.Int64", "System.Int64"),
        new BridgeMethodDescriptor("Get", string.Empty, "System.Int64"),
    ];

    /// <inheritdoc />
    public string InterfaceName => "Counter.Abstractions.IAccumulatorGrain";

    /// <inheritdoc />
    public string GrainType => "accumulator";

    /// <inheritdoc />
    public IReadOnlyList<BridgeMethodDescriptor> Methods => MethodList;

    /// <inheritdoc />
    public IReadOnlyList<string> SupportedKeyKinds => Keys;

    /// <inheritdoc />
    public async Task<byte[]> InvokeAsync(
        IClusterClient client,
        BridgeInvocation invocation,
        CancellationToken cancellationToken)
    {
        var grain = client.GetGrain<IAccumulatorGrain>(invocation.Key.AsInt64());
        return invocation.Method switch
        {
            "Add" => invocation.Encode(await grain.Add(invocation.DecodeRequest<long>())),
            "Get" => invocation.Encode(await grain.Get()),
            _ => throw BridgeException.UnknownMethod(InterfaceName, invocation.Method),
        };
    }
}

/// <summary>Invoker for the GUID-keyed <see cref="IRegisterGrain"/>.</summary>
public sealed class RegisterGrainInvoker : IBridgeGrainInvoker
{
    private static readonly IReadOnlyList<string> Keys = ["guid"];

    private static readonly IReadOnlyList<BridgeMethodDescriptor> MethodList =
    [
        new BridgeMethodDescriptor("Set", "System.String", "System.String"),
        new BridgeMethodDescriptor("Get", string.Empty, "System.String"),
    ];

    /// <inheritdoc />
    public string InterfaceName => "Counter.Abstractions.IRegisterGrain";

    /// <inheritdoc />
    public string GrainType => "register";

    /// <inheritdoc />
    public IReadOnlyList<BridgeMethodDescriptor> Methods => MethodList;

    /// <inheritdoc />
    public IReadOnlyList<string> SupportedKeyKinds => Keys;

    /// <inheritdoc />
    public async Task<byte[]> InvokeAsync(
        IClusterClient client,
        BridgeInvocation invocation,
        CancellationToken cancellationToken)
    {
        var grain = client.GetGrain<IRegisterGrain>(invocation.Key.AsGuid());
        return invocation.Method switch
        {
            "Set" => invocation.Encode(await grain.Set(invocation.DecodeRequest<string>())),
            "Get" => invocation.Encode(await grain.Get()),
            _ => throw BridgeException.UnknownMethod(InterfaceName, invocation.Method),
        };
    }
}
