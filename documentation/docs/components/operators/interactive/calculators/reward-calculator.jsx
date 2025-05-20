import React, { useState } from 'react'

export default function RewardsCalculator() {
  const [a, setA] = useState(5241)
  const [b, setB] = useState(0)
  const [c, setC] = useState(0)
  const [d, setD] = useState(0)
  const [e, setE] = useState(1034081)

  const result =
    e !== 0
      ? `${(
          a * b * c * ((1 / 240) + 0.3 * ((d / e) / 240)) * 1 / (1 + 0.3)
        ).toFixed(4)} NYM`
      : '—'

  return (
    <div
      style={{
        margin: '1.5em 0',
        padding: '1em',
        border: '1px solid #9ca3af',
        borderRadius: '8px',
        maxWidth: '600px',
        backgroundColor: '#162B2C',
        color: '#9ca3af',
        fontSize: '0.95rem',
      }}
    >
    <h3
      style={{
        marginTop: 0,
        marginBottom: '0.75em',
        color: '#9ca3af',
        fontSize: '1.25rem',
        fontWeight: '600',
      }}
    >
      Epoch Reward Calculator
    </h3>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr auto',
          gap: '0.75em 1em',
          alignItems: 'center',
        }}
      >
        <label htmlFor="a">
          {' '}
          <a
            href="https://validator.nymtech.net/api/v1/epoch/reward_params"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: 'inherit', textDecoration: 'underline' }}
          >
            Current epoch reward budget
          </a>{' '}
          (NYM):
        </label>
        <input
          id="a"
          type="number"
          value={a}
          onChange={(e) => setA(Number(e.target.value))}
        />

        <label htmlFor="b">Node performance score:</label>
        <input
          id="b"
          type="number"
          value={b}
          onChange={(e) => setB(Number(e.target.value))}
        />

        <label htmlFor="c">Node stake saturation:</label>
        <input
          id="c"
          type="number"
          value={c}
          onChange={(e) => setC(Number(e.target.value))}
        />

        <label htmlFor="d">Node self bond size (NYM):</label>
        <input
          id="d"
          type="number"
          value={d}
          onChange={(e) => setD(Number(e.target.value))}
        />

        <label htmlFor="e">
          <a
            href="https://validator.nymtech.net/api/v1/epoch/reward_params"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: 'inherit', textDecoration: 'underline' }}
          >
            Current stake saturation point
          </a>{' '}
          (NYM):
        </label>
        <input
          id="e"
          type="number"
          value={e}
          onChange={(e) => setE(Number(e.target.value))}
        />
      </div>

      <p style={{ marginTop: '1.5em' }}>
        <strong>Node epoch rewards (if active):</strong>
        <br />
        {result}
      </p>
    </div>
  )
}
