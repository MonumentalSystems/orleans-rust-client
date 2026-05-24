namespace OrleansRustBridge.Abstractions;

/// <summary>A single grain method in the manifest.</summary>
/// <param name="Name">Method name as exposed on the grain interface.</param>
/// <param name="RequestType">Single-argument .NET type name, or empty for none.</param>
/// <param name="ResponseType">Response .NET type name, or empty for void.</param>
/// <param name="PayloadCodec">Codec used for this method's payloads.</param>
public sealed record GrainMethodDescriptor(
    string Name,
    string RequestType,
    string ResponseType,
    string PayloadCodec);

/// <summary>A single grain contract in the manifest.</summary>
/// <param name="InterfaceName">Fully-qualified grain interface name.</param>
/// <param name="GrainType">Grain type alias used for dispatch.</param>
/// <param name="Methods">Methods the grain exposes.</param>
/// <param name="SupportedKeyKinds">Key kinds the grain supports.</param>
public sealed record GrainContractDescriptor(
    string InterfaceName,
    string GrainType,
    IReadOnlyList<GrainMethodDescriptor> Methods,
    IReadOnlyList<string> SupportedKeyKinds);
