'use client';

import { useMemo } from 'react';
import { useAccount, useConnect, useDisconnect, useSwitchChain } from 'wagmi';

import { BASE_CHAIN_ID } from '@/lib/constants';

export function useBaseWallet() {
  const account = useAccount();
  const { connectAsync, connectors, isPending: connectPending } = useConnect();
  const { disconnect } = useDisconnect();
  const { switchChainAsync, isPending: switchPending } = useSwitchChain();

  const preferredConnector = useMemo(() => {
    if (connectors.length === 0) return undefined;
    return (
      connectors.find((connector) =>
        connector.name.toLowerCase().includes('metamask')
      ) || connectors[0]
    );
  }, [connectors]);

  const connect = async () => {
    const connector = preferredConnector;
    if (!connector) {
      throw new Error('No wallet connector available');
    }
    await connectAsync({ connector });
  };

  const ensureBaseChain = async () => {
    if (account.chainId === BASE_CHAIN_ID) {
      return;
    }
    await switchChainAsync({ chainId: BASE_CHAIN_ID });
  };

  return {
    enabled: true,
    address: account.address,
    isConnected: account.isConnected,
    chainId: account.chainId,
    isConnecting: connectPending,
    isSwitchingChain: switchPending,
    connect,
    disconnect,
    ensureBaseChain,
  };
}
