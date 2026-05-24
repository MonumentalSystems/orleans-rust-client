using System.Net;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;

namespace Counter.Bridge;

/// <summary>
/// TLS certificate setup for the sample bridge. Supports an operator-provided
/// PEM cert/key pair and a development self-signed mode used by the integration
/// tests. Not intended as production certificate management.
/// </summary>
internal static class TlsSetup
{
    /// <summary>
    /// Resolve the server certificate, or <c>null</c> for cleartext.
    /// <list type="bullet">
    /// <item>If <c>BRIDGE_TLS_CERT_PEM</c> and <c>BRIDGE_TLS_KEY_PEM</c> are set,
    /// loads that PEM pair.</item>
    /// <item>Else if <c>BRIDGE_TLS_SELF_SIGNED_CA_OUT</c> is set, generates a dev
    /// CA + server cert, serves with the server cert, and writes the CA's public
    /// PEM to that path (for the client to trust).</item>
    /// </list>
    /// </summary>
    public static X509Certificate2? LoadOrGenerate()
    {
        var certPath = Env("BRIDGE_TLS_CERT_PEM");
        var keyPath = Env("BRIDGE_TLS_KEY_PEM");
        if (certPath is not null && keyPath is not null)
        {
            var loaded = X509Certificate2.CreateFromPemFile(certPath, keyPath);
            return Reimport(loaded);
        }

        var caOut = Env("BRIDGE_TLS_SELF_SIGNED_CA_OUT");
        return caOut is not null ? GenerateDevChain(caOut) : null;
    }

    private static X509Certificate2 GenerateDevChain(string caPemOutputPath)
    {
        var now = DateTimeOffset.UtcNow;

        using var caKey = RSA.Create(2048);
        var caRequest = new CertificateRequest(
            "CN=Orleans Bridge Dev CA", caKey, HashAlgorithmName.SHA256, RSASignaturePadding.Pkcs1);
        caRequest.CertificateExtensions.Add(new X509BasicConstraintsExtension(true, false, 0, critical: true));
        caRequest.CertificateExtensions.Add(
            new X509KeyUsageExtension(X509KeyUsageFlags.KeyCertSign | X509KeyUsageFlags.CrlSign, critical: true));
        using var caCert = caRequest.CreateSelfSigned(now.AddDays(-1), now.AddYears(10));

        using var serverKey = RSA.Create(2048);
        var serverRequest = new CertificateRequest(
            "CN=localhost", serverKey, HashAlgorithmName.SHA256, RSASignaturePadding.Pkcs1);

        var san = new SubjectAlternativeNameBuilder();
        san.AddDnsName("localhost");
        san.AddIpAddress(IPAddress.Loopback);
        serverRequest.CertificateExtensions.Add(san.Build());
        serverRequest.CertificateExtensions.Add(new X509BasicConstraintsExtension(false, false, 0, critical: false));
        serverRequest.CertificateExtensions.Add(
            new X509EnhancedKeyUsageExtension([Oid.FromOidValue("1.3.6.1.5.5.7.3.1", OidGroup.EnhancedKeyUsage)], critical: false));

        var serial = RandomNumberGenerator.GetBytes(16);
        using var serverCert = serverRequest.Create(caCert, now.AddDays(-1), now.AddYears(5), serial);
        using var serverWithKey = serverCert.CopyWithPrivateKey(serverKey);

        File.WriteAllText(caPemOutputPath, caCert.ExportCertificatePem());

        return Reimport(serverWithKey);
    }

    // Round-trip through PKCS#12 so Kestrel can use the private key on every
    // platform (PEM-loaded keys can be ephemeral).
    private static X509Certificate2 Reimport(X509Certificate2 certificate) =>
        X509CertificateLoader.LoadPkcs12(certificate.Export(X509ContentType.Pfx), password: null);

    private static string? Env(string name) =>
        Environment.GetEnvironmentVariable(name) is { Length: > 0 } value ? value : null;
}
