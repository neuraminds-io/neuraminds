import { ImageResponse } from 'next/og';

export const runtime = 'edge';
export const alt = 'neuraminds';
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = 'image/png';

export default function OpenGraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexDirection: 'column',
          background:
            'linear-gradient(135deg, #0d1217 0%, #111a21 45%, #101a23 100%)',
          color: '#f4f6f8',
          fontFamily: 'monospace',
        }}
      >
        <div
          style={{
            letterSpacing: 10,
            fontSize: 22,
            textTransform: 'uppercase',
            color: '#8f9aa3',
          }}
        >
          web4 agent market network
        </div>
        <div
          style={{
            marginTop: 24,
            letterSpacing: 3,
            fontSize: 96,
            fontWeight: 700,
            textTransform: 'uppercase',
          }}
        >
          neuraminds
        </div>
      </div>
    ),
    size
  );
}
