using System.Reflection;
using System.Runtime.Loader;

using OrleansRustBridge.Abstractions;
using OrleansRustBridge.Tools;

return Cli.Run(args);

internal static class Cli
{
    public static int Run(string[] args)
    {
        if (args.Length == 0 || args[0] is "-h" or "--help" or "help")
        {
            PrintUsage();
            return args.Length == 0 ? 1 : 0;
        }

        try
        {
            var options = ToolOptions.Parse(args[1..]);
            return args[0] switch
            {
                "manifest" => Manifest(options),
                "invokers" => Invokers(options),
                var other => Fail($"unknown command '{other}'"),
            };
        }
        catch (ToolError error)
        {
            Console.Error.WriteLine($"error: {error.Message}");
            return 1;
        }
    }

    private static int Manifest(ToolOptions options)
    {
        var assembly = LoadAssembly(options.RequireAssembly());
        var grains = GrainReflection.Discover(assembly, options.Grains.Count > 0 ? options.Grains : null);

        var contracts = grains
            .Select(grain => new GrainContractDescriptor(
                grain.InterfaceName,
                grain.GrainType,
                grain.Methods.Select(GrainReflection.ToDescriptor).ToList(),
                new[] { grain.KeyKind }))
            .ToList();

        var manifest = new BridgeManifest(
            options.ServiceId,
            options.ClusterId,
            options.BridgeVersion,
            BridgeManifest.CurrentSchemaVersion,
            contracts);

        var json = manifest.ToJson();
        if (options.Out is { } path)
        {
            File.WriteAllText(path, json);
            Console.Error.WriteLine($"wrote {path} ({contracts.Count} grain(s))");
        }
        else
        {
            Console.WriteLine(json);
        }

        return 0;
    }

    private static int Invokers(ToolOptions options)
    {
        var outDir = options.OutDir ?? throw new ToolError("--out-dir is required for the 'invokers' command");
        Directory.CreateDirectory(outDir);

        var assembly = LoadAssembly(options.RequireAssembly());
        var grains = GrainReflection.Discover(assembly, options.Grains.Count > 0 ? options.Grains : null);

        foreach (var grain in grains)
        {
            var source = InvokerGenerator.Generate(grain, options.Namespace);
            var file = Path.Combine(outDir, $"{InvokerGenerator.ClassName(grain)}.g.cs");
            File.WriteAllText(file, source);
            Console.Error.WriteLine($"wrote {file}");
        }

        return 0;
    }

    private static Assembly LoadAssembly(string path)
    {
        var full = Path.GetFullPath(path);
        if (!File.Exists(full))
        {
            throw new ToolError($"assembly not found: {full}");
        }

        var directory = Path.GetDirectoryName(full)!;
        AssemblyLoadContext.Default.Resolving += (context, name) =>
        {
            var candidate = Path.Combine(directory, $"{name.Name}.dll");
            return File.Exists(candidate) ? context.LoadFromAssemblyPath(candidate) : null;
        };

        return AssemblyLoadContext.Default.LoadFromAssemblyPath(full);
    }

    private static int Fail(string message)
    {
        Console.Error.WriteLine($"error: {message}");
        PrintUsage();
        return 1;
    }

    private static void PrintUsage()
    {
        Console.Error.WriteLine(
            """
            orleans-rust-bridge-tools <command> [options]

            Commands:
              manifest    Emit a bridge manifest (JSON) for a grain assembly.
              invokers    Generate C# IBridgeGrainInvoker adapters (experimental).

            Options:
              --assembly <path>        Grain abstractions assembly (.dll). Required.
              --grain <FullTypeName>   Restrict to specific interfaces (repeatable).
              --out <file>             Output file for 'manifest' (default: stdout).
              --out-dir <dir>          Output directory for 'invokers'.
              --namespace <ns>         Namespace for generated invokers.
              --service-id <id>        Manifest service id.
              --cluster-id <id>        Manifest cluster id.
              --bridge-version <ver>   Manifest bridge version.
            """);
    }
}

internal sealed class ToolError(string message) : Exception(message);

internal sealed class ToolOptions
{
    public string? Assembly { get; private set; }
    public string? Out { get; private set; }
    public string? OutDir { get; private set; }
    public string Namespace { get; private set; } = "Generated";
    public string ServiceId { get; private set; } = "orleans-service";
    public string ClusterId { get; private set; } = "dev";
    public string BridgeVersion { get; private set; } = "0.1.0";
    public HashSet<string> Grains { get; } = new(StringComparer.Ordinal);

    public string RequireAssembly() =>
        Assembly ?? throw new ToolError("--assembly is required");

    public static ToolOptions Parse(string[] args)
    {
        var options = new ToolOptions();
        for (var i = 0; i < args.Length; i++)
        {
            var flag = args[i];
            string Next()
            {
                if (i + 1 >= args.Length)
                {
                    throw new ToolError($"missing value for {flag}");
                }

                return args[++i];
            }

            switch (flag)
            {
                case "--assembly":
                    options.Assembly = Next();
                    break;
                case "--out":
                    options.Out = Next();
                    break;
                case "--out-dir":
                    options.OutDir = Next();
                    break;
                case "--namespace":
                    options.Namespace = Next();
                    break;
                case "--service-id":
                    options.ServiceId = Next();
                    break;
                case "--cluster-id":
                    options.ClusterId = Next();
                    break;
                case "--bridge-version":
                    options.BridgeVersion = Next();
                    break;
                case "--grain":
                    options.Grains.Add(Next());
                    break;
                default:
                    throw new ToolError($"unknown option '{flag}'");
            }
        }

        return options;
    }
}
