import { describe, it, expect } from 'vitest';
import { storeConnection } from './store-tls.mjs';

const URL = 'postgresql://retrieval_reader:pw@db.example.com:21699/codex';
const CA = '-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n';

describe('storeConnection', () => {
  it('passes the CA as the ssl option', () => {
    expect(storeConnection(URL, CA)).toEqual({ connectionString: URL, ssl: { ca: CA } });
  });

  it('refuses a URL that sets TLS settings beside the CA, because pg would let them replace it', () => {
    expect(() => storeConnection(`${URL}?sslmode=verify-full`, CA)).toThrow(/remove them from the URL/);
    expect(() => storeConnection(`${URL}?application_name=x&sslrootcert=/ca.pem`, CA)).toThrow(/remove them from the URL/);
  });

  it('refuses a CA that is not PEM text, such as a base64-encoded file', () => {
    expect(() => storeConnection(URL, Buffer.from(CA).toString('base64'))).toThrow(/not PEM text/);
  });

  it('uses the URL as it is when there is no CA and the URL sets sslmode', () => {
    const local = 'postgresql://retrieval_reader:pw@127.0.0.1:54329/retrieval?sslmode=disable';
    expect(storeConnection(local, undefined)).toEqual({ connectionString: local });
    expect(storeConnection(local, '')).toEqual({ connectionString: local });
  });

  it('refuses a URL with neither a CA nor sslmode, so that a missing CA cannot turn TLS off', () => {
    expect(() => storeConnection(URL, undefined)).toThrow(/sets no sslmode/);
  });
});
