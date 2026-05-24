using Google.Protobuf;

using Grpc.Core;

using Microsoft.Extensions.Logging.Abstractions;

using Orleans;
using Orleans.Bridge.V1;
using Orleans.Runtime;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

// An invoker whose behaviour is supplied per test; it ignores the cluster
// client, so the service can be tested in-process without a live cluster.
internal sealed class ConfigurableInvoker(Func<BridgeInvocation, Task<byte[]>> handler) : IBridgeGrainInvoker
{
    public string InterfaceName => "Sample.IThingGrain";
    public string GrainType => "thing";
    public IReadOnlyList<BridgeMethodDescriptor> Methods => [new("Echo", "System.String", "System.String")];
    public IReadOnlyList<string> SupportedKeyKinds => ["string"];

    public Task<byte[]> InvokeAsync(IClusterClient client, BridgeInvocation invocation, CancellationToken cancellationToken) =>
        handler(invocation);
}

// Minimal ServerCallContext for tests; only the cancellation token is used by
// the service under test.
internal sealed class FakeServerCallContext(CancellationToken token) : ServerCallContext
{
    protected override string MethodCore => "Invoke";
    protected override string HostCore => "localhost";
    protected override string PeerCore => "peer";
    protected override DateTime DeadlineCore => DateTime.UtcNow.AddSeconds(30);
    protected override Metadata RequestHeadersCore => new();
    protected override CancellationToken CancellationTokenCore => token;
    protected override Metadata ResponseTrailersCore => new();
    protected override Status StatusCore { get; set; }
    protected override WriteOptions? WriteOptionsCore { get; set; }
    protected override AuthContext AuthContextCore => new(null, new Dictionary<string, List<AuthProperty>>());

    protected override ContextPropagationToken CreatePropagationTokenCore(ContextPropagationOptions? options) =>
        throw new NotSupportedException();

    protected override Task WriteResponseHeadersAsyncCore(Metadata responseHeaders) => Task.CompletedTask;
}

public class OrleansBridgeServiceTests
{
    private static OrleansBridgeService Service(IBridgeGrainInvoker invoker, BridgeOptions? options = null) =>
        new(
            clusterClient: null!,
            new GrainInvokerRegistry([invoker]),
            new PayloadCodecRegistry([new JsonPayloadCodec()]),
            options ?? new BridgeOptions(),
            NullLogger<OrleansBridgeService>.Instance);

    private static ServerCallContext Context(CancellationToken token = default) =>
        new FakeServerCallContext(token);

    private static InvokeRequest Request(string method, string payloadJson, uint timeoutMs = 0)
    {
        var request = new InvokeRequest
        {
            Target = new GrainTarget
            {
                InterfaceName = "Sample.IThingGrain",
                GrainType = "thing",
                Key = new GrainKey { StringKey = "k" },
            },
            Method = method,
            Payload = ByteString.CopyFromUtf8(payloadJson),
            PayloadCodec = "json",
            TimeoutMs = timeoutMs,
        };
        return request;
    }

    [Fact]
    public async Task HealthReportsIdentity()
    {
        var service = Service(new ConfigurableInvoker(_ => Task.FromResult(Array.Empty<byte>())),
            new BridgeOptions { ServiceId = "svc", ClusterId = "dev" });
        var health = await service.Health(new HealthRequest(), Context());
        Assert.Equal("healthy", health.Status);
        Assert.Equal("svc", health.ServiceId);
        Assert.False(string.IsNullOrEmpty(health.OrleansVersion));
    }

    [Fact]
    public async Task GetManifestListsRegisteredGrain()
    {
        var service = Service(new ConfigurableInvoker(_ => Task.FromResult(Array.Empty<byte>())));
        var response = await service.GetManifest(new GetManifestRequest(), Context());
        var grain = Assert.Single(response.Manifest.Grains);
        Assert.Equal("Sample.IThingGrain", grain.InterfaceName);
        Assert.Contains(grain.Methods, m => m.Name == "Echo");
    }

    [Fact]
    public async Task InvokeReturnsEncodedResult()
    {
        var service = Service(new ConfigurableInvoker(inv => Task.FromResult(inv.Encode(inv.DecodeRequest<string>().ToUpperInvariant()))));
        var response = await service.Invoke(Request("Echo", "\"hi\""), Context());
        Assert.Equal("\"HI\"", response.Payload.ToStringUtf8());
    }

    [Fact]
    public async Task InvokePropagatesRequestContext()
    {
        var service = Service(new ConfigurableInvoker(inv =>
            Task.FromResult(inv.Encode(RequestContext.Get("caller") as string ?? "none"))));
        var request = Request("Echo", "\"x\"");
        request.RequestContext["caller"] = "bob";
        var response = await service.Invoke(request, Context());
        Assert.Equal("\"bob\"", response.Payload.ToStringUtf8());
    }

    [Fact]
    public async Task InvokeCopiesConfiguredResponseContext()
    {
        var service = Service(
            new ConfigurableInvoker(inv => Task.FromResult(inv.Encode("ok"))),
            new BridgeOptions { PropagatedResponseContextKeys = ["trace"] });

        // In production Orleans propagates this back from the grain; in-process
        // we seed the ambient context so the copy path is exercised.
        RequestContext.Set("trace", "t-123");
        try
        {
            var response = await service.Invoke(Request("Echo", "\"x\""), Context());
            Assert.Equal("t-123", response.ResponseContext["trace"]);
        }
        finally
        {
            RequestContext.Clear();
        }
    }

    [Fact]
    public async Task InvokeTimesOut()
    {
        var service = Service(new ConfigurableInvoker(async _ =>
        {
            await Task.Delay(5_000);
            return Array.Empty<byte>();
        }));
        var ex = await Assert.ThrowsAsync<RpcException>(() => service.Invoke(Request("Echo", "\"x\"", timeoutMs: 50), Context()));
        Assert.Equal(StatusCode.DeadlineExceeded, ex.StatusCode);
    }

    [Fact]
    public async Task InvokeIsCancelledByCaller()
    {
        var service = Service(new ConfigurableInvoker(async _ =>
        {
            await Task.Delay(5_000);
            return Array.Empty<byte>();
        }));
        using var cts = new CancellationTokenSource();
        await cts.CancelAsync();
        var ex = await Assert.ThrowsAsync<RpcException>(() => service.Invoke(Request("Echo", "\"x\""), Context(cts.Token)));
        Assert.Equal(StatusCode.Cancelled, ex.StatusCode);
    }

    [Fact]
    public async Task InvokeUnknownGrainReturnsNotFound()
    {
        var service = Service(new ConfigurableInvoker(_ => Task.FromResult(Array.Empty<byte>())));
        var request = Request("Echo", "\"x\"");
        request.Target.InterfaceName = "Sample.IMissingGrain";
        var ex = await Assert.ThrowsAsync<RpcException>(() => service.Invoke(request, Context()));
        Assert.Equal(StatusCode.NotFound, ex.StatusCode);
    }

    [Fact]
    public async Task InvokeApplicationExceptionMapsToInternal()
    {
        var service = Service(new ConfigurableInvoker(_ => throw new InvalidOperationException("grain blew up")));
        var ex = await Assert.ThrowsAsync<RpcException>(() => service.Invoke(Request("Echo", "\"x\""), Context()));
        Assert.Equal(StatusCode.Internal, ex.StatusCode);
    }
}
