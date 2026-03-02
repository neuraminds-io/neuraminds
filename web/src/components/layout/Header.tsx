'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Search } from 'lucide-react';
import { useBaseWallet } from '@/hooks/useBaseWallet';
import { useSolanaWallet } from '@/hooks/useSolanaWallet';
import { BrandLogo } from '@/components/layout/BrandLogo';
import { ThemeToggle } from '@/components/ui/ThemeToggle';
import { CHAIN_MODE } from '@/lib/constants';
import { cn } from '@/lib/utils';

const navLinks = [
  { href: '/markets', label: 'Markets' },
  { href: '/agents', label: 'Agents' },
  { href: '/portfolio', label: 'Portfolio' },
  { href: '/api', label: 'API' },
];

function ConnectWalletButton() {
  const baseWallet = useBaseWallet();
  const solanaWallet = useSolanaWallet();
  const baseEnabled = CHAIN_MODE === 'base' || CHAIN_MODE === 'dual';
  const solanaEnabled = CHAIN_MODE === 'solana' || CHAIN_MODE === 'dual';
  const solanaAvailable = solanaWallet.enabled;
  const singleMode = baseEnabled !== solanaEnabled;

  const handleBaseClick = () => {
    if (baseWallet.isConnected) {
      baseWallet.disconnect();
      return;
    }
    baseWallet.connect().catch((error) => {
      console.error('Base wallet connect failed:', error);
    });
  };

  const handleSolanaClick = () => {
    if (solanaWallet.isConnected) {
      solanaWallet.disconnect().catch((error) => {
        console.error('Solana wallet disconnect failed:', error);
      });
      return;
    }
    solanaWallet.connect().catch((error) => {
      console.error('Solana wallet connect failed:', error);
    });
  };

  const truncateAddress = (address: string) => {
    return `${address.slice(0, 4)}...${address.slice(-4)}`;
  };

  const installSolanaWallet = (
    <a
      href="https://phantom.app/"
      target="_blank"
      rel="noreferrer"
      className={cn(
        'h-9 px-5 text-sm font-medium inline-flex items-center',
        'border border-border text-text-primary bg-bg-secondary hover:bg-bg-hover transition-colors'
      )}
    >
      Install Phantom
    </a>
  );

  if (singleMode && baseEnabled) {
    return (
      <button
        onClick={handleBaseClick}
        className={cn(
          'h-9 px-5  text-sm font-medium',
          'bg-gradient-to-r from-accent to-[#ff8b5f]',
          'text-white',
          'hover:opacity-90 hover:shadow-lg hover:shadow-accent/25',
          'transition-all cursor-pointer'
        )}
      >
        {baseWallet.isConnected && baseWallet.address
          ? truncateAddress(baseWallet.address)
          : 'Connect Base'}
      </button>
    );
  }

  if (singleMode && solanaEnabled) {
    if (!solanaAvailable) {
      return installSolanaWallet;
    }
    return (
      <button
        onClick={handleSolanaClick}
        className={cn(
          'h-9 px-5  text-sm font-medium',
          'bg-gradient-to-r from-accent to-[#ff8b5f]',
          'text-white',
          'hover:opacity-90 hover:shadow-lg hover:shadow-accent/25',
          'transition-all cursor-pointer'
        )}
      >
        {solanaWallet.isConnected && solanaWallet.address
          ? truncateAddress(solanaWallet.address)
          : 'Connect Solana'}
      </button>
    );
  }

  return (
    <div className="flex items-center gap-2">
      {baseEnabled && (
        <button
          onClick={handleBaseClick}
          className={cn(
            'h-9 px-3 text-xs font-medium border border-border',
            'text-text-primary bg-bg-secondary hover:bg-bg-hover transition-colors'
          )}
        >
          {baseWallet.isConnected && baseWallet.address
            ? `Base ${truncateAddress(baseWallet.address)}`
            : 'Connect Base'}
        </button>
      )}
      {solanaEnabled && (
        solanaAvailable ? (
          <button
            onClick={handleSolanaClick}
            className={cn(
              'h-9 px-3 text-xs font-medium border border-border',
              'text-text-primary bg-bg-secondary hover:bg-bg-hover transition-colors'
            )}
          >
            {solanaWallet.isConnected && solanaWallet.address
              ? `Sol ${truncateAddress(solanaWallet.address)}`
              : 'Connect Solana'}
          </button>
        ) : (
          <a
            href="https://phantom.app/"
            target="_blank"
            rel="noreferrer"
            className={cn(
              'h-9 px-3 text-xs font-medium border border-border inline-flex items-center',
              'text-text-primary bg-bg-secondary hover:bg-bg-hover transition-colors'
            )}
          >
            Install Solana Wallet
          </a>
        )
      )}
    </div>
  );
}

export function Header() {
  const pathname = usePathname();

  return (
    <header className="sticky top-0 z-sticky bg-bg-primary border-b border-border">
      <div className="max-w-[1400px] mx-auto px-4 sm:px-6">
        <div className="relative flex items-center justify-between h-14">
          <div className="flex items-center gap-8">
            <Link href="/" className="flex items-center group">
              <BrandLogo />
            </Link>

            <nav className="hidden md:flex items-center gap-1">
              {navLinks.map(({ href, label }) => {
                const isActive = pathname === href || pathname.startsWith(href + '/');
                return (
                  <Link
                    key={href}
                    href={href}
                    className={cn(
                      'px-3 py-1.5  text-sm font-medium transition-colors',
                      isActive
                        ? 'text-text-primary bg-bg-secondary'
                        : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'
                    )}
                  >
                    {label}
                  </Link>
                );
              })}
            </nav>
          </div>

          <div className="hidden sm:block absolute left-1/2 -translate-x-1/2" style={{ width: 'min(400px, calc(100% - 500px))' }}>
            <div className="relative w-full">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
              <input
                type="text"
                placeholder="Search markets, agents, profiles..."
                className={cn(
                  'w-full h-9 pl-9 pr-4  text-sm',
                  'bg-bg-secondary border border-border',
                  'text-text-primary placeholder:text-text-muted',
                  'focus:outline-none focus:border-border-hover focus:ring-1 focus:ring-accent/20',
                  'transition-colors'
                )}
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            <ThemeToggle />
            <ConnectWalletButton />
          </div>
        </div>
      </div>
    </header>
  );
}
