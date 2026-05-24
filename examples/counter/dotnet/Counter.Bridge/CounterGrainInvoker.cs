using Counter.Abstractions;

using Orleans;

using OrleansRustBridge.Abstractions;

namespace Counter.Bridge;

/// <summary>
/// Hand-written bridge invoker for <see cref="ICounterGrain"/> (the brief's
/// "Mode A"). Keeps Orleans method dispatch strongly typed on the C# side.
/// </summary>
public sealed class CounterGrainInvoker : IBridgeGrainInvoker
{
    private static readonly IReadOnlyList<string> Keys = new[] { "string" };

    private static readonly IReadOnlyList<BridgeMethodDescriptor> MethodList = new[]
    {
        new BridgeMethodDescriptor("Get", string.Empty, "System.Int64"),
        new BridgeMethodDescriptor("Add", "System.Int64", "System.Int64"),
        new BridgeMethodDescriptor("Reset", string.Empty, string.Empty),
        new BridgeMethodDescriptor("Delay", "System.Int32", "System.Int64"),
        new BridgeMethodDescriptor("WhoCalled", string.Empty, "System.String"),
    };

    /// <inheritdoc />
    public string InterfaceName => "Counter.Abstractions.ICounterGrain";

    /// <inheritdoc />
    public string GrainType => "counter";

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
        var grain = client.GetGrain<ICounterGrain>(invocation.Key.AsString());

        switch (invocation.Method)
        {
            case "Get":
                return invocation.Encode(await grain.Get());
            case "Add":
                return invocation.Encode(await grain.Add(invocation.DecodeRequest<long>()));
            case "Reset":
                await grain.Reset();
                return invocation.EncodeUnit();
            case "Delay":
                return invocation.Encode(await grain.Delay(invocation.DecodeRequest<int>()));
            case "WhoCalled":
                return invocation.Encode(await grain.WhoCalled());
            default:
                throw BridgeException.UnknownMethod(InterfaceName, invocation.Method);
        }
    }
}
