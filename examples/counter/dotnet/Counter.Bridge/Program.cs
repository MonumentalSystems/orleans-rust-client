using System.Globalization;

using Counter.Bridge;

using Microsoft.AspNetCore.Server.Kestrel.Core;

using Orleans.Configuration;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

var builder = WebApplication.CreateBuilder(args);

var gatewayPort = EnvInt("ORLEANS_GATEWAY_PORT", 30000);
var serviceId = EnvStr("ORLEANS_SERVICE_ID", "counter-sample");
var clusterId = EnvStr("ORLEANS_CLUSTER_ID", "dev");

if (string.IsNullOrEmpty(Environment.GetEnvironmentVariable("ASPNETCORE_URLS")))
{
    builder.WebHost.UseUrls("http://127.0.0.1:50051");
}

// gRPC over cleartext (h2c) needs HTTP/2 forced; there is no ALPN negotiation.
builder.WebHost.ConfigureKestrel(options =>
    options.ConfigureEndpointDefaults(listen => listen.Protocols = HttpProtocols.Http2));

builder.Host.UseOrleansClient(client =>
{
    client.UseLocalhostClustering(gatewayPort: gatewayPort, serviceId: serviceId, clusterId: clusterId);
    client.Configure<ClusterOptions>(options =>
    {
        options.ServiceId = serviceId;
        options.ClusterId = clusterId;
    });
});

builder.Services.AddOrleansRustBridge(options =>
{
    options.ServiceId = serviceId;
    options.ClusterId = clusterId;
    options.BridgeVersion = "0.1.0";
});
builder.Services.AddSingleton<IBridgeGrainInvoker, CounterGrainInvoker>();

var app = builder.Build();
app.MapOrleansRustBridge();
await app.RunAsync();

static int EnvInt(string name, int fallback) =>
    int.TryParse(Environment.GetEnvironmentVariable(name), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
        ? value
        : fallback;

static string EnvStr(string name, string fallback) =>
    Environment.GetEnvironmentVariable(name) is { Length: > 0 } value ? value : fallback;
