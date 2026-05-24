namespace OrleansRustBridge.Abstractions;

/// <summary>The primitive Orleans key kinds the bridge understands.</summary>
public enum BridgeGrainKeyKind
{
    /// <summary><c>IGrainWithStringKey</c>.</summary>
    String,

    /// <summary><c>IGrainWithIntegerKey</c>.</summary>
    Int64,

    /// <summary><c>IGrainWithGuidKey</c>.</summary>
    Guid,
}

/// <summary>
/// A transport-neutral grain key. The bridge service translates the protocol's
/// key representation into this type before handing it to an invoker, so
/// invoker authors never depend on the wire protocol.
/// </summary>
public sealed class BridgeGrainKey
{
    private readonly string? _stringValue;
    private readonly long _int64Value;
    private readonly Guid _guidValue;

    private BridgeGrainKey(BridgeGrainKeyKind kind, string? stringValue, long int64Value, Guid guidValue)
    {
        Kind = kind;
        _stringValue = stringValue;
        _int64Value = int64Value;
        _guidValue = guidValue;
    }

    /// <summary>The key kind.</summary>
    public BridgeGrainKeyKind Kind { get; }

    /// <summary>Create a string key.</summary>
    public static BridgeGrainKey FromString(string value) =>
        new(BridgeGrainKeyKind.String, value, 0, Guid.Empty);

    /// <summary>Create an integer key.</summary>
    public static BridgeGrainKey FromInt64(long value) =>
        new(BridgeGrainKeyKind.Int64, null, value, Guid.Empty);

    /// <summary>Create a GUID key.</summary>
    public static BridgeGrainKey FromGuid(Guid value) =>
        new(BridgeGrainKeyKind.Guid, null, 0, value);

    /// <summary>The string value. Throws if this is not a string key.</summary>
    public string AsString() =>
        Kind == BridgeGrainKeyKind.String
            ? _stringValue!
            : throw BridgeException.InvalidKey($"expected a string key, got {Kind}");

    /// <summary>The integer value. Throws if this is not an integer key.</summary>
    public long AsInt64() =>
        Kind == BridgeGrainKeyKind.Int64
            ? _int64Value
            : throw BridgeException.InvalidKey($"expected an int64 key, got {Kind}");

    /// <summary>The GUID value. Throws if this is not a GUID key.</summary>
    public Guid AsGuid() =>
        Kind == BridgeGrainKeyKind.Guid
            ? _guidValue
            : throw BridgeException.InvalidKey($"expected a guid key, got {Kind}");

    /// <inheritdoc />
    public override string ToString() => Kind switch
    {
        BridgeGrainKeyKind.String => _stringValue ?? string.Empty,
        BridgeGrainKeyKind.Int64 => _int64Value.ToString(System.Globalization.CultureInfo.InvariantCulture),
        BridgeGrainKeyKind.Guid => _guidValue.ToString(),
        _ => string.Empty,
    };
}
