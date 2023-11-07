import { handleResponseMimeTypes } from './handle-response-mime-types';

describe('handleResponseMimeTypes', () => {
  test('gracefully handles empty values', async () => {
    const resp = await handleResponseMimeTypes(new Response());
    expect(Object.values(resp)).toHaveLength(0);
  });

  test('handles text', async () => {
    const TEXT = 'This is text';
    const resp = await handleResponseMimeTypes(
      new Response(TEXT, { headers: new Headers([['Content-Type', 'text/plain']]) }),
    );
    expect(resp.text).toBe(TEXT);
  });
  test('handles text (charset=utf-8)', async () => {
    const TEXT = 'This is text';
    const resp = await handleResponseMimeTypes(
      new Response(TEXT, { headers: new Headers([['Content-Type', 'text/plain; charset=utf-8']]) }),
    );
    expect(resp.text).toBe(TEXT);
  });
  test('handles html', async () => {
    const TEXT = 'This is html';
    const resp = await handleResponseMimeTypes(
      new Response(TEXT, { headers: new Headers([['Content-Type', 'text/html']]) }),
    );
    expect(resp.text).toBe(TEXT);
  });
  test('handles html (charset=utf-8)', async () => {
    const TEXT = 'This is html';
    const resp = await handleResponseMimeTypes(
      new Response(TEXT, { headers: new Headers([['Content-Type', 'text/html; charset=utf-8']]) }),
    );
    expect(resp.text).toBe(TEXT);
  });
  test('handles images', async () => {
    const DATA = Buffer.from(new Uint8Array([0, 1, 2, 3]));
    const resp = await handleResponseMimeTypes(
      new Response(DATA, { headers: new Headers([['Content-Type', 'image/jpeg']]) }),
    );
    expect(resp.blobUrl).toBeDefined();
  });
  test('handles videos', async () => {
    const DATA = Buffer.from(new Uint8Array([0, 1, 2, 3]));
    const resp = await handleResponseMimeTypes(
      new Response(DATA, { headers: new Headers([['Content-Type', 'video/mpeg4']]) }),
    );
    expect(resp.blobUrl).toBeDefined();
  });
  test('handles form data when URL encoded', async () => {
    const formData = 'foo=bar&baz=42';
    const resp = await handleResponseMimeTypes(
      new Response(formData, { headers: new Headers([['Content-Type', 'application/x-www-form-urlencoded']]) }),
    );
    expect(resp.formData.foo).toBe('bar');
    expect(resp.formData.baz).toBe('42');
  });
  test('handles JSON data', async () => {
    const json = '{ "foo": "bar", "baz": 42 }';
    const resp = await handleResponseMimeTypes(
      new Response(json, { headers: new Headers([['Content-Type', 'application/json']]) }),
    );
    expect(resp.text).toBe(json);
  });
  test('handles JSON data (charset=utf-8)', async () => {
    const json = '{ "foo": "bar", "baz": 42 }';
    const resp = await handleResponseMimeTypes(
      new Response(json, { headers: new Headers([['Content-Type', 'application/json; charset=utf-8']]) }),
    );
    expect(resp.text).toBe(json);
  });
});
