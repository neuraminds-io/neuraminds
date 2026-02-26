import { useState, useEffect, useCallback } from 'react';
import { useSignMessage } from 'wagmi';

import { useBaseWallet } from '@/hooks/useBaseWallet';
import { api } from '@/lib/api';
import { BASE_CHAIN_ID } from '@/lib/constants';

export function useAuth() {
  const baseWallet = useBaseWallet();
  const { signMessageAsync } = useSignMessage();

  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sessionRestored, setSessionRestored] = useState(false);

  const walletConnected = baseWallet.isConnected;
  const walletAddress = baseWallet.address;

  useEffect(() => {
    let mounted = true;
    api.restoreSession().then((restored) => {
      if (mounted) {
        setIsAuthenticated(restored);
        setSessionRestored(true);
      }
    });
    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    if (!walletConnected && sessionRestored) {
      api.logout().then(() => setIsAuthenticated(false));
    }
  }, [walletConnected, sessionRestored]);

  const login = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      if (!baseWallet.isConnected || !baseWallet.address) {
        throw new Error('Wallet not connected');
      }

      await baseWallet.ensureBaseChain();

      const nonce = await api.getSiweNonce();
      const issuedAt = new Date().toISOString();
      const domain = process.env.NEXT_PUBLIC_SIWE_DOMAIN || window.location.host;
      const uri = window.location.origin;
      const chainId = baseWallet.chainId ?? BASE_CHAIN_ID;
      const message = `${domain} wants you to sign in with your Ethereum account:\n${baseWallet.address}\n\nSign in to neuraminds\n\nURI: ${uri}\nVersion: 1\nChain ID: ${chainId}\nNonce: ${nonce}\nIssued At: ${issuedAt}`;

      const signature = await signMessageAsync({ message });
      await api.loginSiwe(baseWallet.address, signature, message);
      setIsAuthenticated(true);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Authentication failed';
      setError(msg);
      console.error('Login error:', err);
    } finally {
      setIsLoading(false);
    }
  }, [baseWallet, signMessageAsync]);

  const logout = useCallback(async () => {
    try {
      await api.logout();
    } catch {
      // Ignore logout errors
    }

    setIsAuthenticated(false);
    baseWallet.disconnect();
  }, [baseWallet]);

  return {
    isAuthenticated,
    isLoading,
    error,
    login,
    logout,
    walletConnected,
    walletAddress,
  };
}
