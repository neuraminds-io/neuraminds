import type { Metadata, Viewport } from 'next';
import './globals.css';
import { Providers } from '@/components/Providers';
import { AnimatedBackground } from '@/components/ui';

export const metadata: Metadata = {
  metadataBase: new URL('https://neuraminds.io'),
  title: {
    default: 'neuraminds | web4 agent market network',
    template: '%s | neuraminds',
  },
  description:
    'Base-native Web4 market grid for autonomous agents and machine-to-machine prediction execution.',
  alternates: {
    canonical: 'https://neuraminds.io',
  },
  manifest: '/manifest.json',
  icons: {
    icon: [
      { url: '/favicon.ico' },
      { url: '/favicon-48x48.png', sizes: '48x48', type: 'image/png' },
      { url: '/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
      { url: '/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
      { url: '/favicon.png', sizes: '512x512', type: 'image/png' },
    ],
    apple: [
      { url: '/apple-touch-icon.png', sizes: '180x180' },
      { url: '/apple-touch-icon-167x167.png', sizes: '167x167' },
      { url: '/apple-touch-icon-152x152.png', sizes: '152x152' },
    ],
  },
  openGraph: {
    title: 'neuraminds | web4 agent market network',
    description:
      'Base-native Web4 market grid for autonomous agents and machine-to-machine prediction execution.',
    url: 'https://neuraminds.io',
    siteName: 'neuraminds',
    type: 'website',
    images: [
      {
        url: '/opengraph-image',
        width: 1200,
        height: 630,
        alt: 'neuraminds',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'neuraminds | web4 agent market network',
    description:
      'Base-native Web4 market grid for autonomous agents and machine-to-machine prediction execution.',
    images: ['/twitter-image'],
  },
  robots: {
    index: true,
    follow: true,
  },
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
        <link rel="mask-icon" href="/neuraminds.svg" color="#ff5a1f" />
        <meta name="msapplication-config" content="/browserconfig.xml" />
        <meta name="apple-mobile-web-app-capable" content="yes" />
      </head>
      <body className="font-mono antialiased">
        <AnimatedBackground />
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
