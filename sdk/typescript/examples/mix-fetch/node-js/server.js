const express = require('express');
const { mixFetch } = require('@nymproject/mix-fetch-node-commonjs');

const app = express();
app.use(express.static('public'));

app.get('/nym-fetch', async (req, res) => {
  try {
    const args = {
      mode: 'unsafe-ignore-cors',
      headers: {
        'Content-Type': 'application/json',
      },
    };

    const url = req.query.url;

    if (!url) {
      return res.status(400).send('input a valid url');
    }

    const extra = {
      hiddenGateways: [
        {
          owner: 'n1ns3v70ul9gnl9l9fkyz8cyxfq75vjcmx8el0t3',
          host: 'sandbox-gateway1.nymtech.net',
          explicitIp: '35.158.238.80',
          identityKey: 'HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua',
          sphinxKey: 'BoXeUD7ERGmzRauMjJD3itVNnQiH42ncUb6kcVLrb3dy',
        },
      ],
    };

    const mixFetchOptions = {
      nymApiUrl: 'https://sandbox-nym-api1.nymtech.net/api',
      preferredGateway: 'HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua',
      preferredNetworkRequester:
        'AzGdJ4MU78Ex22NEWfeycbN7bt3PFZr1MtKstAdhfELG.GSxnKnvKPjjQm3FdtsgG5KyhP6adGbPHRmFWDH4XfUpP@HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua',
      mixFetchOverride: {
        requestTimeoutMs: 60_000,
      },
      forceTls: false,
      extra,
    };

    const response = await mixFetch(url, args, mixFetchOptions);
    const json = await response.json();
    res.send(json);
  } catch (error) {
    console.log(error);
    res.status(500).send(error.message);
  }
});

app.listen(3000, () => console.log('Server running on port 3000'));
