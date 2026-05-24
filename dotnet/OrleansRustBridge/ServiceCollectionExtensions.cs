using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;

using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>Registration helpers for the Orleans Rust bridge.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Register the bridge gRPC service, codecs, and supporting services.
    /// Register your <see cref="IBridgeGrainInvoker"/> implementations
    /// separately (e.g. <c>services.AddSingleton&lt;IBridgeGrainInvoker, MyInvoker&gt;()</c>).
    /// </summary>
    public static IServiceCollection AddOrleansRustBridge(
        this IServiceCollection services,
        Action<BridgeOptions>? configure = null)
    {
        services.AddGrpc();

        services.AddSingleton(serviceProvider =>
        {
            var options = new BridgeOptions();
            serviceProvider.GetService<IConfiguration>()?
                .GetSection(BridgeOptions.SectionName)
                .Bind(options);
            configure?.Invoke(options);
            return options;
        });

        services.AddSingleton<IPayloadCodec, JsonPayloadCodec>();
        services.AddSingleton<IPayloadCodec, ProtobufPayloadCodec>();

        services.AddSingleton<IPayloadCodecRegistry>(serviceProvider =>
        {
            var options = serviceProvider.GetRequiredService<BridgeOptions>();
            var available = serviceProvider
                .GetServices<IPayloadCodec>()
                .ToDictionary(codec => codec.Name, StringComparer.OrdinalIgnoreCase);

            var enabled = new List<IPayloadCodec>();
            foreach (var name in options.PayloadCodecs)
            {
                if (!available.TryGetValue(name, out var codec))
                {
                    throw new InvalidOperationException(
                        $"configured payload codec '{name}' is not registered");
                }

                enabled.Add(codec);
            }

            return new PayloadCodecRegistry(enabled);
        });

        services.AddSingleton<GrainInvokerRegistry>();
        services.AddSingleton<OrleansBridgeService>();

        return services;
    }

    /// <summary>Map the bridge gRPC endpoints.</summary>
    public static IEndpointRouteBuilder MapOrleansRustBridge(this IEndpointRouteBuilder endpoints)
    {
        endpoints.MapGrpcService<OrleansBridgeService>();
        return endpoints;
    }
}
