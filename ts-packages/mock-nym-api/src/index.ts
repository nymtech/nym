/* eslint-disable no-console */
import express from 'express';
import dotenv from 'dotenv';
import { createProxyMiddleware } from 'http-proxy-middleware';
import fs from 'fs';

dotenv.config();

const app = express();
const port = process.env.PORT || 8000;

const { NYM_API_URL } = process.env;

if (!NYM_API_URL || NYM_API_URL.trim().length < 1) {
  throw new Error('Please specify a valid NYM_API_URL in `.env` or as an environment variable');
}

// proxy the Nym API and only override some routes
const proxy = createProxyMiddleware(['/api/**', '/swagger/**'], {
  target: NYM_API_URL,
  changeOrigin: true,
});

/**
 * Return a single custom gateway, from a static file and modify some the fields
 */
app.get('/api/v1/gateways', (req, res) => {
  const customGateways = JSON.parse(fs.readFileSync('./mocks/custom-gateway.json').toString());

  // modify custom gateway
  customGateways[0].gateway.sphinx_key += '-ccc';
  customGateways[0].gateway.identity_key += '-ddd';

  res.json(customGateways);
});

/**
 * Returns only 3 mixnodes from a static file
 */
app.get('/api/v1/mixnodes', (req, res) => {
  const customMixnodes = JSON.parse(fs.readFileSync('./mocks/mixnodes.json').toString());
  res.json(customMixnodes);
});

// start the Express server
app.use(proxy);
app.listen(port, () => {
  console.log(`[API] Nym API mock is running at http://localhost:${port} and proxying ${NYM_API_URL}`);
});
