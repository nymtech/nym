// The store's server certificate is signed by the provider's own CA (Aiven), which
// neither GitHub runners nor Vercel trust. The CA comes in as PEM text, because
// neither has a file to point `sslrootcert` at. pg lets TLS settings in the URL
// replace an `ssl` option, so a URL that carries the CA must not also set them.

const URL_TLS = /[?&](sslmode|sslrootcert|sslcert|sslkey)=/i;

/**
 * The pg connection options for a store URL and its optional CA certificate.
 * With a CA, pg verifies the server's chain and host name (verify-full).
 * Without one, the URL must say how to connect, for example `sslmode=disable`
 * for a local store, so that a missing CA cannot quietly turn TLS off.
 *
 * @param {string} url
 * @param {string | undefined} ca PEM text of the CA certificate
 * @returns {{ connectionString: string, ssl?: { ca: string } }}
 */
export function storeConnection(url, ca) {
  if (!ca) {
    if (!URL_TLS.test(url)) {
      throw new Error('the store URL sets no sslmode and RETRIEVAL_STORE_CA_CERT is not set: set the CA, or add sslmode to the URL for a local store');
    }
    return { connectionString: url };
  }
  // Node accepts any text as a CA without an error, and then trusts nothing.
  if (!ca.includes('-----BEGIN CERTIFICATE-----')) {
    throw new Error('RETRIEVAL_STORE_CA_CERT is not PEM text: it must contain -----BEGIN CERTIFICATE----- (do not encode it with base64)');
  }
  if (URL_TLS.test(url)) {
    throw new Error('the store URL sets sslmode or sslrootcert, which would replace RETRIEVAL_STORE_CA_CERT: remove them from the URL');
  }
  return { connectionString: url, ssl: { ca } };
}
