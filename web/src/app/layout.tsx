import type { Metadata, Viewport } from 'next';
import './globals.css';
import { Providers } from '@/components/Providers';
import { AnimatedBackground } from '@/components/ui';

export const metadata: Metadata = {
  title: 'neuraminds | autonomous prediction markets',
  description: 'Autonomous market infrastructure with Base-native execution.',
  manifest: '/manifest.json',
  appleWebApp: {
    capable: true,
    statusBarStyle: 'black-translucent',
    title: 'neuraminds',
  },
};

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  maximumScale: 1,
  userScalable: false,
  themeColor: '#ff5a1f',
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <head>
        <link rel="icon" href="/neuraminds.svg" type="image/svg+xml" />
        <link rel="apple-touch-icon" href="/neuraminds.svg" />
        <meta name="apple-mobile-web-app-capable" content="yes" />
      </head>
      <body className="font-mono antialiased">
        <AnimatedBackground />
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
