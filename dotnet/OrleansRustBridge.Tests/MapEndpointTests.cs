using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class MapEndpointTests
{
    [Fact]
    public void MapOrleansRustBridgeRegistersTheGrpcEndpoint()
    {
        var builder = WebApplication.CreateBuilder();
        builder.Services.AddSingleton<IBridgeGrainInvoker>(new FakeInvoker("IFoo", "foo"));
        builder.Services.AddOrleansRustBridge();

        using var app = builder.Build();
        var endpoints = app.MapOrleansRustBridge();

        Assert.Same(app, endpoints);
    }
}
