using Counter.Abstractions;

using Orleans.Runtime;

namespace Counter.Silo;

/// <summary>In-memory counter grain implementation.</summary>
public sealed class CounterGrain : Grain, ICounterGrain
{
    private long _value;

    /// <inheritdoc />
    public Task<long> Get() => Task.FromResult(_value);

    /// <inheritdoc />
    public Task<long> Add(long value)
    {
        _value += value;
        return Task.FromResult(_value);
    }

    /// <inheritdoc />
    public Task Reset()
    {
        _value = 0;
        return Task.CompletedTask;
    }

    /// <inheritdoc />
    public async Task<long> Delay(int milliseconds)
    {
        await Task.Delay(milliseconds);
        return _value;
    }

    /// <inheritdoc />
    public Task<string> WhoCalled()
    {
        var caller = RequestContext.Get("caller") as string;
        return Task.FromResult(caller ?? string.Empty);
    }
}
