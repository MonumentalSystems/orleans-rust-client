using System.Globalization;

using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

using Orleans.Configuration;

static int EnvInt(string name, int fallback) =>
    int.TryParse(Environment.GetEnvironmentVariable(name), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
        ? value
        : fallback;

static string EnvStr(string name, string fallback) =>
    Environment.GetEnvironmentVariable(name) is { Length: > 0 } value ? value : fallback;

var siloPort = EnvInt("ORLEANS_SILO_PORT", 11111);
var gatewayPort = EnvInt("ORLEANS_GATEWAY_PORT", 30000);
var serviceId = EnvStr("ORLEANS_SERVICE_ID", "counter-sample");
var clusterId = EnvStr("ORLEANS_CLUSTER_ID", "dev");

using var host = Host.CreateDefaultBuilder(args)
    .UseOrleans(silo =>
    {
        silo.UseLocalhostClustering(
            siloPort: siloPort,
            gatewayPort: gatewayPort,
            serviceId: serviceId,
            clusterId: clusterId);
        silo.Configure<ClusterOptions>(options =>
        {
            options.ServiceId = serviceId;
            options.ClusterId = clusterId;
        });
    })
    .ConfigureLogging(logging => logging.AddSimpleConsole(o => o.SingleLine = true))
    .Build();

await host.RunAsync();
