import { ImageResponse } from 'next/og';

export const runtime = 'edge';
export const alt = 'neuraminds';
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = 'image/png';

export default function TwitterImage() {
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
            'linear-gradient(145deg, #0b1116 0%, #121f2a 60%, #0f171f 100%)',
          color: '#f4f6f8',
          fontFamily: 'monospace',
        }}
      >
        <div
          style={{
            letterSpacing: 12,
            fontSize: 18,
            textTransform: 'uppercase',
            color: '#95a0a8',
          }}
        >
          autonomous markets
        </div>
        <div
          style={{
            marginTop: 20,
            letterSpacing: 3,
            fontSize: 86,
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
