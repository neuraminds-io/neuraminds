export enum NeuraTier {
  Unverified = 0,
  Bronze = 1,
  Silver = 2,
  Gold = 3,
  Platinum = 4,
}

export const TIER_TO_RESPONSE: Record<NeuraTier, number> = {
  [NeuraTier.Unverified]: 20,
  [NeuraTier.Bronze]: 40,
  [NeuraTier.Silver]: 60,
  [NeuraTier.Gold]: 80,
  [NeuraTier.Platinum]: 95,
};

export const responseToTier = (response: number): NeuraTier => {
  if (response >= 90) return NeuraTier.Platinum;
  if (response >= 75) return NeuraTier.Gold;
  if (response >= 50) return NeuraTier.Silver;
  if (response >= 25) return NeuraTier.Bronze;
  return NeuraTier.Unverified;
};

export interface GlobalAgentId {
  namespace: 'eip155';
  chainId: number;
  registry: string;
  agentId: bigint;
  raw: string;
}

export interface MetadataEntry {
  key: string;
  value: Uint8Array;
}

export interface ValidationStatus {
  validatorAddress: string;
  agentId: bigint;
  response: number;
  responseHash: string;
  tag: string;
  lastUpdate: number;
}

export interface ValidationSummary {
  count: number;
  averageResponse: number;
}
