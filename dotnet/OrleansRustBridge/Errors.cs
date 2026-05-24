using Google.Protobuf;
using Grpc.Core;

using Orleans.Bridge.V1;

using OrleansRustBridge.Abstractions;

namespace OrleansRustBridge;

/// <summary>
/// Maps exceptions to gRPC failures. Transport-level failures use a gRPC status
/// code; the structured, stable <see cref="BridgeError"/> is always attached as
/// a <c>bridge-error-bin</c> response trailer so clients recover a stable code.
/// </summary>
public static class Errors
{
    /// <summary>The trailer key carrying the encoded <see cref="BridgeError"/>.</summary>
    public const string TrailerKey = "bridge-error-bin";

    /// <summary>Build an <see cref="RpcException"/> for <paramref name="ex"/>.</summary>
    public static RpcException ToRpcException(Exception ex, bool includeDetail)
    {
        var classified = Classify(ex, includeDetail);

        var error = new BridgeError
        {
            Code = classified.Code,
            Message = classified.Message,
            Detail = classified.Detail ?? string.Empty,
            Retryable = classified.Retryable,
        };

        var trailers = new Metadata { { TrailerKey, error.ToByteArray() } };
        return new RpcException(new Status(classified.StatusCode, classified.Message), trailers);
    }

    private static Classification Classify(Exception ex, bool includeDetail)
    {
        string? Detail(Exception e) => includeDetail ? $"{e.GetType().FullName}: {e.Message}" : null;

        switch (ex)
        {
            case BridgeException bridge:
                return new Classification(
                    bridge.Code,
                    bridge.Message,
                    bridge.Retryable,
                    StatusForCode(bridge.Code),
                    bridge.Detail ?? (includeDetail ? Detail(bridge) : null));

            case OperationCanceledException:
                return new Classification(
                    BridgeErrorCodes.Cancelled,
                    "the call was cancelled",
                    false,
                    StatusCode.Cancelled,
                    Detail(ex));

            case TimeoutException:
                return new Classification(
                    BridgeErrorCodes.OrleansTimeout,
                    "the grain call timed out",
                    false,
                    StatusCode.DeadlineExceeded,
                    Detail(ex));
        }

        // Orleans runtime exceptions, matched by type name to avoid a hard
        // dependency on internal types.
        var typeName = ex.GetType().FullName ?? string.Empty;
        if (typeName.Contains("MessageRejection", StringComparison.Ordinal))
        {
            return new Classification(
                BridgeErrorCodes.OrleansRejection,
                "Orleans rejected the message",
                true,
                StatusCode.Unavailable,
                Detail(ex));
        }

        if (typeName.Contains("SiloUnavailable", StringComparison.Ordinal)
            || typeName.Contains("ConnectionFailed", StringComparison.Ordinal)
            || (typeName.Contains("OrleansException", StringComparison.Ordinal)
                && ex.Message.Contains("gateway", StringComparison.OrdinalIgnoreCase)))
        {
            return new Classification(
                BridgeErrorCodes.OrleansUnavailable,
                "the Orleans cluster is unavailable",
                true,
                StatusCode.Unavailable,
                Detail(ex));
        }

        // Anything else is treated as an application error originating from the
        // grain call.
        return new Classification(
            BridgeErrorCodes.ApplicationError,
            ex.Message,
            false,
            StatusCode.Internal,
            Detail(ex));
    }

    private static StatusCode StatusForCode(string code) => code switch
    {
        BridgeErrorCodes.UnknownGrain => StatusCode.NotFound,
        BridgeErrorCodes.UnknownMethod => StatusCode.Unimplemented,
        BridgeErrorCodes.InvalidKey => StatusCode.InvalidArgument,
        BridgeErrorCodes.InvalidPayload => StatusCode.InvalidArgument,
        BridgeErrorCodes.SerializationError => StatusCode.Internal,
        BridgeErrorCodes.OrleansRejection => StatusCode.Unavailable,
        BridgeErrorCodes.OrleansTimeout => StatusCode.DeadlineExceeded,
        BridgeErrorCodes.OrleansUnavailable => StatusCode.Unavailable,
        BridgeErrorCodes.ApplicationError => StatusCode.Internal,
        BridgeErrorCodes.Cancelled => StatusCode.Cancelled,
        _ => StatusCode.Internal,
    };

    private readonly record struct Classification(
        string Code,
        string Message,
        bool Retryable,
        StatusCode StatusCode,
        string? Detail);
}
