using Grpc.Core;

using Orleans.Bridge.V1;

using OrleansRustBridge;
using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge.Tests;

// Exceptions whose type names mimic Orleans' runtime exceptions, so the
// name-based classification in Errors can be exercised without referencing
// internal Orleans types.
internal sealed class OrleansMessageRejectionException(string message) : Exception(message);

internal sealed class ConnectionFailedException(string message) : Exception(message);

public class ErrorsClassificationTests
{
    private static BridgeError Decode(RpcException ex) =>
        BridgeError.Parser.ParseFrom(ex.Trailers.GetValueBytes(Errors.TrailerKey));

    [Theory]
    [InlineData(BridgeErrorCodes.UnknownGrain, StatusCode.NotFound)]
    [InlineData(BridgeErrorCodes.UnknownMethod, StatusCode.Unimplemented)]
    [InlineData(BridgeErrorCodes.InvalidKey, StatusCode.InvalidArgument)]
    [InlineData(BridgeErrorCodes.InvalidPayload, StatusCode.InvalidArgument)]
    [InlineData(BridgeErrorCodes.SerializationError, StatusCode.Internal)]
    [InlineData(BridgeErrorCodes.OrleansRejection, StatusCode.Unavailable)]
    [InlineData(BridgeErrorCodes.OrleansTimeout, StatusCode.DeadlineExceeded)]
    [InlineData(BridgeErrorCodes.OrleansUnavailable, StatusCode.Unavailable)]
    [InlineData(BridgeErrorCodes.ApplicationError, StatusCode.Internal)]
    [InlineData(BridgeErrorCodes.Cancelled, StatusCode.Cancelled)]
    [InlineData(BridgeErrorCodes.Internal, StatusCode.Internal)]
    public void BridgeCodesMapToStatus(string code, StatusCode expected)
    {
        var ex = Errors.ToRpcException(new BridgeException(code, "x"), includeDetail: false);
        Assert.Equal(expected, ex.StatusCode);
        Assert.Equal(code, Decode(ex).Code);
    }

    [Fact]
    public void CancellationMapsToCancelled()
    {
        var ex = Errors.ToRpcException(new OperationCanceledException(), includeDetail: false);
        Assert.Equal(StatusCode.Cancelled, ex.StatusCode);
        Assert.Equal(BridgeErrorCodes.Cancelled, Decode(ex).Code);
    }

    [Fact]
    public void MessageRejectionIsRetryableUnavailable()
    {
        var ex = Errors.ToRpcException(new OrleansMessageRejectionException("busy"), includeDetail: false);
        Assert.Equal(StatusCode.Unavailable, ex.StatusCode);
        var error = Decode(ex);
        Assert.Equal(BridgeErrorCodes.OrleansRejection, error.Code);
        Assert.True(error.Retryable);
    }

    [Fact]
    public void ConnectionFailureMapsToUnavailable()
    {
        var ex = Errors.ToRpcException(new ConnectionFailedException("no gateway"), includeDetail: false);
        Assert.Equal(BridgeErrorCodes.OrleansUnavailable, Decode(ex).Code);
        Assert.True(Decode(ex).Retryable);
    }

    [Fact]
    public void DetailIncludesTypeWhenEnabled()
    {
        var ex = Errors.ToRpcException(new BridgeException(BridgeErrorCodes.UnknownGrain, "missing"), includeDetail: true);
        Assert.Contains("BridgeException", Decode(ex).Detail);
    }
}
