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

    /// <inheritdoc />
    public Task<long> Adjust(long delta, long floor)
    {
        _value = Math.Max(_value + delta, floor);
        return Task.FromResult(_value);
    }
}

/// <summary>In-memory integer-keyed accumulator.</summary>
public sealed class AccumulatorGrain : Grain, IAccumulatorGrain
{
    private long _total;

    /// <inheritdoc />
    public Task<long> Add(long value)
    {
        _total += value;
        return Task.FromResult(_total);
    }

    /// <inheritdoc />
    public Task<long> Get() => Task.FromResult(_total);
}

/// <summary>In-memory GUID-keyed string register.</summary>
public sealed class RegisterGrain : Grain, IRegisterGrain
{
    private string _value = string.Empty;

    /// <inheritdoc />
    public Task<string> Set(string value)
    {
        _value = value;
        return Task.FromResult(_value);
    }

    /// <inheritdoc />
    public Task<string> Get() => Task.FromResult(_value);
}
