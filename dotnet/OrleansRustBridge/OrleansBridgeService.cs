using Google.Protobuf;
using Grpc.Core;

using Microsoft.Extensions.Logging;

using Orleans.Bridge.V1;
using Orleans.Runtime;

using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>
/// The gRPC service implementing the generic bridge protocol. It owns no domain
/// knowledge: it resolves an invoker for the requested grain, decodes the key,
/// applies request context and a deadline, and dispatches to Orleans.
/// </summary>
public sealed class OrleansBridgeService : OrleansBridge.OrleansBridgeBase
{
    private readonly IClusterClient _clusterClient;
    private readonly GrainInvokerRegistry _registry;
    private readonly IPayloadCodecRegistry _codecs;
    private readonly BridgeOptions _options;
    private readonly ILogger<OrleansBridgeService> _logger;
    private readonly string _orleansVersion;

    /// <summary>Create the service.</summary>
    public OrleansBridgeService(
        IClusterClient clusterClient,
        GrainInvokerRegistry registry,
        IPayloadCodecRegistry codecs,
        BridgeOptions options,
        ILogger<OrleansBridgeService> logger)
    {
        _clusterClient = clusterClient;
        _registry = registry;
        _codecs = codecs;
        _options = options;
        _logger = logger;
        _orleansVersion = typeof(IClusterClient).Assembly.GetName().Version?.ToString() ?? "unknown";
    }

    /// <inheritdoc />
    public override Task<HealthResponse> Health(HealthRequest request, ServerCallContext context)
    {
        return Task.FromResult(new HealthResponse
        {
            Status = "healthy",
            ServiceId = _options.ServiceId,
            ClusterId = _options.ClusterId,
            BridgeVersion = _options.BridgeVersion,
            OrleansVersion = _orleansVersion,
        });
    }

    /// <inheritdoc />
    public override Task<GetManifestResponse> GetManifest(GetManifestRequest request, ServerCallContext context)
    {
        return Task.FromResult(new GetManifestResponse { Manifest = BuildManifest() });
    }

    /// <inheritdoc />
    public override async Task<InvokeResponse> Invoke(InvokeRequest request, ServerCallContext context)
    {
        try
        {
            var target = request.Target
                ?? throw BridgeException.InvalidKey("request is missing a grain target");
            var invoker = _registry.Resolve(target.InterfaceName, target.GrainType);
            var codec = _codecs.Resolve(request.PayloadCodec);
            var key = GrainKeyParser.Parse(target.Key, invoker.SupportedKeyKinds);

            ApplyRequestContext(request.RequestContext);
            try
            {
                var timeout = ResolveTimeout(request.TimeoutMs);
                var invocation = new BridgeInvocation(key, request.Method, request.Payload.Memory, codec);
                var bytes = await InvokeWithTimeout(invoker, invocation, timeout, context.CancellationToken);

                var response = new InvokeResponse
                {
                    Payload = ByteString.CopyFrom(bytes),
                    PayloadCodec = codec.Name,
                };
                CopyResponseContext(response);
                return response;
            }
            finally
            {
                RequestContext.Clear();
            }
        }
        catch (Exception ex)
        {
            if (ex is BridgeException bridge)
            {
                _logger.LogDebug(ex, "bridge error {Code} for {Method}", bridge.Code, request.Method);
            }
            else
            {
                _logger.LogWarning(ex, "invocation of {Method} failed", request.Method);
            }

            throw Errors.ToRpcException(ex, _options.IncludeExceptionDetail);
        }
    }

    private async Task<byte[]> InvokeWithTimeout(
        IBridgeGrainInvoker invoker,
        BridgeInvocation invocation,
        TimeSpan timeout,
        CancellationToken callerToken)
    {
        using var timeoutCts = new CancellationTokenSource(timeout);
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(callerToken, timeoutCts.Token);

        var grainTask = invoker.InvokeAsync(_clusterClient, invocation, linked.Token);

        // Grain calls are not cancellable by a plain token, so race the call
        // against the deadline rather than relying on cooperative cancellation.
        var finished = await Task.WhenAny(grainTask, Delay(linked.Token));
        if (ReferenceEquals(finished, grainTask))
        {
            return await grainTask;
        }

        if (callerToken.IsCancellationRequested)
        {
            throw new BridgeException(BridgeErrorCodes.Cancelled, "the call was cancelled by the client");
        }

        throw new BridgeException(
            BridgeErrorCodes.OrleansTimeout,
            $"the grain call exceeded its {timeout.TotalMilliseconds:F0}ms deadline");
    }

    private static Task Delay(CancellationToken token) =>
        Task.Delay(Timeout.InfiniteTimeSpan, token).ContinueWith(
            static _ => { },
            CancellationToken.None,
            TaskContinuationOptions.ExecuteSynchronously,
            TaskScheduler.Default);

    private TimeSpan ResolveTimeout(uint requestTimeoutMs)
    {
        var ms = requestTimeoutMs > 0 ? requestTimeoutMs : (uint)_options.DefaultTimeoutMs;
        return TimeSpan.FromMilliseconds(ms);
    }

    private static void ApplyRequestContext(IDictionary<string, string> entries)
    {
        foreach (var (k, v) in entries)
        {
            RequestContext.Set(k, v);
        }
    }

    private void CopyResponseContext(InvokeResponse response)
    {
        foreach (var key in _options.PropagatedResponseContextKeys)
        {
            if (RequestContext.Get(key) is { } value)
            {
                response.ResponseContext[key] = value.ToString() ?? string.Empty;
            }
        }
    }

    private ContractManifest BuildManifest()
    {
        var manifest = new ContractManifest
        {
            ServiceId = _options.ServiceId,
            ClusterId = _options.ClusterId,
            BridgeVersion = _options.BridgeVersion,
            SchemaVersion = BridgeManifest.CurrentSchemaVersion,
        };

        foreach (var invoker in _registry.All)
        {
            var contract = new GrainContract
            {
                InterfaceName = invoker.InterfaceName,
                GrainType = invoker.GrainType,
            };
            contract.SupportedKeyKinds.AddRange(invoker.SupportedKeyKinds);
            foreach (var method in invoker.Methods)
            {
                var grainMethod = new GrainMethod
                {
                    Name = method.Name,
                    RequestType = method.RequestType,
                    ResponseType = method.ResponseType,
                    PayloadCodec = method.PayloadCodec,
                };
                foreach (var parameter in method.Parameters)
                {
                    grainMethod.Parameters.Add(new GrainMethodParameter
                    {
                        Name = parameter.Name,
                        TypeName = parameter.Type,
                    });
                }

                contract.Methods.Add(grainMethod);
            }

            manifest.Grains.Add(contract);
        }

        return manifest;
    }
}
