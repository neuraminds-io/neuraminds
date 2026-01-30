import { PublicKey } from '@solana/web3.js';

// Program IDs - Devnet
export const PROGRAM_IDS = {
  market: new PublicKey('98jqxMe88XGjXzCY3bwV1Kuqzj32fcwdhPZa193RUffQ'),
  orderbook: new PublicKey('59LqZtVU2YBrhv8B2E1iASJMzcyBHWhY2JuaJsCXkAS8'),
  privacy: new PublicKey('9QGtHZJvmjMKTME1s3mVfNXtGpEdXDQZJTxsxqve9GsL'),
} as const;

// RPC Endpoints
export const RPC_ENDPOINTS = {
  devnet: 'https://api.devnet.solana.com',
  mainnet: 'https://mainnet.helius-rpc.com/?api-key=c4a9b21c-8650-451d-9572-8c8a3543a0be',
} as const;

// Current network
export const NETWORK = 'devnet' as const;
export const RPC_ENDPOINT = RPC_ENDPOINTS[NETWORK];

// IDL imports
export { default as MarketIDL } from './idl/polyguard_market.json';
export { default as OrderbookIDL } from './idl/polyguard_orderbook.json';
export { default as PrivacyIDL } from './idl/polyguard_privacy.json';
