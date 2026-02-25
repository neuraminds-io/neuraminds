import { useState, useEffect, useCallback } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { useSignMessage } from 'wagmi';
import bs58 from 'bs58';

import { useBaseWallet } from '@/hooks/useBaseWallet';
import { api } from '@/lib/api';
import { BASE_CHAIN_ID, CHAIN_MODE } from '@/lib/constants';

export function useAuth() {
  const solanaWallet = useWallet();
  const baseWallet = useBaseWallet();
  const { signMessageAsync } = useSignMessage();

  const isBaseMode = CHAIN_MODE === 'base';

  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sessionRestored, setSessionRestored] = useState(false);

  const walletConnected = isBaseMode ? baseWallet.isConnected : solanaWallet.connected;
  const walletAddress = isBaseMode ? baseWallet.address : solanaWallet.publicKey?.toBase58();

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
      if (isBaseMode) {
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
        return;
      }

      if (!solanaWallet.publicKey || !solanaWallet.signMessage) {
        throw new Error('Wallet not connected');
      }

      const nonce = await api.getNonce();
      const timestamp = Math.floor(Date.now() / 1000);
      const wallet = solanaWallet.publicKey.toBase58();
      const message = `polyguard:${wallet}:${timestamp}:${nonce}`;

      const encoded = new TextEncoder().encode(message);
      const signature = await solanaWallet.signMessage(encoded);
      const signatureBase58 = bs58.encode(signature);

      await api.login(wallet, signatureBase58, message);
      setIsAuthenticated(true);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Authentication failed';
      setError(msg);
      console.error('Login error:', err);
    } finally {
      setIsLoading(false);
    }
  }, [baseWallet, isBaseMode, signMessageAsync, solanaWallet]);

  const logout = useCallback(async () => {
    try {
      await api.logout();
    } catch {
      // Ignore logout errors
    }

    setIsAuthenticated(false);

    if (isBaseMode) {
      baseWallet.disconnect();
      return;
    }

    solanaWallet.disconnect();
  }, [baseWallet, isBaseMode, solanaWallet]);

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
