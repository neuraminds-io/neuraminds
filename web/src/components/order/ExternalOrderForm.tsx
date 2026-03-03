'use client';

import { useEffect, useMemo, useState } from 'react';
import { Button, Card, Input, useToast } from '@/components/ui';
import { api, type ExternalCredential, type ExternalOrderRecord } from '@/lib/api';
import { useBaseWallet } from '@/hooks/useBaseWallet';
import type { Market, Outcome, OrderSide } from '@/types';
import { cn } from '@/lib/utils';

export interface ExternalOrderFormProps {
  market: Market;
  onSuccess?: () => void;
}

function providerFromMarket(market: Market): 'limitless' | 'polymarket' {
  return market.provider === 'polymarket' ? 'polymarket' : 'limitless';
}

function signedOrderFallback(input: string): Record<string, unknown> {
  if (!input.trim()) {
    return {};
  }
  return JSON.parse(input);
}

export function ExternalOrderForm({ market, onSuccess }: ExternalOrderFormProps) {
  const { addToast } = useToast();
  const baseWallet = useBaseWallet();
  const [outcome, setOutcome] = useState<Outcome>('yes');
  const [side, setSide] = useState<OrderSide>('buy');
  const [price, setPrice] = useState(String(Math.round(market.yesPrice * 100) / 100));
  const [quantity, setQuantity] = useState('10');
  const [credentialId, setCredentialId] = useState('');
  const [credentials, setCredentials] = useState<ExternalCredential[]>([]);
  const [signedOrderJson, setSignedOrderJson] = useState('');
  const [isLoadingCredentials, setIsLoadingCredentials] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [preflight, setPreflight] = useState<Record<string, unknown> | null>(null);
  const [lastOrder, setLastOrder] = useState<ExternalOrderRecord | null>(null);

  const provider = providerFromMarket(market);
  const currentPrice = outcome === 'yes' ? market.yesPrice : market.noPrice;

  useEffect(() => {
    let canceled = false;

    async function load() {
      setIsLoadingCredentials(true);
      try {
        const list = await api.getExternalCredentials(provider);
        if (canceled) return;
        setCredentials(list);
        if (!credentialId && list.length > 0) {
          setCredentialId(list[0].id);
        }
      } catch (error) {
        if (!canceled) {
          const message = error instanceof Error ? error.message : 'Failed to load credentials';
          addToast(message, 'error');
        }
      } finally {
        if (!canceled) {
          setIsLoadingCredentials(false);
        }
      }
    }

    void load();

    return () => {
      canceled = true;
    };
  }, [addToast, provider, credentialId]);

  const preflightChecks = useMemo(() => {
    const checks = preflight?.checks;
    return Array.isArray(checks) ? checks : [];
  }, [preflight]);

  const signTypedData = async (typedData: Record<string, unknown>) => {
    const ethereum = (window as unknown as { ethereum?: { request: (args: Record<string, unknown>) => Promise<unknown> } }).ethereum;
    if (!ethereum || !baseWallet.address) {
      throw new Error('Connect wallet to sign typed data');
    }

    const signature = await ethereum.request({
      method: 'eth_signTypedData_v4',
      params: [baseWallet.address, JSON.stringify(typedData)],
    });

    return String(signature || '');
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();

    const numericPrice = Number(price || currentPrice);
    const numericQuantity = Number(quantity);
    if (!Number.isFinite(numericPrice) || numericPrice <= 0 || numericPrice >= 1) {
      addToast('Price must be between 0 and 1', 'error');
      return;
    }
    if (!Number.isFinite(numericQuantity) || numericQuantity <= 0) {
      addToast('Quantity must be greater than zero', 'error');
      return;
    }
    if (!credentialId) {
      addToast('Select a credential first', 'error');
      return;
    }

    setIsSubmitting(true);
    try {
      const intent = await api.createExternalOrderIntent({
        provider,
        market_id: market.id,
        outcome,
        side,
        price: numericPrice,
        quantity: numericQuantity,
        credential_id: credentialId,
      });
      setPreflight(intent.preflight || null);

      let signedOrder: Record<string, unknown>;
      if (signedOrderJson.trim()) {
        signedOrder = signedOrderFallback(signedOrderJson);
      } else {
        const signature = await signTypedData(intent.typed_data);
        signedOrder = {
          typedData: intent.typed_data,
          signature,
        };
      }

      const order = await api.submitExternalOrder({
        intent_id: intent.id,
        signed_order: signedOrder,
        credential_id: credentialId,
      });

      setLastOrder(order);
      addToast('External order submitted', 'success');
      onSuccess?.();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'External order submit failed';
      addToast(message, 'error');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Card className="!p-4 sm:!p-6">
      <h3 className="font-display font-semibold text-lg mb-4">External Trade</h3>
      <p className="text-xs text-text-muted mb-4">
        Venue: {provider} · Chain {market.chainId}
      </p>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            onClick={() => setOutcome('yes')}
            className={cn(
              'py-2 border text-sm',
              outcome === 'yes' ? 'border-bid text-bid bg-bid-muted' : 'border-border text-text-secondary'
            )}
          >
            YES
          </button>
          <button
            type="button"
            onClick={() => setOutcome('no')}
            className={cn(
              'py-2 border text-sm',
              outcome === 'no' ? 'border-ask text-ask bg-ask-muted' : 'border-border text-text-secondary'
            )}
          >
            NO
          </button>
        </div>

        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            onClick={() => setSide('buy')}
            className={cn('py-2 border text-sm', side === 'buy' ? 'border-accent text-accent' : 'border-border text-text-secondary')}
          >
            Buy
          </button>
          <button
            type="button"
            onClick={() => setSide('sell')}
            className={cn('py-2 border text-sm', side === 'sell' ? 'border-accent text-accent' : 'border-border text-text-secondary')}
          >
            Sell
          </button>
        </div>

        <Input
          label="Credential"
          value={credentialId}
          onChange={(event) => setCredentialId(event.target.value)}
          placeholder={isLoadingCredentials ? 'Loading credentials...' : 'Credential ID'}
          disabled={isLoadingCredentials}
        />

        <Input
          type="number"
          label="Price"
          value={price}
          onChange={(event) => setPrice(event.target.value)}
          min="0.01"
          max="0.99"
          step="0.01"
          placeholder={String(currentPrice)}
        />

        <Input
          type="number"
          label="Quantity"
          value={quantity}
          onChange={(event) => setQuantity(event.target.value)}
          min="0.01"
          step="0.01"
        />

        <Input
          label="Signed Order JSON (optional)"
          value={signedOrderJson}
          onChange={(event) => setSignedOrderJson(event.target.value)}
          placeholder='{"typedData":{...},"signature":"0x..."}'
        />

        <Button type="submit" className="w-full" loading={isSubmitting} disabled={isSubmitting}>
          Submit External Order
        </Button>
      </form>

      {credentials.length > 0 ? (
        <div className="mt-4 text-xs text-text-muted">
          Credentials loaded: {credentials.length}
        </div>
      ) : null}

      {preflightChecks.length > 0 ? (
        <div className="mt-4 space-y-2">
          <p className="text-xs text-text-muted">Preflight checks</p>
          {preflightChecks.map((entry, index) => (
            <div key={index} className="text-xs border border-border p-2">
              {String((entry as { message?: string }).message || 'Check')}
            </div>
          ))}
        </div>
      ) : null}

      {lastOrder ? (
        <div className="mt-4 text-xs border border-border p-2">
          <div>Status: {lastOrder.status}</div>
          <div>Provider Order: {lastOrder.provider_order_id || 'n/a'}</div>
        </div>
      ) : null}
    </Card>
  );
}
