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
}
