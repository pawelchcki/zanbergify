// CORS-enabled CDN proxy for R2 model files with Range request support
export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);

    // Handle OPTIONS for CORS preflight
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: {
          'Access-Control-Allow-Origin': '*',
          'Access-Control-Allow-Methods': 'GET, HEAD, OPTIONS',
          'Access-Control-Allow-Headers': '*',
          'Access-Control-Max-Age': '86400',
        }
      });
    }

    // Only allow GET and HEAD requests
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      return new Response('Method not allowed', { status: 405 });
    }

    // Get the object from R2
    const objectKey = url.pathname.slice(1) || 'BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx';

    // Handle Range requests
    const range = request.headers.get('Range');
    const object = range
      ? await env.MODELS.get(objectKey, { range: request.headers })
      : await env.MODELS.get(objectKey);

    if (!object) {
      return new Response('Model not found', { status: 404 });
    }

    // Return with CORS headers and proper metadata
    const headers = new Headers();
    object.writeHttpMetadata(headers);
    headers.set('Access-Control-Allow-Origin', '*');
    headers.set('Access-Control-Allow-Methods', 'GET, HEAD, OPTIONS');
    headers.set('Access-Control-Allow-Headers', 'Range');
    headers.set('Access-Control-Expose-Headers', 'Content-Range, Content-Length, ETag');
    headers.set('Access-Control-Max-Age', '86400');
    headers.set('Cache-Control', 'public, max-age=86400, immutable');
    headers.set('Content-Type', 'application/octet-stream');
    headers.set('Accept-Ranges', 'bytes');
    headers.set('etag', object.httpEtag);

    // Handle partial content
    if (range) {
      headers.set('Content-Range', `bytes ${object.range.offset}-${object.range.offset + object.size - 1}/${object.size}`);
      return new Response(object.body, {
        status: 206,
        headers: headers
      });
    }

    headers.set('Content-Length', object.size.toString());
    return new Response(object.body, {
      headers: headers
    });
  }
}
