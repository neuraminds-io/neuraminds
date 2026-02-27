'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Search } from 'lucide-react';
import { useBaseWallet } from '@/hooks/useBaseWallet';
import { BrandLogo } from '@/components/layout/BrandLogo';
import { ThemeToggle } from '@/components/ui/ThemeToggle';
import { cn } from '@/lib/utils';

const navLinks = [
  { href: '/markets', label: 'Markets' },
  { href: '/agents', label: 'Agents' },
  { href: '/portfolio', label: 'Portfolio' },
  { href: '/api', label: 'API' },
];

function ConnectWalletButton() {
  const baseWallet = useBaseWallet();

  const handleClick = () => {
    if (baseWallet.isConnected) {
      baseWallet.disconnect();
      return;
    }

    baseWallet.connect().catch((error) => {
      console.error('Base wallet connect failed:', error);
    });
  };

  const truncateAddress = (address: string) => {
    return `${address.slice(0, 4)}...${address.slice(-4)}`;
  };

  return (
    <button
      onClick={handleClick}
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
        : 'Connect Wallet'}
    </button>
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
