'use client';

import { useEffect, useState } from 'react';

import { useBaseWallet } from '@/hooks/useBaseWallet';
import { api, type BaseTokenState } from '@/lib/api';
import { PageShell } from '@/components/layout';
import { Card, Button } from '@/components/ui';
import {
  BASE_RPC_ENDPOINT,
} from '@/lib/constants';
import { truncateAddress } from '@/lib/utils';

function formatTokenSupply(totalSupplyHex: string, decimals: number): string {
  if (!totalSupplyHex.startsWith('0x')) return totalSupplyHex;

  const raw = BigInt(totalSupplyHex);
  let divisor = BigInt(1);
  for (let i = 0; i < decimals; i += 1) {
    divisor *= BigInt(10);
  }
  const whole = raw / divisor;
  const fraction = raw % divisor;

  if (fraction === BigInt(0)) {
    return whole.toString();
  }

  const paddedFraction = fraction.toString().padStart(decimals, '0').replace(/0+$/, '');
  return `${whole.toString()}.${paddedFraction}`;
}

export default function SettingsPage() {
  const baseWallet = useBaseWallet();

  const [baseTokenState, setBaseTokenState] = useState<BaseTokenState | null>(null);
  const [baseTokenError, setBaseTokenError] = useState<string | null>(null);

  const connected = baseWallet.isConnected;
  const walletAddress = baseWallet.address;

  useEffect(() => {
    let mounted = true;

    api
      .getBaseTokenState()
      .then((state) => {
        if (!mounted) return;
        setBaseTokenState(state);
        setBaseTokenError(null);
      })
      .catch((error) => {
        if (!mounted) return;
        const message = error instanceof Error ? error.message : 'Unable to load Base token state';
        setBaseTokenError(message);
      });

    return () => {
      mounted = false;
    };
  }, []);

  const disconnectWallet = () => {
    baseWallet.disconnect();
  };

  return (
    <PageShell>
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      {connected && walletAddress && (
        <Card className="mb-6">
          <h2 className="font-semibold mb-4">Wallet</h2>
          <div className="flex items-center justify-between">
            <div>
              <div className="text-text-secondary text-sm">Connected Address</div>
              <div className="font-mono">{truncateAddress(walletAddress)}</div>
            </div>
            <Button variant="danger" size="sm" onClick={disconnectWallet}>
              Disconnect
            </Button>
          </div>
        </Card>
      )}

      <Card className="mb-6">
        <h2 className="font-semibold mb-4">Preferences</h2>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="font-medium">Dark Mode</div>
              <div className="text-text-secondary text-sm">Always enabled</div>
            </div>
            <div className="w-12 h-6 bg-accent  relative">
              <div className="absolute right-1 top-1 w-4 h-4 bg-white " />
            </div>
          </div>
          <div className="flex items-center justify-between">
            <div>
              <div className="font-medium">Push Notifications</div>
              <div className="text-text-secondary text-sm">Price alerts and updates</div>
            </div>
            <div className="w-12 h-6 bg-bg-tertiary  relative">
              <div className="absolute left-1 top-1 w-4 h-4 bg-text-muted " />
            </div>
          </div>
        </div>
      </Card>

      <Card className="mb-6">
        <h2 className="font-semibold mb-4">Network</h2>
          <div className="space-y-2">
          <div className="flex items-center justify-between py-2">
            <span>Base</span>
            <span className="w-2 h-2 bg-accent " />
          </div>
          <div className="text-text-secondary text-sm break-all">
            RPC: {BASE_RPC_ENDPOINT}
          </div>
          {baseTokenState && (
            <>
              <div className="text-text-secondary text-sm break-all">
                Token: {baseTokenState.token_address}
              </div>
              <div className="text-text-secondary text-sm">
                Supply: {formatTokenSupply(baseTokenState.total_supply_hex, baseTokenState.decimals)}
              </div>
            </>
          )}
          {baseTokenError && (
            <div className="text-red-400 text-sm">Token state unavailable: {baseTokenError}</div>
          )}
        </div>
      </Card>

      <Card>
        <h2 className="font-semibold mb-4">About</h2>
        <div className="space-y-3 text-sm">
          <div className="flex justify-between">
            <span className="text-text-secondary">Version</span>
            <span>1.0.0</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-secondary">Build</span>
            <span>dev</span>
          </div>
        </div>
      </Card>
    </PageShell>
  );
}
