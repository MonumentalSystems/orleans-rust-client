using Grpc.Core;

using Orleans.Bridge.V1;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

public class ErrorsTests
{
    private static BridgeError Decode(RpcException ex)
    {
        var bytes = ex.Trailers.GetValueBytes(Errors.TrailerKey);
        Assert.NotNull(bytes);
        return BridgeError.Parser.ParseFrom(bytes);
    }

    [Fact]
    public void BridgeExceptionMapsCodeAndTrailer()
    {
        var ex = Errors.ToRpcException(
            BridgeException.UnknownMethod("ISomeGrain", "Nope"),
            includeDetail: false);

        Assert.Equal(StatusCode.Unimplemented, ex.StatusCode);
        var error = Decode(ex);
        Assert.Equal(BridgeErrorCodes.UnknownMethod, error.Code);
        Assert.False(error.Retryable);
    }

    [Fact]
    public void RetryableFlagIsPreserved()
    {
        var ex = Errors.ToRpcException(
            new BridgeException(BridgeErrorCodes.OrleansRejection, "busy", retryable: true),
            includeDetail: false);

        Assert.Equal(StatusCode.Unavailable, ex.StatusCode);
        Assert.True(Decode(ex).Retryable);
    }

    [Fact]
    public void TimeoutExceptionMapsToOrleansTimeout()
    {
        var ex = Errors.ToRpcException(new TimeoutException("slow"), includeDetail: false);
        Assert.Equal(StatusCode.DeadlineExceeded, ex.StatusCode);
        Assert.Equal(BridgeErrorCodes.OrleansTimeout, Decode(ex).Code);
    }

    [Fact]
    public void GenericExceptionIsApplicationError()
    {
        var ex = Errors.ToRpcException(new InvalidOperationException("boom"), includeDetail: false);
        Assert.Equal(BridgeErrorCodes.ApplicationError, Decode(ex).Code);
    }

    [Fact]
    public void DetailIsSuppressedUnlessRequested()
    {
        var without = Errors.ToRpcException(new InvalidOperationException("secret"), includeDetail: false);
        Assert.Equal(string.Empty, Decode(without).Detail);

        var with = Errors.ToRpcException(new InvalidOperationException("secret"), includeDetail: true);
        Assert.Contains("secret", Decode(with).Detail);
    }
}
