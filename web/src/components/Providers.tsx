'use client';

import { FC, ReactNode, useMemo } from 'react';
import {
  ConnectionProvider,
  WalletProvider,
} from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { WagmiProvider, createConfig, http } from 'wagmi';
import { base, baseSepolia } from 'wagmi/chains';
import { injected } from '@wagmi/core';

import { ErrorBoundary } from '@/components/ErrorBoundary';
import { ThemeProvider } from '@/components/ThemeProvider';
import { ToastProvider } from '@/components/ui';
import {
  BASE_CHAIN_ID,
  BASE_RPC_ENDPOINT,
  SOLANA_RPC_ENDPOINT,
} from '@/lib/constants';

import '@solana/wallet-adapter-react-ui/styles.css';

const getSolanaEndpoint = () => process.env.NEXT_PUBLIC_RPC_ENDPOINT || SOLANA_RPC_ENDPOINT;

const wagmiConfig = createConfig({
  chains: [base, baseSepolia],
  connectors: [injected()],
  transports: {
    [base.id]: http(BASE_CHAIN_ID === base.id ? BASE_RPC_ENDPOINT : undefined),
    [baseSepolia.id]: http(BASE_CHAIN_ID === baseSepolia.id ? BASE_RPC_ENDPOINT : undefined),
  },
  ssr: true,
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 10 * 1000,
      refetchOnWindowFocus: false,
    },
  },
});

interface ProvidersProps {
  children: ReactNode;
}

export const Providers: FC<ProvidersProps> = ({ children }) => {
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
    ],
    []
  );

  return (
    <ErrorBoundary>
      <ThemeProvider>
        <QueryClientProvider client={queryClient}>
          <WagmiProvider config={wagmiConfig}>
            <ConnectionProvider endpoint={getSolanaEndpoint()}>
              <WalletProvider wallets={wallets} autoConnect>
                <WalletModalProvider>
                  <ToastProvider>{children}</ToastProvider>
                </WalletModalProvider>
              </WalletProvider>
            </ConnectionProvider>
          </WagmiProvider>
        </QueryClientProvider>
      </ThemeProvider>
    </ErrorBoundary>
  );
};
