'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import { PageShell } from '@/components/layout';
import { Button, Card, Input, useToast } from '@/components/ui';
import {
  useAgents,
  useCreateAgent,
  useCreateExternalAgent,
  useExecuteAgent,
  useExecuteExternalAgent,
  useExternalAgents,
  useMarkets,
} from '@/hooks';
import { useBaseWallet } from '@/hooks/useBaseWallet';
import { api, type ExternalCredential } from '@/lib/api';
import { cn } from '@/lib/utils';

function truncateAddress(address: string) {
  if (!address) return '';
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

function statusLabel(status: string) {
  if (status === 'ready') return 'Ready';
  if (status === 'cooldown') return 'Cooldown';
  return 'Inactive';
}

export default function AgentsPage() {
  const { addToast } = useToast();
  const wallet = useBaseWallet();
  const [mode, setMode] = useState<'onchain' | 'external'>('onchain');

  const createAgent = useCreateAgent();
  const executeAgent = useExecuteAgent();
  const createExternalAgent = useCreateExternalAgent();
  const executeExternalAgent = useExecuteExternalAgent();

  const web4ApiBase = (process.env.NEXT_PUBLIC_API_URL || '/v1').replace(/\/$/, '');
  const { data: marketsData } = useMarkets({ limit: 100, sort: 'newest', source: 'internal' });
  const { data: externalMarketsData } = useMarkets({
    limit: 200,
    sort: 'newest',
    source: 'all',
    tradable: 'agent',
  });

  const [filterMarketId, setFilterMarketId] = useState('');
  const [filterActiveOnly, setFilterActiveOnly] = useState(true);
  const [filterExternalProvider, setFilterExternalProvider] = useState<'limitless' | 'polymarket' | ''>('');

  const [marketId, setMarketId] = useState('');
  const [isYes, setIsYes] = useState(true);
  const [priceBps, setPriceBps] = useState('5500');
  const [size, setSize] = useState('0.10');
  const [cadence, setCadence] = useState('300');
  const [expiryWindow, setExpiryWindow] = useState('1800');
  const [strategy, setStrategy] = useState('web4-research-signal-v1');
  const [externalName, setExternalName] = useState('external-agent');
  const [externalProvider, setExternalProvider] = useState<'limitless' | 'polymarket'>('limitless');
  const [externalMarketId, setExternalMarketId] = useState('');
  const [externalOutcome, setExternalOutcome] = useState<'yes' | 'no'>('yes');
  const [externalSide, setExternalSide] = useState<'buy' | 'sell'>('buy');
  const [externalPrice, setExternalPrice] = useState('0.55');
  const [externalQuantity, setExternalQuantity] = useState('10');
  const [externalCadence, setExternalCadence] = useState('300');
  const [externalStrategy, setExternalStrategy] = useState('cross-venue-momentum-v1');
  const [externalCredentialId, setExternalCredentialId] = useState('');
  const [externalCredentials, setExternalCredentials] = useState<ExternalCredential[]>([]);

  const marketOptions = useMemo(() => (marketsData?.data ?? []).filter((entry) => !entry.isExternal), [marketsData?.data]);
  const externalMarketOptions = useMemo(
    () => (externalMarketsData?.data ?? []).filter((entry) => entry.isExternal),
    [externalMarketsData?.data]
  );

  const { data: agentsData, isLoading } = useAgents({
    limit: 50,
    marketId: filterMarketId || undefined,
    active: filterActiveOnly ? true : undefined,
  });
  const { data: externalAgentsData, isLoading: isLoadingExternal } = useExternalAgents({
    limit: 50,
    provider: filterExternalProvider || undefined,
    active: filterActiveOnly ? true : undefined,
  });

  const agents = agentsData?.data ?? [];
  const externalAgents = externalAgentsData?.data ?? [];
  const selectedMarket = marketOptions.find((entry) => entry.id === marketId);
  const selectedExternalMarket = externalMarketOptions.find((entry) => entry.id === externalMarketId);
  const filteredExternalMarketOptions = externalMarketOptions.filter(
    (entry) => entry.provider === externalProvider
  );

  useEffect(() => {
    let canceled = false;

    async function loadCredentials() {
      try {
        const creds = await api.getExternalCredentials(externalProvider);
        if (canceled) return;
        setExternalCredentials(creds);
        if (creds.length > 0) {
          setExternalCredentialId((current) => current || creds[0].id);
        }
      } catch {
        if (!canceled) setExternalCredentials([]);
      }
    }

    void loadCredentials();
    return () => {
      canceled = true;
    };
  }, [externalProvider]);

  const onCreateAgent = async (event: React.FormEvent) => {
    event.preventDefault();

    if (!wallet.isConnected) {
      addToast('Connect wallet before launching an agent', 'error');
      return;
    }
    if (!marketId) {
      addToast('Select a market', 'error');
      return;
    }

    try {
      await createAgent.mutateAsync({
        marketId,
        isYes,
        priceBps: Number(priceBps),
        size: Number(size),
        cadence: Number(cadence),
        expiryWindow: Number(expiryWindow),
        strategy,
      });
      addToast('Agent launched onchain', 'success');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Agent launch failed';
      addToast(message, 'error');
    }
  };

  const onCreateExternalAgent = async (event: React.FormEvent) => {
    event.preventDefault();

    if (!wallet.isConnected) {
      addToast('Connect wallet before launching an external agent', 'error');
      return;
    }
    if (!externalMarketId) {
      addToast('Select an external market', 'error');
      return;
    }
    if (!externalCredentialId) {
      addToast('Select a credential', 'error');
      return;
    }

    try {
      await createExternalAgent.mutateAsync({
        name: externalName.trim() || 'external-agent',
        provider: externalProvider,
        market_id: externalMarketId,
        outcome: externalOutcome,
        side: externalSide,
        price: Number(externalPrice),
        quantity: Number(externalQuantity),
        cadence_seconds: Number(externalCadence),
        strategy: externalStrategy.trim() || 'external',
        credential_id: externalCredentialId,
        active: true,
      });
      addToast('External agent created', 'success');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'External agent launch failed';
      addToast(message, 'error');
    }
  };

  const onExecuteAgent = async (agentId: string) => {
    if (!wallet.isConnected) {
      addToast('Connect wallet before executing an agent', 'error');
      return;
    }

    try {
      await executeAgent.mutateAsync(agentId);
      addToast(`Agent ${agentId} executed`, 'success');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Execution failed';
      addToast(message, 'error');
    }
  };

  const onExecuteExternalAgent = async (agentId: string) => {
    try {
      await executeExternalAgent.mutateAsync({ agentId, force: true });
      addToast(`External agent ${agentId} executed`, 'success');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'External execution failed';
      addToast(message, 'error');
    }
  };

  return (
    <PageShell>
      <section className="mb-6">
        <h1 className="text-2xl font-semibold text-text-primary">Web4 Agent Grid</h1>
        <p className="text-sm text-text-secondary mt-2 max-w-3xl">
          Launch autonomous market agents, monitor execution windows, and operate machine-native
          strategies on Base.
        </p>
      </section>

      <section className="mb-6">
        <div className="inline-flex border border-border">
          <button
            type="button"
            onClick={() => setMode('onchain')}
            className={cn(
              'h-9 px-4 text-sm border-r border-border',
              mode === 'onchain' ? 'text-accent bg-accent/10' : 'text-text-secondary'
            )}
          >
            Onchain Agents
          </button>
          <button
            type="button"
            onClick={() => setMode('external')}
            className={cn(
              'h-9 px-4 text-sm',
              mode === 'external' ? 'text-accent bg-accent/10' : 'text-text-secondary'
            )}
          >
            External Agents
          </button>
        </div>
      </section>

      {mode === 'onchain' ? (
        <>
          <section className="grid lg:grid-cols-2 gap-6 mb-8">
            <Card>
              <h2 className="text-lg font-semibold mb-4">Launch Agent</h2>

              <form onSubmit={onCreateAgent} className="space-y-3">
                <div>
                  <label className="block text-sm font-medium text-text-primary mb-1">Market</label>
                  <select
                    value={marketId}
                    onChange={(event) => setMarketId(event.target.value)}
                    className="h-10 w-full border border-border bg-bg-secondary px-3 text-sm text-text-primary"
                  >
                    <option value="">Select market</option>
                    {marketOptions.map((market) => (
                      <option key={market.id} value={market.id}>
                        #{market.id} {market.question}
                      </option>
                    ))}
                  </select>
                  {selectedMarket ? (
                    <p className="text-xs text-text-muted mt-1">
                      Trading closes {new Date(selectedMarket.tradingEnd).toLocaleString()}
                    </p>
                  ) : null}
                </div>

                <div className="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    className={cn(
                      'h-10 border text-sm font-medium',
                      isYes ? 'border-bid text-bid bg-bid-muted' : 'border-border text-text-secondary'
                    )}
                    onClick={() => setIsYes(true)}
                  >
                    YES Agent
                  </button>
                  <button
                    type="button"
                    className={cn(
                      'h-10 border text-sm font-medium',
                      !isYes ? 'border-ask text-ask bg-ask-muted' : 'border-border text-text-secondary'
                    )}
                    onClick={() => setIsYes(false)}
                  >
                    NO Agent
                  </button>
                </div>

                <div className="grid sm:grid-cols-2 gap-3">
                  <Input
                    label="Price (bps)"
                    type="number"
                    value={priceBps}
                    onChange={(event) => setPriceBps(event.target.value)}
                    min="1"
                    max="9999"
                  />
                  <Input
                    label="Order Size (USDC)"
                    type="number"
                    value={size}
                    onChange={(event) => setSize(event.target.value)}
                    step="0.01"
                    min="0.01"
                  />
                  <Input
                    label="Cadence (sec)"
                    type="number"
                    value={cadence}
                    onChange={(event) => setCadence(event.target.value)}
                    min="1"
                  />
                  <Input
                    label="Expiry Window (sec)"
                    type="number"
                    value={expiryWindow}
                    onChange={(event) => setExpiryWindow(event.target.value)}
                    min="1"
                  />
                </div>

                <Input
                  label="Strategy"
                  value={strategy}
                  onChange={(event) => setStrategy(event.target.value)}
                  placeholder="signal-source + risk profile"
                />

                <Button type="submit" className="w-full" loading={createAgent.isPending}>
                  Launch Onchain Agent
                </Button>
              </form>
            </Card>

            <Card>
              <h2 className="text-lg font-semibold mb-4">Web4 Operating Notes</h2>
              <ul className="space-y-3 text-sm text-text-secondary">
                <li>Agents are persisted in `AgentRuntime` and executable by the network.</li>
                <li>Execution status is calculated from cadence and last execution timestamp.</li>
                <li>Use this directory as the control plane for autonomous market participation.</li>
              </ul>
              <div className="mt-6 pt-4 border-t border-border text-sm">
                <div className="flex flex-wrap gap-3">
                  <Link href="/docs/api" className="text-accent hover:text-accent-hover">
                    API Reference
                  </Link>
                  <a
                    href={`${web4ApiBase}/web4/mcp`}
                    className="text-accent hover:text-accent-hover"
                    target="_blank"
                    rel="noreferrer"
                  >
                    MCP Manifest
                  </a>
                  <a
                    href={`${web4ApiBase}/web4/agent-card`}
                    className="text-accent hover:text-accent-hover"
                    target="_blank"
                    rel="noreferrer"
                  >
                    Agent Card
                  </a>
                </div>
              </div>
            </Card>
          </section>

          <section>
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold">Agent Directory</h2>
              <div className="flex items-center gap-2">
                <select
                  value={filterMarketId}
                  onChange={(event) => setFilterMarketId(event.target.value)}
                  className="h-9 border border-border bg-bg-secondary px-2 text-sm"
                >
                  <option value="">All markets</option>
                  {marketOptions.map((market) => (
                    <option key={market.id} value={market.id}>
                      #{market.id}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  onClick={() => setFilterActiveOnly((prev) => !prev)}
                  className={cn(
                    'h-9 px-3 border text-sm',
                    filterActiveOnly
                      ? 'border-accent text-accent bg-accent/10'
                      : 'border-border text-text-secondary'
                  )}
                >
                  Active only
                </button>
              </div>
            </div>

            {isLoading ? (
              <Card>Loading agents...</Card>
            ) : agents.length === 0 ? (
              <Card>No agents found for current filter.</Card>
            ) : (
              <div className="grid gap-3">
                {agents.map((agent) => (
                  <Card key={agent.id} className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="text-sm text-text-muted">#{agent.id}</span>
                        <span
                          className={cn(
                            'text-xs px-2 py-1 border',
                            agent.status === 'ready'
                              ? 'border-bid text-bid'
                              : agent.status === 'cooldown'
                                ? 'border-border-hover text-text-secondary'
                                : 'border-border text-text-muted'
                          )}
                        >
                          {statusLabel(agent.status)}
                        </span>
                      </div>
                      <p className="text-sm text-text-primary">
                        Market #{agent.marketId} · {agent.isYes ? 'YES' : 'NO'} · {agent.priceBps} bps
                      </p>
                      <p className="text-xs text-text-muted">
                        Owner {truncateAddress(agent.owner)} · Size {Number(agent.size) / 1_000_000} USDC · Cadence {agent.cadence}s
                      </p>
                      {agent.identityTier !== undefined || agent.reputationScoreBps !== undefined ? (
                        <p className="text-xs text-text-muted">
                          Identity {agent.identityTier ?? 'n/a'} · Reputation {agent.reputationScoreBps ?? 'n/a'} bps
                        </p>
                      ) : null}
                      <p className="text-xs text-text-muted">Strategy: {agent.strategy || 'n/a'}</p>
                    </div>

                    <div className="flex items-center gap-2">
                      <Link href={`/markets/${encodeURIComponent(agent.marketId)}`} className="h-9 px-3 border border-border text-sm flex items-center">
                        Open Market
                      </Link>
                      <Button
                        type="button"
                        variant={agent.isYes ? 'bid' : 'ask'}
                        size="sm"
                        disabled={!agent.canExecute || executeAgent.isPending}
                        loading={executeAgent.isPending}
                        onClick={() => onExecuteAgent(agent.id)}
                      >
                        Execute
                      </Button>
                    </div>
                  </Card>
                ))}
              </div>
            )}
          </section>
        </>
      ) : (
        <>
          <section className="grid lg:grid-cols-2 gap-6 mb-8">
            <Card>
              <h2 className="text-lg font-semibold mb-4">Launch External Agent</h2>
              <form onSubmit={onCreateExternalAgent} className="space-y-3">
                <Input
                  label="Name"
                  value={externalName}
                  onChange={(event) => setExternalName(event.target.value)}
                />
                <div>
                  <label className="block text-sm font-medium text-text-primary mb-1">Provider</label>
                  <select
                    value={externalProvider}
                    onChange={(event) => setExternalProvider(event.target.value as 'limitless' | 'polymarket')}
                    className="h-10 w-full border border-border bg-bg-secondary px-3 text-sm text-text-primary"
                  >
                    <option value="limitless">limitless</option>
                    <option value="polymarket">polymarket</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-text-primary mb-1">Market</label>
                  <select
                    value={externalMarketId}
                    onChange={(event) => setExternalMarketId(event.target.value)}
                    className="h-10 w-full border border-border bg-bg-secondary px-3 text-sm text-text-primary"
                  >
                    <option value="">Select market</option>
                    {filteredExternalMarketOptions.map((market) => (
                      <option key={market.id} value={market.id}>
                        {market.id} {market.question}
                      </option>
                    ))}
                  </select>
                  {selectedExternalMarket ? (
                    <p className="text-xs text-text-muted mt-1">
                      Chain {selectedExternalMarket.chainId} · closes {new Date(selectedExternalMarket.tradingEnd).toLocaleString()}
                    </p>
                  ) : null}
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-sm font-medium text-text-primary mb-1">Outcome</label>
                    <select
                      value={externalOutcome}
                      onChange={(event) => setExternalOutcome(event.target.value as 'yes' | 'no')}
                      className="h-10 w-full border border-border bg-bg-secondary px-3 text-sm text-text-primary"
                    >
                      <option value="yes">yes</option>
                      <option value="no">no</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-text-primary mb-1">Side</label>
                    <select
                      value={externalSide}
                      onChange={(event) => setExternalSide(event.target.value as 'buy' | 'sell')}
                      className="h-10 w-full border border-border bg-bg-secondary px-3 text-sm text-text-primary"
                    >
                      <option value="buy">buy</option>
                      <option value="sell">sell</option>
                    </select>
                  </div>
                </div>

                <div className="grid sm:grid-cols-2 gap-3">
                  <Input
                    label="Price"
                    type="number"
                    value={externalPrice}
                    onChange={(event) => setExternalPrice(event.target.value)}
                    min="0.01"
                    max="0.99"
                    step="0.01"
                  />
                  <Input
                    label="Quantity"
                    type="number"
                    value={externalQuantity}
                    onChange={(event) => setExternalQuantity(event.target.value)}
                    min="0.01"
                    step="0.01"
                  />
                  <Input
                    label="Cadence (sec)"
                    type="number"
                    value={externalCadence}
                    onChange={(event) => setExternalCadence(event.target.value)}
                    min="1"
                  />
                  <Input
                    label="Credential ID"
                    value={externalCredentialId}
                    onChange={(event) => setExternalCredentialId(event.target.value)}
                    placeholder={externalCredentials[0]?.id || 'credential id'}
                  />
                </div>

                <Input
                  label="Strategy"
                  value={externalStrategy}
                  onChange={(event) => setExternalStrategy(event.target.value)}
                />

                <Button type="submit" className="w-full" loading={createExternalAgent.isPending}>
                  Launch External Agent
                </Button>
              </form>
            </Card>

            <Card>
              <h2 className="text-lg font-semibold mb-4">External Execution Notes</h2>
              <ul className="space-y-3 text-sm text-text-secondary">
                <li>External agents use BYOK credentials and venue-native execution paths.</li>
                <li>Funding and allowance checks are surfaced via preflight on each execution intent.</li>
                <li>Launch scope is binary YES/NO markets only.</li>
              </ul>
            </Card>
          </section>

          <section>
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold">External Agent Directory</h2>
              <div className="flex items-center gap-2">
                <select
                  value={filterExternalProvider}
                  onChange={(event) => setFilterExternalProvider(event.target.value as 'limitless' | 'polymarket' | '')}
                  className="h-9 border border-border bg-bg-secondary px-2 text-sm"
                >
                  <option value="">All providers</option>
                  <option value="limitless">limitless</option>
                  <option value="polymarket">polymarket</option>
                </select>
                <button
                  type="button"
                  onClick={() => setFilterActiveOnly((prev) => !prev)}
                  className={cn(
                    'h-9 px-3 border text-sm',
                    filterActiveOnly
                      ? 'border-accent text-accent bg-accent/10'
                      : 'border-border text-text-secondary'
                  )}
                >
                  Active only
                </button>
              </div>
            </div>

            {isLoadingExternal ? (
              <Card>Loading external agents...</Card>
            ) : externalAgents.length === 0 ? (
              <Card>No external agents found for current filter.</Card>
            ) : (
              <div className="grid gap-3">
                {externalAgents.map((agent) => (
                  <Card key={agent.id} className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
                    <div className="space-y-1">
                      <p className="text-sm text-text-primary">
                        {agent.name} · {agent.provider} · {agent.market_id}
                      </p>
                      <p className="text-xs text-text-muted">
                        {agent.outcome.toUpperCase()} {agent.side.toUpperCase()} · price {agent.price} · qty {agent.quantity}
                      </p>
                      <p className="text-xs text-text-muted">Cadence {agent.cadence_seconds}s · Strategy {agent.strategy}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Link href={`/markets/${encodeURIComponent(agent.market_id)}`} className="h-9 px-3 border border-border text-sm flex items-center">
                        Open Market
                      </Link>
                      <Button
                        type="button"
                        size="sm"
                        loading={executeExternalAgent.isPending}
                        onClick={() => onExecuteExternalAgent(agent.id)}
                      >
                        Execute
                      </Button>
                    </div>
                  </Card>
                ))}
              </div>
            )}
          </section>
        </>
      )}
    </PageShell>
  );
}
