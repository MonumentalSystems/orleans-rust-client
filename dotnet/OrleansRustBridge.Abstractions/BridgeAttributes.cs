namespace OrleansRustBridge.Abstractions;

/// <summary>
/// Marks a grain interface for bridge exposure. <c>OrleansRustBridge.Tools</c>
/// uses this attribute to decide which interfaces to include when generating
/// invokers and manifest entries. Applying it is optional: the tools can also
/// be pointed at interfaces explicitly.
/// </summary>
[AttributeUsage(AttributeTargets.Interface, AllowMultiple = false, Inherited = false)]
public sealed class BridgeGrainAttribute : Attribute
{
    /// <summary>Create the attribute with an explicit grain type alias.</summary>
    public BridgeGrainAttribute(string grainType)
    {
        GrainType = grainType;
    }

    /// <summary>The grain type alias used for dispatch.</summary>
    public string GrainType { get; }

    /// <summary>The payload codec to use for this grain's methods.</summary>
    public string PayloadCodec { get; init; } = "json";
}

/// <summary>
/// Overrides the exposed name of a grain method. Optional; by default the
/// method's own name is used.
/// </summary>
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class BridgeMethodAttribute : Attribute
{
    /// <summary>Create the attribute with an explicit exposed name.</summary>
    public BridgeMethodAttribute(string name)
    {
        Name = name;
    }

    /// <summary>The exposed method name.</summary>
    public string Name { get; }
}
