using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>
/// Resolves the <see cref="IBridgeGrainInvoker"/> for a requested grain
/// interface. Built from all invokers registered in DI.
/// </summary>
public sealed class GrainInvokerRegistry
{
    private readonly Dictionary<string, IBridgeGrainInvoker> _byInterface;

    /// <summary>Build the registry from the registered invokers.</summary>
    public GrainInvokerRegistry(IEnumerable<IBridgeGrainInvoker> invokers)
    {
        _byInterface = new Dictionary<string, IBridgeGrainInvoker>(StringComparer.Ordinal);
        foreach (var invoker in invokers)
        {
            if (!_byInterface.TryAdd(invoker.InterfaceName, invoker))
            {
                throw new InvalidOperationException(
                    $"duplicate bridge invoker registered for interface '{invoker.InterfaceName}'");
            }
        }
    }

    /// <summary>All registered invokers.</summary>
    public IReadOnlyCollection<IBridgeGrainInvoker> All => _byInterface.Values;

    /// <summary>
    /// Resolve the invoker for an interface.
    /// </summary>
    /// <exception cref="BridgeException">If no invoker is registered.</exception>
    public IBridgeGrainInvoker Resolve(string interfaceName, string grainType)
    {
        if (_byInterface.TryGetValue(interfaceName, out var invoker))
        {
            return invoker;
        }

        throw BridgeException.UnknownGrain(interfaceName, grainType);
    }
}
