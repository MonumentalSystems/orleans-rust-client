namespace Counter.Abstractions;

/// <summary>A trivial per-key counter grain used by the sample and tests.</summary>
public interface ICounterGrain : IGrainWithStringKey
{
    /// <summary>Return the current value.</summary>
    Task<long> Get();

    /// <summary>Add <paramref name="value"/> and return the new total.</summary>
    Task<long> Add(long value);

    /// <summary>Reset the value to zero.</summary>
    Task Reset();

    /// <summary>Sleep for <paramref name="milliseconds"/>, then return the value.</summary>
    Task<long> Delay(int milliseconds);

    /// <summary>Return the value of the <c>caller</c> request-context entry.</summary>
    Task<string> WhoCalled();

    /// <summary>
    /// Multi-argument method: add <paramref name="delta"/>, then clamp the
    /// result to at least <paramref name="floor"/>; returns the new value.
    /// </summary>
    Task<long> Adjust(long delta, long floor);
}

/// <summary>An integer-keyed accumulator, used to exercise int64 grain keys.</summary>
public interface IAccumulatorGrain : IGrainWithIntegerKey
{
    /// <summary>Add <paramref name="value"/> and return the new total.</summary>
    Task<long> Add(long value);

    /// <summary>Return the current total.</summary>
    Task<long> Get();
}

/// <summary>A GUID-keyed string register, used to exercise GUID grain keys.</summary>
public interface IRegisterGrain : IGrainWithGuidKey
{
    /// <summary>Store <paramref name="value"/> and return it.</summary>
    Task<string> Set(string value);

    /// <summary>Return the stored value (empty if unset).</summary>
    Task<string> Get();
}
